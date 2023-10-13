// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::super::{Store, TrustedCertificate};
use crate::auth::assert_repository_read;

use drawbridge_type::TreeContext;

use async_std::sync::Arc;
use axum::body::Body;
use axum::http::Request;
use axum::response::IntoResponse;
use axum::Extension;
use tracing::{debug, trace};

pub async fn head(
    Extension(ref store): Extension<Arc<Store>>,
    cert: Option<Extension<TrustedCertificate>>,
    cx: TreeContext,
    req: Request<Body>,
) -> impl IntoResponse {
    trace!(target: "app::trees::head", "called for `{cx}`");

    if cert.is_none() {
        assert_repository_read(store, &cx.tag.repository, req)
            .await
            .map_err(IntoResponse::into_response)
            .map(|(repo, _)| repo)?
    } else {
        store.repository(&cx.tag.repository)
    }
    .tag(&cx.tag.name)
    .node(&cx.path)
    .get_meta()
    .await
    .map_err(|e| {
        debug!(target: "app::trees::head", "failed for `{cx}`: {:?}", e);
        e.into_response()
    })
    .map(|meta| (meta, ()))
}
