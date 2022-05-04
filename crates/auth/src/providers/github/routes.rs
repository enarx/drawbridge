// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0
use crate::{
    providers::{github::OAuthClient, Provider},
    session::Session,
};

use axum::{
    extract::{Extension, Query},
    response::{IntoResponse, Redirect},
};
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthorizationCode, CsrfToken, Scope, TokenResponse,
};
use rsa::RsaPrivateKey;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub code: String,
    pub state: String,
}

/// Authenticate with GitHub OAuth.
pub async fn login(Extension(OAuthClient(client)): Extension<OAuthClient>) -> impl IntoResponse {
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    Redirect::to(auth_url.as_str())
}

/// Prepare an encrypted token for GitHub OAuth.
pub async fn authorized(
    query: Query<AuthRequest>,
    oauth_client: Extension<BasicClient>,
    key: Extension<RsaPrivateKey>,
) -> Result<String, String> {
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request(http_client)
        .map_err(|e| format!("Failed to get token: {}", e))?;

    // TODO: pull user info from the GitHub API here: https://github.com/profianinc/drawbridge/issues/7
    Session::new(Provider::GitHub, token.access_token().clone())
        .encrypt(&key.0)
        .map_err(|e| format!("Failed to encrypt token: {}", e))
}
