// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::UserContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use tracing::{debug, trace};

pub async fn get(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    ref cx: UserContext,
) -> impl IntoResponse {
    trace!(target: "app::users::get", "called for `{cx}`");

    let user = claims
        .assert_user(store, cx)
        .await
        .map_err(IntoResponse::into_response)?;

    // TODO: Stream body
    // https://github.com/profianinc/drawbridge/issues/56
    let mut body = vec![];
    user.get_to_writer(&mut body)
        .await
        .map_err(|e| {
            debug!(target: "app::users::get", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, body))
}
