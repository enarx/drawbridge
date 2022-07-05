// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, App, Store, TlsConfig};

use anyhow::{anyhow, Context};
use async_std::fs::File;
use async_std::sync::Arc;
use axum::handler::Handler;
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

/// OpenID Connect client configuration.
pub struct OidcConfig {
    pub label: String,
    pub issuer: Url,
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// [App] builder.
pub struct Builder<S> {
    store: S,
    tls: TlsConfig,
    oidc: OidcConfig,
}

impl<S: AsRef<Path>> Builder<S> {
    /// Constructs a new [Builder].
    pub fn new(store: S, tls: TlsConfig, oidc: OidcConfig) -> Self {
        Self { store, tls, oidc }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub async fn build(self) -> anyhow::Result<App> {
        let store_path = self.store.as_ref();
        let store = File::open(store_path)
            .and_then(|f| Store::new(Dir::from_std_file(f), self.oidc.label))
            .await
            .context(anyhow!(
                "failed to open store at `{}`",
                store_path.to_string_lossy()
            ))?;

        let oidc_md =
            CoreProviderMetadata::discover(&IssuerUrl::from_url(self.oidc.issuer), http_client)
                .context("failed to discover provider metadata")?;
        let oidc = CoreClient::from_provider_metadata(
            oidc_md,
            ClientId::new(self.oidc.client_id),
            self.oidc.client_secret.map(ClientSecret::new),
        )
        .set_auth_type(AuthType::RequestBody);

        Ok(App {
            make_service: Mutex::new(
                Router::new()
                    .fallback(handle.into_service())
                    .layer(Extension(Arc::new(store)))
                    .layer(Extension(oidc))
                    .into_make_service(),
            ),
            tls: TlsAcceptor::from(Arc::new(self.tls.into())),
        })
    }
}
