// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod oidc;
mod tls;

pub use oidc::Claims as OidcClaims;
pub use oidc::Verifier as OidcVerifier;
pub use oidc::{ScopeContext, ScopeLevel};
pub use tls::{Config as TlsConfig, TrustedCertificate};

use super::{Repository, Store, User};

use drawbridge_type::RepositoryContext;

use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::Request;
use axum::response::IntoResponse;

pub async fn assert_repository_read<'a>(
    store: &'a Store,
    cx: &'a RepositoryContext,
    req: Request<Body>,
) -> Result<(Repository<'a>, Option<User<'a>>), impl IntoResponse> {
    let repo = store.repository(cx);
    if repo
        .is_public()
        .await
        .map_err(IntoResponse::into_response)?
    {
        Ok((repo, None))
    } else {
        RequestParts::new(req)
            .extract::<OidcClaims>()
            .await?
            .assert_user(store, &cx.owner, ScopeContext::Repository, ScopeLevel::Read)
            .await
            .map_err(IntoResponse::into_response)
            .map(|user| (repo, Some(user)))
    }
}
