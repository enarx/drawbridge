// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::super::{OidcClaims, ScopeContext, ScopeLevel, Store};

use drawbridge_type::{Meta, RepositoryConfig, RepositoryContext};

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use tracing::{debug, trace};

pub async fn put(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: RepositoryContext,
    meta: Meta,
    Json(config): Json<RepositoryConfig>,
) -> impl IntoResponse {
    trace!(target: "app::trees::put", "called for `{cx}`");

    claims
        .assert_user(
            store,
            &cx.owner,
            ScopeContext::Repository,
            ScopeLevel::Write,
        )
        .await
        .map_err(IntoResponse::into_response)?
        .create_repository(&cx.name, meta, &config)
        .await
        .map_err(|e| {
            debug!(target: "app::repos::put", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}
