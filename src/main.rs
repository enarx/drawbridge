// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use drawbridge_app::{App, TlsConfig};

use anyhow::Context as _;
use async_std::net::TcpListener;
use clap::Parser;
use futures::StreamExt;
use log::{debug, error};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address to bind to.
    #[clap(long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))]
    addr: SocketAddr,

    /// Path to the Drawbridge store.
    #[clap(long)]
    store: PathBuf,

    /// Path to PEM-encoded server certificate.
    #[clap(long)]
    cert: PathBuf,

    /// Path to PEM-encoded server certificate key.
    #[clap(long)]
    key: PathBuf,

    /// Path to PEM-encoded trusted CA certificate.
    ///
    /// Clients that present a valid certificate signed by this CA
    /// are granted read-only access to all repositories in the store.
    #[clap(long)]
    ca: PathBuf,
}

fn open_buffered(p: impl AsRef<Path>) -> io::Result<impl BufRead> {
    File::open(p).map(BufReader::new)
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let Args {
        store,
        cert,
        key,
        ca,
        addr,
    } = Args::parse();

    let cert = open_buffered(cert).context("Failed to open server certificate file")?;
    let key = open_buffered(key).context("Failed to open server key file")?;
    let ca = open_buffered(ca).context("Failed to open CA certificate file")?;
    let tls = TlsConfig::read(cert, key, ca).context("Failed to construct server TLS config")?;

    let app = App::builder(store, tls)
        .build()
        .await
        .context("Failed to build app")?;
    TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?
        .incoming()
        .for_each_concurrent(Some(1), |stream| async {
            if let Err(e) = async {
                let stream = stream.context("failed to initialize connection")?;
                debug!(
                    target: "main",
                    "received TCP connection from {}",
                    stream
                        .peer_addr()
                        .map(|peer| peer.to_string())
                        .unwrap_or_else(|_| "unknown address".into())
                );
                app.handle(stream).await
            }
            .await
            {
                error!(target: "main", "failed to handle request: {e}");
            }
        })
        .await;
    Ok(())
}
