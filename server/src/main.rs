#[macro_use]
extern crate anyhow;

mod routes;
#[cfg(test)]
mod tests;
mod types;

use axum::{routing::get, AddExtensionLayer, Router, Server};
use clap::{AppSettings, Parser};
use http::Uri;
use routes::{auth_routes, protected};
use rsa::{pkcs8::FromPrivateKey, RsaPrivateKey};

use std::{env, net::SocketAddr, str::FromStr};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
/// The drawbridge server.
pub struct Args {
    #[clap(short, long, default_value = "0.0.0.0:3000")]
    /// The server ip/port to listen on.
    host: String,
}

fn app(host: &str) -> Router {
    // TODO: generate this key at runtime or pull the path from the command line args: https://github.com/profianinc/drawbridge/issues/18
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../rsa2048-priv.der")).unwrap();

    Router::new()
        .merge(auth_routes(
            host,
            env::var("CLIENT_ID").expect("CLIENT_ID env var"),
            env::var("CLIENT_SECRET").expect("CLIENT_SECRET env var"),
        ))
        .route("/protected", get(protected))
        .layer(AddExtensionLayer::new(key))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let host_uri = Uri::from_str(&args.host).expect("valid host uri");

    let addr: SocketAddr = host_uri
        .authority()
        .expect("expected authority in url")
        .to_string()
        .parse()
        .expect("parse socket address to host on");

    tracing::debug!("Listening on {}", addr);

    Server::bind(&addr)
        .serve(app(&args.host).into_make_service())
        .await
        .unwrap();
}
