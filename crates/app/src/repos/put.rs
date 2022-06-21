// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::{Meta, RepositoryConfig, RepositoryContext};

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use log::debug;

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: RepositoryContext,
    meta: Meta,
    Json(config): Json<RepositoryConfig>,
) -> impl IntoResponse {
    let (oidc_cx, user) = claims
        .get_user(&store)
        .await
        .map_err(IntoResponse::into_response)?;
    if oidc_cx != cx.owner {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!(
                "You are logged in as `{oidc_cx}`, please relogin as `{}` to access `{cx}`",
                cx.owner
            ),
        )
            .into_response());
    }

    user.create_repository(&cx.name, meta, &config)
        .await
        .map_err(|e| {
            debug!(target: "app::repos::put", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}
