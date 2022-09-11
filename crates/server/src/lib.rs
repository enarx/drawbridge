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
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_results,
    variant_size_differences
)]

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
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tower::MakeService;
use tracing::trace;

#[allow(missing_debug_implementations)] // TlsAcceptor does not implement Debug
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
