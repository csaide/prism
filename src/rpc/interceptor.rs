// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::service::Interceptor;
use tonic::{Request, Status};

/// The LoggerExt handles injecting a request specific logger into the gRPC execution
/// chain.
pub struct LoggerExt {
    /// The logger to use throughout this requests life cycle.
    pub logger: slog::Logger,
}

/// The interceptor wrapper to have all gRPC requests pass through.
#[derive(Debug, Clone)]
pub struct RaftInterceptor {
    logger: slog::Logger,
}

impl RaftInterceptor {
    /// Create a new RaftInterceptor based on the supplied arguments.
    pub fn new(logger: &slog::Logger) -> RaftInterceptor {
        RaftInterceptor {
            logger: logger.clone(),
        }
    }
}

impl Interceptor for RaftInterceptor {
    fn call(&mut self, mut req: Request<()>) -> Result<Request<()>, Status> {
        let req_id = if let Some(req_id) = req.metadata().get("x-request-id") {
            req_id.to_str().unwrap().to_string()
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        req.extensions_mut().insert(LoggerExt {
            logger: self.logger.new(o!("reqID" => req_id)),
        });

        Ok(req)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use tonic::metadata::MetadataValue;

    use super::*;

    #[test]
    fn test_interceptor() {
        let logger = slog::Logger::root(slog::Discard {}, o!());
        let mut interceptor = RaftInterceptor::new(&logger);

        let req = Request::new(());
        let res = interceptor.call(req);
        assert!(res.is_ok());

        let res = res.unwrap();
        let ext = res.extensions().get::<LoggerExt>();
        assert!(ext.is_some());

        let mut req = Request::new(());
        req.metadata_mut()
            .insert("x-request-id", MetadataValue::from_static("hello"));

        let res = interceptor.call(req);
        assert!(res.is_ok());

        let res = res.unwrap();
        let ext = res.extensions().get::<LoggerExt>();
        assert!(ext.is_some());
    }
}
