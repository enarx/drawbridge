// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::RepositoryContext;

use axum::response::IntoResponse;
use axum::Extension;

pub async fn head(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .get_meta()
        .await
        .map_err(|e| {
            eprintln!("Failed to HEAD repository `{}`: {:?}", repo, e);
            e
        })
        .map(|meta| (meta, ()))
}
