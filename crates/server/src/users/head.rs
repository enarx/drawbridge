// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::super::{OidcClaims, ScopeContext, ScopeLevel, Store};

use drawbridge_type::UserContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use tracing::{debug, trace};

pub async fn head(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: UserContext,
) -> impl IntoResponse {
    trace!(target: "app::users::head", "called for `{cx}`");

    claims
        .assert_user(&store, &cx, ScopeContext::User, ScopeLevel::Read)
        .await
        .map_err(IntoResponse::into_response)?
        .get_meta()
        .await
        .map_err(|e| {
            debug!(target: "app::users::head", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, ()))
}
