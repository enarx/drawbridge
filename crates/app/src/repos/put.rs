// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{Meta, RepositoryConfig, RepositoryName};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    Extension(name): Extension<RepositoryName>,
    meta: Meta,
    Json(config): Json<RepositoryConfig>,
) -> impl IntoResponse {
    store
        .repository(&name)
        .create(meta, &config)
        .await
        .map_err(|e| {
            eprintln!("Failed to PUT repository `{}`: {:?}", name, e);
            e
        })
        .map(|_| StatusCode::CREATED)
}
