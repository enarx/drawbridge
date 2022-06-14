// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

pub mod routes;

use super::super::session::Session;

use std::{fmt, io};

use axum::http::header::{AUTHORIZATION, USER_AGENT};
use axum::http::status::InvalidStatusCode;
use axum::http::StatusCode;
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

pub const LOGIN_URI: &str = "/github";
pub const AUTHORIZED_URI: &str = "/github/authorized";

#[derive(Debug)]
pub enum ValidateError {
    Http(ureq::Error),
    Json(io::Error),
    InvalidStatusCode(InvalidStatusCode),
    GitHub(String),
    Internal(String),
}

impl fmt::Display for ValidateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "session {}",
            match self {
                ValidateError::Http(e) => format!("HTTP request error: {}", e),
                ValidateError::Json(e) => format!("JSON deserialization error: {}", e),
                ValidateError::InvalidStatusCode(e) => format!("invalid status code: {}", e),
                ValidateError::GitHub(e) => format!("github error: {}", e),
                ValidateError::Internal(e) => format!("internal error: {}", e),
            }
        )
    }
}

impl std::error::Error for ValidateError {}

pub(crate) async fn validate(session: &Session) -> Result<String, ValidateError> {
    #[derive(Deserialize)]
    struct User {
        login: String,
    }

    #[derive(Deserialize)]
    struct Error {
        message: String,
    }

    let token = session
        .token
        .as_ref()
        .ok_or_else(|| ValidateError::Internal("No token in session".to_string()))?;

    let res = ureq::get("https://api.github.com/user")
        .set(USER_AGENT.as_str(), "drawbridge")
        .set(
            AUTHORIZATION.as_str(),
            &format!("Bearer {}", token.secret()),
        )
        .call()
        .map_err(ValidateError::Http)?;
    match StatusCode::from_u16(res.status()) {
        Ok(s) if s.is_success() => res
            .into_json()
            .map_err(ValidateError::Json)
            .map(|User { login }| login),
        Ok(_) => res
            .into_json()
            .map_err(ValidateError::Json)
            .and_then(|Error { message }| Err(ValidateError::GitHub(message))),
        Err(e) => Err(ValidateError::InvalidStatusCode(e)),
    }
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
