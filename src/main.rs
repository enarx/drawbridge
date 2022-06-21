// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::{read_to_string, File};
use std::io::{self, BufRead, BufReader};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use drawbridge_app::url::Url;
use drawbridge_app::{App, OidcConfig, TlsConfig};

use anyhow::Context as _;
use async_std::net::TcpListener;
use clap::Parser;
use futures::StreamExt;
use log::{debug, error};
use toml::Value;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Address to bind to.
    ///
    /// If no value is specified for this argument either on
    /// the command line or in a configuration file,
    /// the value will default to the unspecified IPv4 address
    /// 0.0.0.0 and port 8080.
    #[clap(long)]
    addr: Option<SocketAddr>,

    /// Path to the Drawbridge store.
    #[clap(long)]
    store: Option<PathBuf>,

    /// Path to PEM-encoded server certificate.
    #[clap(long)]
    cert: Option<PathBuf>,

    /// Path to PEM-encoded server certificate key.
    #[clap(long)]
    key: Option<PathBuf>,

    /// Path to PEM-encoded trusted CA certificate.
    ///
    /// Clients that present a valid certificate signed by this CA
    /// are granted read-only access to all repositories in the store.
    #[clap(long)]
    ca: Option<PathBuf>,

    /// OpenID Connect provider label.
    #[clap(long)]
    oidc_label: Option<String>,

    /// OpenID Connect issuer URL.
    #[clap(long)]
    oidc_issuer: Option<Url>,

    /// OpenID Connect client ID.
    #[clap(long)]
    oidc_client: Option<String>,

    /// OpenID Connect secret.
    #[clap(long)]
    oidc_secret: Option<String>,

    /// Path to a TOML configuration file.
    ///
    /// As an alternative to passing options to Drawbridge as
    /// command-line arguments, options can be read from a file.
    /// Options passed on the command-line will take precedence
    /// over options defined in the configuration file.
    #[clap(long)]
    config: Option<PathBuf>,
}

fn open_buffered(p: impl AsRef<Path>) -> io::Result<impl BufRead> {
    File::open(p).map(BufReader::new)
}

fn optional_arg<T>(arg: Option<T>, config: &Option<Value>, name: &str) -> anyhow::Result<Option<T>>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    // Unwrap the argument if present
    if let Some(a) = arg {
        return Ok(Some(a));
    }

    // Otherwise, look for a config file
    if let Some(vals) = config {
        // Look for the arg in the config file
        if let Some(val) = vals.get(name) {
            // Return the arg unless an error occurs
            return Ok(Some(
                val.as_str()
                    .with_context(|| format!("Failed to read field from config file: {name}"))?
                    .parse()
                    .with_context(|| format!("Failed to parse field from config file: {name}"))?,
            ));
        }
    }

    // Argument not found, but no error occurred
    Ok(None)
}

fn required_arg<T>(arg: Option<T>, config: &Option<Value>, name: &str) -> anyhow::Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    optional_arg(arg, config, name)?.with_context(|| format!("Missing required argument: {name}"))
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    let config = match args.config {
        Some(path) => Some(
            read_to_string(path)
                .context("Failed to read config file")?
                .parse::<Value>()
                .context("Failed to parse config file")?,
        ),
        None => None,
    };

    let addr: SocketAddr = optional_arg(args.addr, &config, "addr")?
        .unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080));
    let store = required_arg(args.store, &config, "store")?;
    let cert = required_arg(args.cert, &config, "cert")?;
    let key = required_arg(args.key, &config, "key")?;
    let ca = required_arg(args.ca, &config, "ca")?;
    let oidc_label = required_arg(args.oidc_label, &config, "oidc_label")?;
    let oidc_issuer = required_arg(args.oidc_issuer, &config, "oidc_issuer")?;
    let oidc_client = required_arg(args.oidc_client, &config, "oidc_client")?;
    let oidc_secret = optional_arg(args.oidc_secret, &config, "oidc_secret")?;

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
