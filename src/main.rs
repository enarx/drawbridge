// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::env::{self, args};
use std::fs::read;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use drawbridge_app as app;

#[tokio::main]
async fn main() {
    let certificates =
        read(env::var("CERTS").expect("CERTS must be the path to the servers pem certificates"))
            .expect("PK must be the path to the servers private key pem");
    let private_key =
        read(env::var("PK").expect("PK must be the path to the servers private key pem"))
            .expect("failed to read private key file");
    let ca = env::var("CA")
        .map(|path| read(path).expect("failed to read ca file"))
        .ok();

    app::Builder::new(args().nth(1).expect("Store path must be specified"))
        .serve(
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080),
            &certificates,
            &private_key,
            ca,
            Arc::new(AtomicBool::new(false)),
        )
        .await
        .unwrap()
}
