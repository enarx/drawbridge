// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use drawbridge_type::RepositoryContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use log::warn;

pub async fn head(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .get_meta()
        .await
        .map_err(|e| {
            warn!(target: "app::repos::head", "failed for `{repo}`: {:?}", e);
            e
        })
        .map(|meta| (meta, ()))
}
