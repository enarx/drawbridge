// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![forbid(unsafe_code)]
#![deny(
    clippy::all,
    absolute_paths_not_starting_with_crate,
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    noop_method_call,
    rust_2018_compatibility,
    rust_2018_idioms,
    rust_2021_compatibility,
    single_use_lifetimes,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub,
    unstable_features,
    unused,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use drawbridge_server::url::Url;
use drawbridge_server::{App, OidcConfig, TlsConfig};

use anyhow::Context as _;
use async_std::net::TcpListener;
use clap::Parser;
use confargs::{args, prefix_char_filter, Toml};
use futures::StreamExt;
use tracing::{debug, error};

/// Server for hosting WebAssembly modules for use in Enarx keeps.
///
/// Any command-line options listed here may be specified by one or
/// more configuration files, which can be used by passing the
/// name of the file on the command-line with the syntax `@config.toml`.
/// The configuration file must contain valid TOML table mapping argument
/// names to their values.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Address to bind to.
    #[arg(long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))]
    addr: SocketAddr,

    /// Path to the Drawbridge store.
    #[arg(long)]
    store: PathBuf,

    /// Path to PEM-encoded server certificate.
    #[arg(long)]
    cert: PathBuf,

    /// Path to PEM-encoded server certificate key.
    #[arg(long)]
    key: PathBuf,

    /// Path to PEM-encoded trusted CA certificate.
    ///
    /// Clients that present a valid certificate signed by this CA
    /// are granted read-only access to all repositories in the store.
    #[arg(long)]
    ca: PathBuf,

    /// OpenID Connect issuer URL.
    #[arg(long)]
    oidc_issuer: Url,

    /// OpenID Connect audience.
    #[arg(long)]
    oidc_audience: String,
}

fn open_buffered(p: impl AsRef<Path>) -> io::Result<impl BufRead> {
    File::open(p).map(BufReader::new)
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG_JSON").is_ok() {
        tracing_subscriber::fmt::fmt()
            .json()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    } else {
        tracing_subscriber::fmt::init();
    }

    let Args {
        addr,
        store,
        cert,
        key,
        ca,
        oidc_audience,
        oidc_issuer,
    } = args::<Toml>(prefix_char_filter::<'@'>)
        .context("Failed to parse config")
        .map(Args::parse_from)?;

    let cert = open_buffered(cert).context("Failed to open server certificate file")?;
    let key = open_buffered(key).context("Failed to open server key file")?;
    let ca = open_buffered(ca).context("Failed to open CA certificate file")?;
    let tls = TlsConfig::read(cert, key, ca).context("Failed to construct server TLS config")?;

    let app = App::new(
        store,
        tls,
        OidcConfig {
            audience: oidc_audience,
            issuer: oidc_issuer,
        },
    )
    .await
    .context("Failed to build app")?;
    TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?
        .incoming()
        .for_each_concurrent(None, |stream| async {
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
