// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{handle, RepoStore, TagStore, TreeStore};

use std::sync::Arc;

use axum::handler::Handler;
use axum::routing::IntoMakeService;
use axum::{Extension, Router};

#[derive(Default)]
pub struct Builder;

impl Builder {
    pub fn new() -> Self {
        Self
    }

    // TODO: Add configuration functionality

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> IntoMakeService<Router> {
        let repos: Arc<RepoStore> = Default::default();
        let tags: Arc<TagStore> = Default::default();
        let trees: Arc<TreeStore> = Default::default();
        Router::new()
            .fallback(handle.into_service())
            .layer(Extension(repos))
            .layer(Extension(tags))
            .layer(Extension(trees))
            .into_make_service()
    }
}
