// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::RepositoryContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use log::{debug, trace};

pub async fn get(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: RepositoryContext,
) -> impl IntoResponse {
    trace!(target: "app::trees::get", "called for `{cx}`");

    let user = claims
        .assert_user(store, &cx.owner)
        .await
        .map_err(IntoResponse::into_response)?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    user.repository(&cx.name)
        .get_to_writer(&mut body)
        .await
        .map_err(|e| {
            debug!(target: "app::repos::get", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, body))
}
