// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;
use crate::auth::assert_repository_read;

use drawbridge_type::{Meta, RepositoryContext};

use async_std::sync::Arc;
use axum::body::Body;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::Extension;
use log::{debug, trace};
use mime::APPLICATION_JSON;

pub async fn query(
    Extension(ref store): Extension<Arc<Store>>,
    ref cx: RepositoryContext,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::tags::query", "called for `{cx}`");

    assert_repository_read(store, cx, req)
        .await
        .map_err(IntoResponse::into_response)
        .map(|(repo, _)| repo)?
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
            debug!(target: "app::tags::query", "failed: {:?}", e);
            e.into_response()
        })
}
