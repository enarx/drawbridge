// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use drawbridge_repo as repo;

use axum::Server;

#[tokio::main]
async fn main() {
    let app = repo::app();

    Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
