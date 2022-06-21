// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{GetError, OidcClaims, Store};

use drawbridge_type::{Meta, UserContext, UserRecord};

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use log::{debug, warn};

pub async fn put(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: UserContext,
    meta: Meta,
    Json(record): Json<UserRecord>,
) -> impl IntoResponse {
    if record.subject != claims.subject().as_str() {
        return Err((StatusCode::UNAUTHORIZED, "OpenID Connect subject mismatch").into_response());
    }

    // TODO: Remove this check once there is support for transactions and rollbacks.
    // https://github.com/profianinc/drawbridge/issues/144
    let subj = claims.subject();
    match store.user_by_subject(subj).await {
        Err(GetError::NotFound) => (),
        Err(e) => {
            warn!(target: "app::users::put", "failed to get user by OpenID Connect subject `{}`: {:?}", subj.as_str(), e);
            return Err(e.into_response());
        }
        Ok(_) => {
            return Err((
                StatusCode::CONFLICT,
                format!(
                    "User already associated with OpenID Connect subject `{}`",
                    subj.as_str()
                ),
            )
                .into_response())
        }
    }

    store
        .create_user(&cx, meta, &record)
        .await
        .map_err(|e| {
            debug!(target: "app::users::put", "failed for `{cx}`: {:?}", e);
            e.into_response()
        })
        .map(|_| StatusCode::CREATED)
}
