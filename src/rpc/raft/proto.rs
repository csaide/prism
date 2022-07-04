// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use tonic::transport::Channel;

use crate::raft::{
    self, AppendEntriesRequest, AppendEntriesResponse, Client, Error, RequestVoteRequest,
    RequestVoteResponse, Result,
};

use super::RaftClient;

tonic::include_proto!("raft");

#[tonic::async_trait]
impl Client for RaftClient<Channel> {
    async fn connect(addr: String) -> Result<RaftClient<Channel>> {
        RaftClient::connect(addr).await.map_err(Error::from)
    }
    async fn vote(&mut self, req: RequestVoteRequest) -> Result<RequestVoteResponse> {
        self.request_vote(VoteRequest::from(req))
            .await
            .map(|resp| resp.into_inner())
            .map(RequestVoteResponse::from)
            .map_err(Error::from)
    }
    async fn append(&mut self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        self.append_entries(AppendRequest::from(req))
            .await
            .map(|resp| resp.into_inner())
            .map(AppendEntriesResponse::from)
            .map_err(Error::from)
    }
}

impl From<entry::Payload> for raft::Entry {
    fn from(input: entry::Payload) -> Self {
        use entry::Payload::*;
        match input {
            Command(cmd) => {
                let term = cmd
                    .term
                    .as_slice()
                    .try_into()
                    .map(u128::from_be_bytes)
                    .unwrap_or(0);
                raft::Entry::Command(raft::Command {
                    term,
                    data: cmd.data,
                })
            }
            Config(cfg) => {
                let term = cfg
                    .term
                    .as_slice()
                    .try_into()
                    .map(u128::from_be_bytes)
                    .unwrap_or(0);
                raft::Entry::ClusterConfig(raft::ClusterConfig {
                    term,
                    replicas: cfg.replicas,
                    voters: cfg.voters,
                })
            }
            Registration(reg) => {
                let term = reg
                    .term
                    .as_slice()
                    .try_into()
                    .map(u128::from_be_bytes)
                    .unwrap_or(0);
                raft::Entry::Registration(raft::Registration { term })
            }
        }
    }
}

impl From<raft::Entry> for entry::Payload {
    fn from(input: raft::Entry) -> Self {
        match input {
            raft::Entry::Command(cmd) => entry::Payload::Command(Command {
                data: cmd.data,
                term: cmd.term.to_be_bytes().to_vec(),
            }),
            raft::Entry::ClusterConfig(cfg) => entry::Payload::Config(ClusterConfig {
                term: cfg.term.to_be_bytes().to_vec(),
                replicas: cfg.replicas,
                voters: cfg.voters,
            }),
            raft::Entry::Registration(reg) => entry::Payload::Registration(Registration {
                term: reg.term.to_be_bytes().to_vec(),
            }),
        }
    }
}

impl From<Entry> for raft::Entry {
    fn from(input: Entry) -> Self {
        if let Some(payload) = input.payload {
            raft::Entry::from(payload)
        } else {
            panic!("invalid input Entry.payload must never be None");
        }
    }
}

impl From<raft::Entry> for Entry {
    fn from(input: raft::Entry) -> Self {
        Entry {
            payload: Some(entry::Payload::from(input)),
        }
    }
}

impl From<AppendRequest> for AppendEntriesRequest {
    fn from(mut input: AppendRequest) -> Self {
        let term = input
            .term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let leader_commit_idx = input
            .leader_commit_idx
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let prev_log_idx = input
            .prev_log_idx
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let prev_log_term = input
            .prev_log_term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let entries = input.entries.drain(..).map(raft::Entry::from).collect();

        AppendEntriesRequest {
            term,
            leader_commit_idx,
            prev_log_idx,
            prev_log_term,
            leader_id: input.leader_id,
            entries,
        }
    }
}

impl From<AppendEntriesRequest> for AppendRequest {
    fn from(mut input: AppendEntriesRequest) -> Self {
        let entries = input.entries.drain(..).map(Entry::from).collect();

        AppendRequest {
            leader_id: input.leader_id,
            term: input.term.to_be_bytes().to_vec(),
            leader_commit_idx: input.leader_commit_idx.to_be_bytes().to_vec(),
            prev_log_idx: input.prev_log_idx.to_be_bytes().to_vec(),
            prev_log_term: input.prev_log_term.to_be_bytes().to_vec(),
            entries,
        }
    }
}

