// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;

use drawbridge_type::{Meta, RepositoryContext};

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use log::warn;
use mime::APPLICATION_JSON;

pub async fn query(
    Extension(store): Extension<Arc<Store>>,
    repo: RepositoryContext,
) -> impl IntoResponse {
    store
        .repository(&repo)
        .tags_json()
        .await
        .map(|(hash, buf)| {
            (
                Meta {
                    hash,
                    size: buf.len() as _,
                    mime: APPLICATION_JSON,
                },
                buf,
            )
        })
        .map_err(|e| {
            warn!(target: "app::tags::query", "failed: {:?}", e);
            e
        })
}
