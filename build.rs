// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

use std::env;
use std::path::PathBuf;

const PROTO_DIR: &str = "./proto/";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let files = std::fs::read_dir(PROTO_DIR).expect("failed to list proto files.");
    for file in files {
        let file = file.expect("failed to read file path");
        let file = file.path();
        let file_name = String::from(
            file.file_name()
                .expect("failed to determine filename of current path")
                .to_str()
                .unwrap(),
        );

        let descriptor_name = file_name.replace(".proto", "_descriptor.bin");

        tonic_build::configure()
            .build_client(true)
            .build_server(true)
            .type_attribute(
                ".raft",
                "#[derive(serde_derive::Serialize, serde_derive::Deserialize, Eq)]",
            )
            .type_attribute(".frontend", "#[derive(Eq)]")
            .type_attribute(".cluster", "#[derive(Eq)]")
            .field_attribute(".raft.Command.data", "#[serde(with = \"serde_bytes\")]")
            .field_attribute(".raft.Snapshot.data", "#[serde(with = \"serde_bytes\")]")
            .file_descriptor_set_path(out_dir.join(descriptor_name))
            .compile(&[file], &[PROTO_DIR])?;
    }

    Ok(())
}
