// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::{GetError, Store, User};

use drawbridge_type::UserContext;

use std::ops::Deref;

use axum::extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason};
use axum::extract::{Extension, FromRequest, RequestParts};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, TypedHeader};
use log::{debug, error, trace, warn};
use openidconnect::core::{CoreClient, CoreUserInfoClaims};
use openidconnect::ureq::http_client;
use openidconnect::AccessToken;

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Claims(CoreUserInfoClaims);

impl Deref for Claims {
    type Target = CoreUserInfoClaims;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Claims {
    /// Gets the user that the client is claiming to be.
    pub async fn get_user<'a>(
        &self,
        store: &'a Store,
    ) -> Result<(UserContext, User<'a>), impl IntoResponse> {
        let subj = self.0.subject();
        store.user_by_subject(subj).await.map_err(|e|{
            match e {
                GetError::NotFound => (StatusCode::UNAUTHORIZED, format!("User with OpenID Connect subject `{}` not found", subj.as_str())).into_response(),
                _ => {
            warn!(target: "app::auth::oidc", "failed to get user by OpenID Connect subject `{}`: {:?}", subj.as_str(), e);
e.into_response()
                },
            }
        })
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for Claims {
    type Rejection = Response;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization::<Bearer>(token)) =
            req.extract()
                .await
                .map_err(|e: TypedHeaderRejection| match e.reason() {
                    TypedHeaderRejectionReason::Missing => {
                        (StatusCode::UNAUTHORIZED, "Bearer token header missing").into_response()
                    }
                    _ => e.into_response(),
                })?;
        let token = AccessToken::new(token.token().into());

        let Extension(oidc) = req.extract::<Extension<CoreClient>>().await.map_err(|e| {
            error!(target: "app::auth::oidc", "OpenID Connect client extension missing");
            e.into_response()
        })?;

        let info_req = oidc.user_info(token, None).map_err(|e| {
            error!(target: "app::auth::oidc", "failed to construct user info request: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "OpenID Connect client initialization failed",
            )
                .into_response()
        })?;

        trace!(target: "app:auth::oidc", "request user info");
        info_req.request(http_client).map(Self).map_err(|e| {
            debug!(target: "app::auth::oidc", "failed to request user info: {e}");
            (
                StatusCode::UNAUTHORIZED,
                format!("OpenID Connect credential validation failed: {e}"),
            )
                .into_response()
        })
    }
}
