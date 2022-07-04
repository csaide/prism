// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::raft::{
    MutateStateRequest, MutateStateResponse, ReadStateRequest, ReadStateResponse,
    RegisterClientRequest, RegisterClientResponse,
};

tonic::include_proto!("frontend");

impl From<MutateRequest> for MutateStateRequest {
    fn from(input: MutateRequest) -> MutateStateRequest {
        MutateStateRequest {
            client_id: input.client_id,
            command: input.command,
            sequence_num: input.sequence_num,
        }
    }
}

impl From<MutateStateRequest> for MutateRequest {
    fn from(input: MutateStateRequest) -> MutateRequest {
        MutateRequest {
            client_id: input.client_id,
            command: input.command,
            sequence_num: input.sequence_num,
        }
    }
}

impl From<MutateResponse> for MutateStateResponse {
    fn from(input: MutateResponse) -> MutateStateResponse {
        MutateStateResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        }
    }
}

impl From<MutateStateResponse> for MutateResponse {
    fn from(input: MutateStateResponse) -> MutateResponse {
        MutateResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        }
    }
}

impl From<ReadRequest> for ReadStateRequest {
    fn from(input: ReadRequest) -> ReadStateRequest {
        ReadStateRequest { query: input.query }
    }
}

impl From<ReadStateRequest> for ReadRequest {
    fn from(input: ReadStateRequest) -> ReadRequest {
        ReadRequest { query: input.query }
    }
}

impl From<ReadResponse> for ReadStateResponse {
    fn from(input: ReadResponse) -> ReadStateResponse {
        ReadStateResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        }
    }
}

impl From<ReadStateResponse> for ReadResponse {
    fn from(input: ReadStateResponse) -> ReadResponse {
        ReadResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        }
    }
}

impl From<RegisterRequest> for RegisterClientRequest {
    fn from(_: RegisterRequest) -> RegisterClientRequest {
        RegisterClientRequest {}
    }
}

impl From<RegisterClientRequest> for RegisterRequest {
    fn from(_: RegisterClientRequest) -> RegisterRequest {
        RegisterRequest {}
    }
}

impl From<RegisterResponse> for RegisterClientResponse {
    fn from(input: RegisterResponse) -> RegisterClientResponse {
        RegisterClientResponse {
            client_id: input.client_id,
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

impl From<RegisterClientResponse> for RegisterResponse {
    fn from(input: RegisterClientResponse) -> RegisterResponse {
        RegisterResponse {
            client_id: input.client_id,
            leader_hint: input.leader_hint,
            status: input.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutate_request() {
        let client_id = 1u128;
        let cmd = vec![0x00, 0x01, 0x02];
        let seq = 1;
        let raft = MutateStateRequest {
            client_id: client_id.to_be_bytes().to_vec(),
            command: cmd.clone(),
            sequence_num: seq,
        };

        let actual = MutateRequest::from(raft);
        assert_eq!(actual.client_id, client_id.to_be_bytes().to_vec());
        assert_eq!(actual.command, cmd);
        assert_eq!(actual.sequence_num, seq);

        let actual_raft = MutateStateRequest::from(actual);
        assert_eq!(actual_raft.client_id, client_id.to_be_bytes().to_vec());
        assert_eq!(actual_raft.command, cmd);
        assert_eq!(actual_raft.sequence_num, seq);
    }

    #[test]
    fn test_mutate_response() {
        let leader_hint = String::from("leader");
        let response = vec![0x00, 0x01, 0x02, 0x03];
        let status = String::from("OK");

        let raft = MutateStateResponse {
            leader_hint: leader_hint.clone(),
            response: response.clone(),
            status: status.clone(),
        };

        let actual = MutateResponse::from(raft);
        assert_eq!(actual.leader_hint, leader_hint);
        assert_eq!(actual.response, response);
        assert_eq!(actual.status, status);

        let actual_raft = MutateStateResponse::from(actual);
        assert_eq!(actual_raft.leader_hint, leader_hint);
        assert_eq!(actual_raft.response, response);
        assert_eq!(actual_raft.status, status);
    }

    #[test]
    fn test_read_request() {
        let query = vec![0x01, 0x02];
        let raft = ReadStateRequest {
            query: query.clone(),
        };

        let actual = ReadRequest::from(raft);
        assert_eq!(actual.query, query);

        let actual_raft = ReadStateRequest::from(actual);
        assert_eq!(actual_raft.query, query);
    }

    #[test]
    fn test_read_response() {
        let leader_hint = String::from("leader");
        let response = vec![0x00, 0x01, 0x02, 0x03];
        let status = String::from("OK");

        let raft = ReadStateResponse {
            leader_hint: leader_hint.clone(),
            response: response.clone(),
            status: status.clone(),
        };

        let actual = ReadResponse::from(raft);
        assert_eq!(actual.leader_hint, leader_hint);
        assert_eq!(actual.response, response);
        assert_eq!(actual.status, status);

        let actual_raft = ReadStateResponse::from(actual);
        assert_eq!(actual_raft.leader_hint, leader_hint);
        assert_eq!(actual_raft.response, response);
        assert_eq!(actual_raft.status, status);
    }

    #[test]
    fn test_register_request() {
        let raft = RegisterClientRequest {};

        let actual = RegisterRequest::from(raft);
        let _actual_raft = RegisterClientRequest::from(actual);
    }

    #[test]
    fn test_register_response() {
        let client_id = 1u128.to_be_bytes().to_vec();
        let leader_hint = String::from("leader");
        let status = String::from("OK");

        let raft = RegisterClientResponse {
            client_id: client_id.clone(),
            leader_hint: leader_hint.clone(),
            status: status.clone(),
        };

        let actual = RegisterResponse::from(raft);
        assert_eq!(actual.client_id, client_id);
        assert_eq!(actual.leader_hint, leader_hint);
        assert_eq!(actual.status, status);

        let actual_raft = RegisterClientResponse::from(actual);
        assert_eq!(actual_raft.client_id, client_id);
        assert_eq!(actual_raft.leader_hint, leader_hint);
        assert_eq!(actual_raft.status, status);
    }
}
