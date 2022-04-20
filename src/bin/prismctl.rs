// (c) Copyright 2022 Christian Saide
// SPDX-License-Identifier: GPL-3.0-or-later

#[tokio::main]
async fn main() {
    let code = libprism::prismctl::run().await;
    std::process::exit(code)
}
