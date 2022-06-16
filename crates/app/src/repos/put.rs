// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{Meta, RepositoryConfig, RepositoryContext};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use log::warn;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
    meta: Meta,
    Json(config): Json<RepositoryConfig>,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .create(meta, &config)
        .await
        .map_err(|e| {
            warn!(target: "app::repos::put", "failed for `{repo}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}