impl From<AppendResponse> for AppendEntriesResponse {
    fn from(input: AppendResponse) -> Self {
        let term = input
            .term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        AppendEntriesResponse {
            term,
            success: input.success,
        }
    }
}

impl From<AppendEntriesResponse> for AppendResponse {
    fn from(input: AppendEntriesResponse) -> Self {
        AppendResponse {
            term: input.term.to_be_bytes().to_vec(),
            success: input.success,
        }
    }
}

impl From<VoteRequest> for RequestVoteRequest {
    fn from(input: VoteRequest) -> Self {
        let term = input
            .term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let last_log_idx = input
            .last_log_idx
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        let last_log_term = input
            .last_log_term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);

        RequestVoteRequest {
            candidate_id: input.candidate_id,
            last_log_idx,
            last_log_term,
            term,
        }
    }
}

impl From<RequestVoteRequest> for VoteRequest {
    fn from(input: RequestVoteRequest) -> Self {
        VoteRequest {
            candidate_id: input.candidate_id,
            last_log_idx: input.last_log_idx.to_be_bytes().to_vec(),
            last_log_term: input.last_log_term.to_be_bytes().to_vec(),
            term: input.term.to_be_bytes().to_vec(),
        }
    }
}

impl From<VoteResponse> for RequestVoteResponse {
    fn from(input: VoteResponse) -> Self {
        let term = input
            .term
            .as_slice()
            .try_into()
            .map(u128::from_be_bytes)
            .unwrap_or(0);
        RequestVoteResponse {
            term,
            vote_granted: input.vote_granted,
        }
    }
}

