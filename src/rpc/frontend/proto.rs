// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::raft::{
    MutateStateRequest, MutateStateResponse, ReadStateRequest, ReadStateResponse,
    RegisterClientRequest, RegisterClientResponse, Result,
};

tonic::include_proto!("frontend");

impl MutateRequest {
    pub fn into_raft(self) -> Result<MutateStateRequest> {
        Ok(MutateStateRequest {
            client_id: self.client_id,
            command: self.command,
            sequence_num: self.sequence_num,
        })
    }

    pub fn from_raft(input: MutateStateRequest) -> Result<MutateRequest> {
        Ok(MutateRequest {
            client_id: input.client_id,
            command: input.command,
            sequence_num: input.sequence_num,
        })
    }
}

impl MutateResponse {
    pub fn into_raft(self) -> Result<MutateStateResponse> {
        Ok(MutateStateResponse {
            leader_hint: self.leader_hint,
            response: self.response,
            status: self.status,
        })
    }

    pub fn from_raft(input: MutateStateResponse) -> Result<MutateResponse> {
        Ok(MutateResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        })
    }
}

impl ReadRequest {
    pub fn into_raft(self) -> Result<ReadStateRequest> {
        Ok(ReadStateRequest { query: self.query })
    }

    pub fn from_raft(input: ReadStateRequest) -> Result<ReadRequest> {
        Ok(ReadRequest { query: input.query })
    }
}

impl ReadResponse {
    pub fn into_raft(self) -> Result<ReadStateResponse> {
        Ok(ReadStateResponse {
            leader_hint: self.leader_hint,
            response: self.response,
            status: self.status,
        })
    }

    pub fn from_raft(input: ReadStateResponse) -> Result<ReadResponse> {
        Ok(ReadResponse {
            leader_hint: input.leader_hint,
            response: input.response,
            status: input.status,
        })
    }
}

impl RegisterRequest {
    pub fn into_raft(self) -> Result<RegisterClientRequest> {
        Ok(RegisterClientRequest {})
    }

    pub fn from_raft(_: RegisterClientRequest) -> Result<RegisterRequest> {
        Ok(RegisterRequest {})
    }
}

impl RegisterResponse {
    pub fn into_raft(self) -> Result<RegisterClientResponse> {
        Ok(RegisterClientResponse {
            client_id: self.client_id,
            leader_hint: self.leader_hint,
            status: self.status,
        })
    }

    pub fn from_raft(input: RegisterClientResponse) -> Result<RegisterResponse> {
        Ok(RegisterResponse {
            client_id: input.client_id,
            leader_hint: input.leader_hint,
            status: input.status,
        })
    }
}
