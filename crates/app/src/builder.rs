// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, App, Store, TlsConfig};

use anyhow::{anyhow, Context};
use async_std::fs::File;
use async_std::sync::Arc;
use axum::handler::Handler;
use axum::{Extension, Router};
use cap_async_std::path::Path;
use futures::lock::Mutex;
use futures_rustls::TlsAcceptor;

pub struct Builder<S> {
    store: S,
    tls: TlsConfig,
}

impl<S: AsRef<Path>> Builder<S> {
    /// Constructs a new [Builder].
    pub fn new(store: S, tls: TlsConfig) -> Self {
        Self { store, tls }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub async fn build(self) -> anyhow::Result<App> {
        let path = self.store.as_ref();
        let store = File::open(path).await.map(Store::from).context(anyhow!(
            "failed to open store at `{}`",
            path.to_string_lossy()
        ))?;
        Ok(App {
            make_service: Mutex::new(
                Router::new()
                    .fallback(handle.into_service())
                    .layer(Extension(Arc::new(store)))
                    .into_make_service(),
            ),
            tls: TlsAcceptor::from(Arc::new(self.tls.into())),
        })
    }
}