impl From<RequestVoteResponse> for VoteResponse {
    fn from(input: RequestVoteResponse) -> Self {
        VoteResponse {
            term: input.term.to_be_bytes().to_vec(),
            vote_granted: input.vote_granted,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use rstest::rstest;

    mod entry_and_payload {
        use super::*;
        #[rstest]
        #[case::command(
            entry::Payload::Command(Command{
                term: 1u128.to_be_bytes().to_vec(),
                data: vec![0x00],
            }),
            raft::Entry::Command(raft::Command{
                term: 1,
                data: vec![0x00]
            }),
        )]
        #[case::command_invalid_term(
            entry::Payload::Command(Command{
                term: vec![0x00, 0x00],
                data: vec![0x00],
            }),
            raft::Entry::Command(raft::Command{
                term: 0,
                data: vec![0x00]
            }),
        )]
        #[case::config(
            entry::Payload::Config(ClusterConfig{
                term: 1u128.to_be_bytes().to_vec(),
                replicas: vec![String::from("hello"), String::from("noop")],
                voters: vec![String::from("hello"), String::from("noop")],
            }),
            raft::Entry::ClusterConfig(raft::ClusterConfig{
                term: 1,
                replicas: vec![String::from("hello"), String::from("noop")],
                voters: vec![String::from("hello"), String::from("noop")],
            }),
        )]
        #[case::config_invalid_term(
            entry::Payload::Config(ClusterConfig{
                term: vec![0x00,0x00],
                replicas: vec![String::from("hello"), String::from("noop")],
                voters: vec![String::from("hello"), String::from("noop")],
            }),
            raft::Entry::ClusterConfig(raft::ClusterConfig{
                term: 0,
                replicas: vec![String::from("hello"), String::from("noop")],
                voters: vec![String::from("hello"), String::from("noop")],
            }),
        )]
        #[case::registration(
            entry::Payload::Registration(Registration {
                term: 1u128.to_be_bytes().to_vec(),
            }),
            raft::Entry::Registration(raft::Registration {
                term: 1,
            })
        )]
        #[case::registration_invalid_term(
            entry::Payload::Registration(Registration {
                term: vec![0x00, 0x00],
            }),
            raft::Entry::Registration(raft::Registration {
                term: 0,
            })
        )]
        fn test_raft_entry_from_entry_payload(
            #[case] payload: entry::Payload,
            #[case] expected: raft::Entry,
        ) {
            let actual = raft::Entry::from(payload);
            assert_eq!(actual, expected)
        }

        #[rstest]
        #[case::command(
            raft::Entry::Command(raft::Command {
                data: vec![0x00],
                term: 1,
            }),
            entry::Payload::Command(Command {
                data: vec!{0x00},
                term: 1u128.to_be_bytes().to_vec(),
            })
        )]
        #[case::conig(
            raft::Entry::ClusterConfig(raft::ClusterConfig {
                replicas:  vec![String::from("repl1")],
                voters: vec![
                    String::from("vote1"),
                    String::from("vote2"),
                    String::from("vote3"),
                ],
                term: 1,
            }),
            entry::Payload::Config(ClusterConfig {
                replicas:  vec![String::from("repl1")],
                voters: vec![
                    String::from("vote1"),
                    String::from("vote2"),
                    String::from("vote3"),
                ],
                term: 1u128.to_be_bytes().to_vec(),
            })
        )]
        #[case::registration(
            raft::Entry::Registration(raft::Registration{
                term: 1,
            }),
            entry::Payload::Registration(Registration {
                term: 1u128.to_be_bytes().to_vec(),
            })
        )]
        fn test_entry_payload_from_raft_entry(
            #[case] raft: raft::Entry,
            #[case] expected: entry::Payload,
        ) {
            let actual = entry::Payload::from(raft);
            assert_eq!(actual, expected)
        }

        #[rstest]
        #[case::some(
            Entry {
                payload: Some(entry::Payload::Command(Command{
                    term: 1u128.to_be_bytes().to_vec(),
                    data: vec![0x00],
                })),
            },
            Some(raft::Entry::Command(raft::Command{
                term: 1,
                data: vec![0x00]
            }))
        )]
        #[should_panic]
        #[case::none(
            Entry {
                payload: None,
            },
            None
        )]
        fn test_raft_entry_from_entry(#[case] entry: Entry, #[case] expected: Option<raft::Entry>) {
            let actual = raft::Entry::from(entry);
            let expected = expected.unwrap();
            assert_eq!(actual, expected);
        }

        #[test]
        fn test_entry_from_raft_entry() {
            let expected = Entry {
                payload: Some(entry::Payload::Command(Command {
                    term: 1u128.to_be_bytes().to_vec(),
                    data: vec![0x00],
                })),
            };
            let raft = raft::Entry::Command(raft::Command {
                term: 1,
                data: vec![0x00],
            });
            let actual = Entry::from(raft);
            assert_eq!(expected, actual)
        }
    }

    mod append {
        use super::*;

        #[test]
        fn test_raft_append_to_append() {
            let append = AppendEntriesRequest {
                entries: Vec::default(),
                leader_commit_idx: 1,
                leader_id: String::from("leader"),
                prev_log_idx: 1,
                prev_log_term: 1,
                term: 1,
            };
            let expected = AppendRequest {
                term: 1u128.to_be_bytes().to_vec(),
                leader_id: String::from("leader"),
                leader_commit_idx: 1u128.to_be_bytes().to_vec(),
                prev_log_idx: 1u128.to_be_bytes().to_vec(),
                prev_log_term: 1u128.to_be_bytes().to_vec(),
                entries: Vec::default(),
            };
            let actual = AppendRequest::from(append);
            assert_eq!(actual, expected)
        }

        #[rstest]
        #[case::happy_path(
            AppendRequest {
                term: 1u128.to_be_bytes().to_vec(),
                leader_id: String::from("leader"),
                leader_commit_idx: 1u128.to_be_bytes().to_vec(),
                prev_log_idx: 1u128.to_be_bytes().to_vec(),
                prev_log_term: 1u128.to_be_bytes().to_vec(),
                entries: Vec::default(),
            },
            AppendEntriesRequest {
                entries: Vec::default(),
                leader_commit_idx: 1,
                leader_id: String::from("leader"),
                prev_log_idx: 1,
                prev_log_term: 1,
                term: 1,
            }
        )]
        #[case::happy_path(
            AppendRequest {
                entries: Vec::default(),
                leader_id: String::from("leader"),
                leader_commit_idx: vec![0x00],
                prev_log_idx: vec![0x00],
                prev_log_term: vec![0x00],
                term: vec![0x00],
            },
            AppendEntriesRequest {
                entries: Vec::default(),
                leader_id: String::from("leader"),
                leader_commit_idx: 0,
                prev_log_idx: 0,
                prev_log_term: 0,
                term: 0,
            }
        )]
        fn test_append_to_raft_append(
            #[case] append: AppendRequest,
            #[case] expected: AppendEntriesRequest,
        ) {
            let actual = AppendEntriesRequest::from(append);
            assert_eq!(actual, expected)
        }

        #[test]
        fn test_raft_append_resp_to_append_resp() {
            let resp = AppendEntriesResponse {
                success: true,
                term: 1,
            };
            let expected = AppendResponse {
                success: true,
                term: 1u128.to_be_bytes().to_vec(),
            };
            let actual = AppendResponse::from(resp);
            assert_eq!(actual, expected);
        }

        #[rstest]
        #[case::happy_path(
            AppendResponse {
                success: true,
                term: 1u128.to_be_bytes().to_vec(),
            },
            AppendEntriesResponse {
                success: true,
                term: 1,
            }
        )]
        #[case::invalid(
            AppendResponse {
                success: true,
                term: vec![0x00],
            },
            AppendEntriesResponse {
                success: true,
                term: 0,
            }
        )]
        fn test_append_resp_to_raft_append_resp(
            #[case] append: AppendResponse,
            #[case] expected: AppendEntriesResponse,
        ) {
            let actual = AppendEntriesResponse::from(append);
            assert_eq!(actual, expected);
        }
    }

    mod vote {
        use super::*;

        #[test]
        fn test_raft_vote_to_vote() {
            let vote = RequestVoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 1,
                last_log_term: 1,
                term: 1,
            };
            let expected = VoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 1u128.to_be_bytes().to_vec(),
                last_log_term: 1u128.to_be_bytes().to_vec(),
                term: 1u128.to_be_bytes().to_vec(),
            };
            let actual = VoteRequest::from(vote);
            assert_eq!(actual, expected);
        }

        #[rstest]
        #[case::happy_path(
            VoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 1u128.to_be_bytes().to_vec(),
                last_log_term: 1u128.to_be_bytes().to_vec(),
                term: 1u128.to_be_bytes().to_vec(),
            },
            RequestVoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 1,
                last_log_term: 1,
                term: 1,
            }
        )]
        #[case::invalid(
            VoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: vec![0x00],
                last_log_term: vec![0x00],
                term: vec![0x00],
            },
            RequestVoteRequest {
                candidate_id: String::from("leader"),
                last_log_idx: 0,
                last_log_term: 0,
                term: 0,
            }
        )]
        fn test_vote_to_raft_vote(#[case] vote: VoteRequest, #[case] expected: RequestVoteRequest) {
            let actual = RequestVoteRequest::from(vote);
            assert_eq!(actual, expected)
        }

        #[test]
        fn test_raft_vote_resp_to_vote_resp() {
            let resp = RequestVoteResponse {
                term: 1,
                vote_granted: true,
            };
            let expected = VoteResponse {
                term: 1u128.to_be_bytes().to_vec(),
                vote_granted: true,
            };
            let actual = VoteResponse::from(resp);
            assert_eq!(actual, expected)
        }

        #[rstest]
        #[case::happy_path(
            VoteResponse {
                term: 1u128.to_be_bytes().to_vec(),
                vote_granted: true,
            },
            RequestVoteResponse {
                term: 1,
                vote_granted: true,
            }
        )]
        #[case::invalid(
            VoteResponse {
                term: vec![0x00],
                vote_granted: true,
            },
            RequestVoteResponse {
                term: 0,
                vote_granted: true,
            }
        )]
        fn test_vote_resp_to_raft_vote_resp(
            #[case] resp: VoteResponse,
            #[case] expected: RequestVoteResponse,
        ) {
            let actual = RequestVoteResponse::from(resp);
            assert_eq!(actual, expected)
        }
    }
}
