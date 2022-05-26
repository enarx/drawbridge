// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, RepoStore, TagStore, TreeStore};

use drawbridge_store::Filesystem;

use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use axum::handler::Handler;
use axum::routing::IntoMakeService;
use axum::{Extension, Router};
use cap_async_std::fs::Dir;
use tokio::sync::RwLock;

pub struct Builder<'a> {
    data_directory: &'a Path,
}

impl<'a> Builder<'a> {
    /// Constructs a new [Builder].
    pub fn new(data_directory: &'a Path) -> Self {
        Self { data_directory }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> Result<IntoMakeService<Router>, Box<dyn Error>> {
        let path = self.data_directory;

        if !path.exists() {
            return Err(format!(
                "The configured data directory does not exist: {}. Please create it or select another directory.",
                self.data_directory.display()
            ).into());
        }

        let root = Dir::from_std_file(File::open(path)?.into());
        let repos: Arc<RepoStore> = Arc::new(RwLock::new(Filesystem::new(
            root.clone(),
            "repo".to_string(),
        )));
        let tags: Arc<TagStore> = Arc::new(RwLock::new(Filesystem::new(
            root.clone(),
            "tag".to_string(),
        )));
        let trees: Arc<TreeStore> =
            Arc::new(RwLock::new(Filesystem::new(root, "tree".to_string())));
        Ok(Router::new()
            .fallback(handle.into_service())
            .layer(Extension(repos))
            .layer(Extension(tags))
            .layer(Extension(trees))
            .into_make_service())
    }
}
