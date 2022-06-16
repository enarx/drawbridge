// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use std::sync::Arc;

use drawbridge_type::{Meta, UserConfig, UserContext};

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use log::warn;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    user: UserContext,
    meta: Meta,
    Json(config): Json<UserConfig>,
) -> impl IntoResponse {
    store
        .user(&user)
        .create(meta, &config)
        .await
        .map_err(|e| {
            warn!(target: "app::users::put", "failed for `{user}`: {:?}", e);
            e
        })
        .map(|_| StatusCode::CREATED)
}
