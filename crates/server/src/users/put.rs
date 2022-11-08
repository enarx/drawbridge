// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::{Meta, UserContext, UserRecord};

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use tracing::{debug, trace};

pub async fn put(
    Extension(ref store): Extension<Arc<Store>>,
    claims: OidcClaims,
    ref cx: UserContext,
    meta: Meta,
    Json(ref record): Json<UserRecord>,
) -> impl IntoResponse {
    trace!(target: "app::users::put", "called for `{cx}`");

    if let Some(nickname) = claims.nickname().and_then(|n| n.get(None)) {
        if nickname.as_str() != *cx.name {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Username and OpenID Connect nickname mismatch",
            )
                .into_response());
        }
    } else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "OpenID Connect nickname not present",
        )
            .into_response());
    }
    if record.subject != claims.subject().as_str() {
        return Err((StatusCode::UNAUTHORIZED, "OpenID Connect subject mismatch").into_response());
    }

    store
        .create_user(cx, meta, record)
        .await
        .map_err(|e| {
            debug!(target: "app::users::put", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}
