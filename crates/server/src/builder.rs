// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, App, Store, TlsConfig};

use anyhow::{anyhow, Context};
use async_std::fs::File;
use async_std::sync::Arc;
use axum::handler::Handler;
use axum::routing::any;
use axum::{Extension, Router};
use cap_async_std::fs_utf8::Dir;
use cap_async_std::path::Path;
use futures::lock::Mutex;
use futures::TryFutureExt;
use futures_rustls::TlsAcceptor;
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::ureq::http_client;
use openidconnect::url::Url;
use openidconnect::{AuthType, ClientId, ClientSecret, IssuerUrl};
use tower_http::{
    trace::{
        DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse,
        TraceLayer,
    },
    LatencyUnit,
};
use tracing::Level;

/// OpenID Connect client configuration.
#[derive(Debug)]
pub struct OidcConfig {
    pub label: String,
    pub issuer: Url,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SpanMaker;

impl<B> tower_http::trace::MakeSpan<B> for SpanMaker {
    fn make_span(&mut self, request: &axum::http::request::Request<B>) -> tracing::span::Span {
        let reqid = uuid::Uuid::new_v4();
        tracing::span!(
            Level::INFO,
            "request",
            method = %request.method(),
            uri = %request.uri(),
            version = ?request.version(),
            headers = ?request.headers(),
            request_id = %reqid,
        )
    }
}

/// [App] builder.
pub struct Builder<S> {
    store: S,
    tls: TlsConfig,
    oidc: OidcConfig,
}

impl<S: std::fmt::Debug> std::fmt::Debug for Builder<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder")
            .field("store", &self.store)
            .field("oidc", &self.oidc)
            .finish()
    }
}

impl<S: AsRef<Path>> Builder<S> {
    /// Constructs a new [Builder].
    pub fn new(store: S, tls: TlsConfig, oidc: OidcConfig) -> Self {
        Self { store, tls, oidc }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub async fn build(self) -> anyhow::Result<App> {
        let Self { store, tls, oidc } = self;
        let store_path = store.as_ref();
        let store = File::open(store_path)
            .and_then(|f| Store::new(Dir::from_std_file(f), oidc.label))
            .await
            .context(anyhow!(
                "failed to open store at `{}`",
                store_path.to_string_lossy()
            ))?;

        let oidc_md =
            CoreProviderMetadata::discover(&IssuerUrl::from_url(oidc.issuer), http_client)
                .context("failed to discover provider metadata")?;
        let oidc = CoreClient::from_provider_metadata(
            oidc_md,
            ClientId::new(oidc.client_id),
            oidc.client_secret.map(ClientSecret::new),
        )
        .set_auth_type(AuthType::RequestBody);

        Ok(App {
            make_service: Mutex::new(
                Router::new()
                    .fallback(handle.into_service())
                    .route("/health", any(|| async {}))
                    .layer(Extension(Arc::new(store)))
                    .layer(Extension(oidc))
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(SpanMaker::default())
                            .on_request(DefaultOnRequest::new().level(Level::INFO))
                            .on_response(
                                DefaultOnResponse::new()
                                    .level(Level::INFO)
                                    .latency_unit(LatencyUnit::Micros),
                            )
                            .on_body_chunk(DefaultOnBodyChunk::new())
                            .on_eos(
                                DefaultOnEos::new()
                                    .level(Level::INFO)
                                    .latency_unit(LatencyUnit::Micros),
                            )
                            .on_failure(
                                DefaultOnFailure::new()
                                    .level(Level::INFO)
                                    .latency_unit(LatencyUnit::Micros),
                            ),
                    )
                    .into_make_service(),
            ),
            tls: TlsAcceptor::from(Arc::new(tls.into())),
        })
    }
}
