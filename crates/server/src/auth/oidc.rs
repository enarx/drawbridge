// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::store::GetError;

use super::super::{Store, User};

use drawbridge_type::{UserContext, UserRecord};

use std::ops::Deref;

use axum::extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason};
use axum::extract::{Extension, FromRequest, RequestParts};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, TypedHeader};
use openidconnect::core::{CoreClient, CoreUserInfoClaims};
use openidconnect::ureq::http_client;
use openidconnect::AccessToken;
use tracing::{debug, error, trace, warn};

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
    /// Assert that the client is the user identified by `cx`.
    pub async fn assert_user<'a>(
        &self,
        store: &'a Store,
        cx: &UserContext,
    ) -> Result<User<'a>, impl IntoResponse> {
        let subj = self.0.subject().as_str();
        let oidc_record = UserRecord {
            subject: subj.to_string(),
        };

        let user = store.user(cx);
        let owner_record: UserRecord = user.get_content_json().await.map_err(|e|{
            match e {
                GetError::NotFound => (StatusCode::UNAUTHORIZED, format!("User `{cx}` not found")).into_response(),
                _ => {
            warn!(target: "app::auth::oidc", ?oidc_record, error = ?e, "failed to get user by OpenID Connect subject");
e.into_response()
                },
            }})?;

        if oidc_record != owner_record {
            warn!(target: "app::auth::oidc", ?oidc_record, user = ?cx, ?owner_record, "User access not authorized");
            return Err((
                StatusCode::UNAUTHORIZED,
                format!("You are logged in as `{subj}`, and not authorized for user `{cx}`"),
            )
                .into_response());
        }

        Ok(user)
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
        let claims = info_req.request(http_client).map_err(|e| {
            debug!(target: "app::auth::oidc", "failed to request user info: {e}");
            (
                StatusCode::UNAUTHORIZED,
                format!("OpenID Connect credential validation failed: {e}"),
            )
                .into_response()
        })?;
        trace!(target: "app:auth::oidc", "received user claims: {:?}", claims);
        Ok(Self(claims))
    }
}
