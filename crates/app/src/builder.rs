// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, Store};

use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use axum::handler::Handler;
use axum::routing::IntoMakeService;
use axum::{Extension, Router};

pub struct Builder<S> {
    store: S,
}

impl<S: AsRef<Path>> Builder<S> {
    /// Constructs a new [Builder].
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> Result<IntoMakeService<Router>, Box<dyn Error>> {
        let path = self.store.as_ref();
        let store = File::open(path).map(Store::from).context(format!(
            "failed to open store at `{}`",
            path.to_string_lossy()
        ))?;
        Ok(Router::new()
            .fallback(handle.into_service())
            .layer(Extension(Arc::new(store)))
            .into_make_service())
    }
}
