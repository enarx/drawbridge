// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod builder;
mod handle;

pub mod auth;
pub mod repos;
pub mod store;
pub mod tags;
pub mod trees;
pub mod users;

pub use auth::{OidcClaims, TlsConfig, TrustedCertificate};
pub use builder::*;
pub(crate) use handle::*;
pub(crate) use store::*;

pub use openidconnect::url;

use anyhow::Context as _;
use axum::extract::Extension;
use axum::routing::IntoMakeService;
use axum::Router;
use cap_async_std::path::Path;
use futures::lock::Mutex;
use futures::{AsyncRead, AsyncWrite};
use futures_rustls::TlsAcceptor;
use hyper::server::conn::Http;
use log::trace;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tower::MakeService;

pub struct App {
    make_service: Mutex<IntoMakeService<Router>>,
    tls: TlsAcceptor,
}

impl App {
    pub fn builder<S: AsRef<Path>>(store: S, tls: TlsConfig, oidc: OidcConfig) -> Builder<S> {
        Builder::new(store, tls, oidc)
    }

    pub async fn new(
        store: impl AsRef<Path>,
        tls: TlsConfig,
        oidc: OidcConfig,
    ) -> anyhow::Result<Self> {
        Self::builder(store, tls, oidc).build().await
    }

    pub async fn handle(
        &self,
        stream: impl 'static + Unpin + AsyncRead + AsyncWrite,
    ) -> anyhow::Result<()> {
        trace!(target: "app::App::handle", "begin TLS handshake");
        let stream = self
            .tls
            .accept(stream)
            .await
            .context("failed to accept TLS connection")?;
        trace!(target: "app::App::handle", "completed TLS handshake");

        let mut svc = self
            .make_service
            .lock()
            .await
            .make_service(())
            .await
            .context("failed to create app service")?;
        let (_, conn) = stream.get_ref();
        if conn.peer_certificates().is_some() {
            svc = svc.layer(Extension(TrustedCertificate));
            trace!(target: "app::App::handle", "add TrustedCertificate to extensions");
        }
        trace!(target: "app::App::handle", "begin HTTP request serving");
        Http::new()
            .serve_connection(stream.compat(), svc)
            .await
            .context("failed to handle request")
    }
}
