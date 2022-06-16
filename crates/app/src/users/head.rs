// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use drawbridge_type::UserContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use log::warn;

pub async fn head(Extension(store): Extension<Arc<Store>>, user: UserContext) -> impl IntoResponse {
    store
        .user(&user)
        .get_meta()
        .await
        .map_err(|e| {
            warn!(target: "app::users::head", "failed for `{user}`: {:?}", e);
            e
        })
        .map(|meta| (meta, ()))
}
