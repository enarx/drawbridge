// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use drawbridge_app::{self as app, TLSConfig};

use axum_server::bind_rustls;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    store: PathBuf,

    #[clap(long)]
    cert: PathBuf,

    #[clap(long)]
    key: PathBuf,

    #[clap(long)]
    ca: PathBuf,
}

fn open_buffered(p: impl AsRef<Path>) -> io::Result<impl BufRead> {
    File::open(p).map(BufReader::new)
}

#[tokio::main]
async fn main() {
    let Args {
        store,
        cert,
        key,
        ca,
    } = Args::parse();

    // TODO: Proper error handling

    let cert = open_buffered(cert).expect("Failed to open server certificate file");
    let key = open_buffered(key).expect("Failed to open server key file");
    let ca = open_buffered(ca).expect("Failed to open CA certificate file");

    let tls = TLSConfig::read(cert, key, ca)
        .map(Into::into)
        .map(Arc::new)
        .map(RustlsConfig::from_config)
        .expect("Failed to construct server TLS config");

    bind_rustls(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080),
        tls,
    )
    .serve(app::Builder::new(store).build().unwrap())
    .await
    .unwrap();
}
