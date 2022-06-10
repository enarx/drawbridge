// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

//! This example sets up a local Drawbridge server on port 12345.
//! Run this example with `cargo run --example server`.

use std::net::Ipv4Addr;

use drawbridge_app::Builder;
use hyper::Server;
use tempfile::tempdir;

#[tokio::main]
async fn main() {
    let store = tempdir().unwrap();

    let service = Builder::new(store.path()).build().unwrap();

    let socket = (Ipv4Addr::LOCALHOST, 12345).into();

    let server = Server::bind(&socket).serve(service);

    println!(
        "Listening on {} and writing temporary files to {}",
        socket,
        store.path().display()
    );

    server.await.unwrap();
}
