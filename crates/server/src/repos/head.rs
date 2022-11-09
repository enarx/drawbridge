// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, ScopeContext, ScopeLevel, Store};

use drawbridge_type::RepositoryContext;

use async_std::sync::Arc;
use axum::response::IntoResponse;
use axum::Extension;
use tracing::{debug, trace};

pub async fn head(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: RepositoryContext,
) -> impl IntoResponse {
    trace!(target: "app::trees::head", "called for `{cx}`");

    claims
        .assert_user(store, &cx.owner, ScopeContext::Repository, ScopeLevel::Read)
        .await
        .map_err(IntoResponse::into_response)?
        .repository(&cx.name)
        .get_meta()
        .await
        .map_err(|e| {
            debug!(target: "app::repos::head", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|meta| (meta, ()))
}
