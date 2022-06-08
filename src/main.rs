// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::env::args;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use drawbridge_app as app;

use hyper::Server;

#[tokio::main]
async fn main() {
    Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))
        .serve(
            app::Builder::new(args().nth(1).expect("Store path must be specified"))
                .build()
                .unwrap(),
        )
        .await
        .unwrap();
}
