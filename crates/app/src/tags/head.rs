// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{RepositoryName, TagName};

use axum::response::IntoResponse;
use axum::Extension;

pub async fn head(
    Extension(store): Extension<Arc<Store>>,
    Extension(repo): Extension<RepositoryName>,
    Extension(name): Extension<TagName>,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .tag(&name)
        .get_meta()
        .await
        .map_err(|e| {
            eprintln!(
                "Failed to HEAD tag `{}` on repository `{}`: {:?}",
                name, repo, e
            );
            e
        })
        .map(|meta| (meta, ()))
}
