// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Store;
use crate::auth::assert_repository_read;

use drawbridge_type::TagContext;

use async_std::sync::Arc;
use axum::body::Body;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::Extension;
use log::{debug, trace};

pub async fn head(
    Extension(ref store): Extension<Arc<Store>>,
    cx: TagContext,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::tags::head", "called for `{cx}`");

    assert_repository_read(store, &cx.repository, req)
        .await
        .map_err(IntoResponse::into_response)
        .map(|(repo, _)| repo)?
        .tag(&cx.name)
        .get_meta()
        .await
        .map_err(|e| {
            debug!(target: "app::tags::head", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, ()))
}
