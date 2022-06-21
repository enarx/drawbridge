// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{OidcClaims, Store, TrustedCertificate};

use drawbridge_type::TreeContext;

use async_std::sync::Arc;
use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::Extension;
use log::debug;

pub async fn head(
    Extension(store): Extension<Arc<Store>>,
    cert: Option<Extension<TrustedCertificate>>,
    cx: TreeContext,
    req: Request<Body>,
) -> impl IntoResponse {
    // TODO: Check if repo is public
    let user = if cert.is_none() {
        let (oidc_cx, user) = RequestParts::new(req)
            .extract::<OidcClaims>()
            .await?
            .get_user(&store)
            .await
            .map_err(IntoResponse::into_response)?;
        if oidc_cx != cx.tag.repository.owner {
            return Err((
                StatusCode::UNAUTHORIZED,
                format!(
                    "You are logged in as `{oidc_cx}`, please relogin as `{}` to access `{cx}`",
                    cx.tag.repository.owner
                ),
            )
                .into_response());
        }
        user
    } else {
        store.user(&cx.tag.repository.owner)
    };

    user.repository(&cx.tag.repository.name)
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
