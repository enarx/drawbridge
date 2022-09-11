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
use tracing::{debug, trace};

pub async fn get(
    Extension(ref store): Extension<Arc<Store>>,
    cx: TagContext,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::tags::get", "called for `{cx}`");

    let (repo, _) = assert_repository_read(store, &cx.repository, req)
        .await
        .map_err(IntoResponse::into_response)?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    repo.tag(&cx.name)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            debug!(target: "app::tags::get", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, body))
}
