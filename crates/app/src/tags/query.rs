// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store};

use drawbridge_type::{Meta, RepositoryContext};

use async_std::sync::Arc;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Extension;
use log::debug;
use mime::APPLICATION_JSON;

pub async fn query(
    Extension(store): Extension<Arc<Store>>,
    claims: OidcClaims,
    cx: RepositoryContext,
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

    user.repository(&cx.name)
        .tags_json()
        .await
        .map(|(hash, buf)| {
            (
                Meta {
                    hash,
                    size: buf.len() as _,
                    mime: APPLICATION_JSON,
                },
                buf,
            )
        })
        .map_err(|e| {
            debug!(target: "app::tags::query", "failed: {:?}", e);
            e.into_response()
        })
}
