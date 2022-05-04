// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

pub mod routes;

use crate::error::Error;
use crate::session::Session;

use axum::http::header::USER_AGENT;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

pub const LOGIN_URI: &str = "/github";
pub const AUTHORIZED_URI: &str = "/github/authorized";

pub async fn validate(session: &Session) -> Result<String, Error> {
    #[derive(Deserialize)]
    struct GitHubUser {
        login: String,
    }

    #[derive(Deserialize)]
    struct GitHubError {
        message: String,
    }

    let client = reqwest::Client::new();

    let body = client
        .get("https://api.github.com/user")
        .header(USER_AGENT, "drawbridge")
        .bearer_auth(session.token.secret())
        .send()
        .await
        .map_err(Error::Request)?
        .text()
        .await
        .map_err(Error::Request)?;

    let user =
        serde_json::from_str::<GitHubUser>(&body).map_err(|_| {
            match serde_json::from_str::<GitHubError>(&body) {
                Err(e) => Error::Serde(e.to_string()),
                Ok(error) => Error::OAuth(error.message),
            }
        })?;

    Ok(user.login)
}

#[derive(Clone)]
pub struct OAuthClient(pub BasicClient);

impl OAuthClient {
    pub fn new(host: &str, client_id: String, client_secret: String) -> OAuthClient {
        const AUTH_URL: &str = "https://github.com/login/oauth/authorize?response_type=code";
        const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

        OAuthClient(
            BasicClient::new(
                ClientId::new(client_id),
                Some(ClientSecret::new(client_secret)),
                AuthUrl::new(AUTH_URL.to_string()).unwrap(),
                Some(TokenUrl::new(TOKEN_URL.to_string()).unwrap()),
            )
            .set_redirect_uri(
                RedirectUrl::new(format!("http://{}{}", host, AUTHORIZED_URI)).unwrap(),
            ),
        )
    }
}
