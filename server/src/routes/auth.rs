use axum::{
    extract::{Extension, Query},
    response::{IntoResponse, Redirect},
    routing::get,
    AddExtensionLayer, Router,
};
use drawbridge_common::endpoint::{GITHUB, GITHUB_AUTHORIZED, STATUS};
use http::StatusCode;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use rsa::RsaPrivateKey;

use crate::types::{AuthRequest, AuthType, User};

pub static AUTH_URL: &str = "https://github.com/login/oauth/authorize?response_type=code";
pub static TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub fn auth_routes(host: &str, client_id: String, client_secret: String) -> Router {
    let oauth_client = BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(AUTH_URL.to_string()).unwrap(),
        Some(TokenUrl::new(TOKEN_URL.to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(format!("http://{}{}", host, GITHUB_AUTHORIZED)).unwrap());

    Router::new()
        .route(STATUS, get(status))
        .route(GITHUB_AUTHORIZED, get(github_login_authorized))
        .route(GITHUB, get(github_login))
        .layer(AddExtensionLayer::new(oauth_client))
}

/// Check if a user is authenticated.
async fn status(user: Option<User>) -> (StatusCode, String) {
    match user {
        None => (StatusCode::FORBIDDEN, "Not logged in.".to_owned()),
        Some(user) => match user.validate().await {
            Err(e) => (StatusCode::FORBIDDEN, format!("Invalid session: {}", e)),
            Ok(username) => (
                StatusCode::OK,
                format!("Logged in as {} via {}.", username, user.auth_type),
            ),
        },
    }
}

/// Authenticate with GitHub OAuth.
async fn github_login(client: Extension<BasicClient>) -> impl IntoResponse {
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    Redirect::to(auth_url.to_string().parse().unwrap())
}

/// Prepare an encrypted token for GitHub OAuth.
async fn github_login_authorized(
    query: Query<AuthRequest>,
    oauth_client: Extension<BasicClient>,
    key: Extension<RsaPrivateKey>,
) -> Result<String, String> {
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
        .await
        .map_err(|e| format!("Failed to get token: {}", e))?;

    // TODO: use reqwest to pull user info from the GitHub API here:
    User {
        auth_type: AuthType::GitHub,
        token: token.access_token().clone(),
    }
    .encrypt(&key.0)
    .map_err(|e| format!("Failed to encrypt token: {}", e))
}
