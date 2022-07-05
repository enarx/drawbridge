// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::{read, File};
use std::io::{self, BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use drawbridge_server::url::Url;
use drawbridge_server::{App, OidcConfig, TlsConfig};

use anyhow::{bail, Context as _};
use async_std::net::TcpListener;
use clap::Parser;
use futures::StreamExt;
use log::{debug, error};

/// Server for hosting WebAssembly modules for use in Enarx keeps.
///
/// Any command-line options listed here may be specified by one or
/// more configuration files, which can be used by passing the
/// name of the file on the command-line with the syntax `@config.toml`.
/// The configuration file must contain valid TOML table mapping argument
/// names to their values.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
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

    /// OpenID Connect provider label.
    #[clap(long)]
    oidc_label: String,

    /// OpenID Connect issuer URL.
    #[clap(long)]
    oidc_issuer: Url,

    /// OpenID Connect client ID.
    #[clap(long)]
    oidc_client: String,

    /// OpenID Connect secret.
    #[clap(long)]
    oidc_secret: Option<String>,
}

fn open_buffered(p: impl AsRef<Path>) -> io::Result<impl BufRead> {
    File::open(p).map(BufReader::new)
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let Args {
        addr,
        store,
        cert,
        key,
        ca,
        oidc_label,
        oidc_issuer,
        oidc_client,
        oidc_secret,
    } = std::env::args()
        .try_fold(Vec::new(), |mut args, arg| {
            if let Some(path) = arg.strip_prefix('@') {
                let conf = read(path).context(format!("failed to read config file at `{path}`"))?;
                match toml::from_slice(&conf)
                    .context(format!("failed to parse config file at `{path}` as TOML"))?
                {
                    toml::Value::Table(kv) => kv.into_iter().try_for_each(|(k, v)| {
                        match v {
                            toml::Value::String(v) => args.push(format!("--{k}={v}")),
                            toml::Value::Integer(v) => args.push(format!("--{k}={v}")),
                            toml::Value::Float(v) => args.push(format!("--{k}={v}")),
                            toml::Value::Boolean(v) => {
                                if v {
                                    args.push(format!("--{k}"))
                                }
                            }
                            _ => bail!(
                                "unsupported value type for field `{k}` in config file at `{path}`"
                            ),
                        }
                        Ok(())
                    })?,
                    _ => bail!("invalid config file format in file at `{path}`"),
                }
            } else {
                args.push(arg);
            }
            Ok(args)
        })
        .map(Args::parse_from)
        .context("Failed to parse arguments")?;

    let cert = open_buffered(cert).context("Failed to open server certificate file")?;
    let key = open_buffered(key).context("Failed to open server key file")?;
    let ca = open_buffered(ca).context("Failed to open CA certificate file")?;
    let tls = TlsConfig::read(cert, key, ca).context("Failed to construct server TLS config")?;

    let app = App::new(
        store,
        tls,
        OidcConfig {
            label: oidc_label,
            issuer: oidc_issuer,
            client_id: oidc_client,
            client_secret: oidc_secret,
        },
    )
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
