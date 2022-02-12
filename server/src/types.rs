use std::fmt;

use anyhow::Context;
use axum::{
    async_trait,
    extract::{
        rejection::TypedHeaderRejectionReason, Extension, FromRequest, RequestParts, TypedHeader,
    },
    response::{IntoResponse, Redirect, Response},
};
use drawbridge_common::{endpoint::GITHUB, COOKIE_NAME};
use http::header::{self, USER_AGENT};
use oauth2::AccessToken;
use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum AuthType {
    GitHub,
}

impl fmt::Display for AuthType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AuthType::GitHub => "GitHub.com",
            }
        )
    }
}

#[test]
fn auth_type_display() {
    assert_eq!(format!("{}", AuthType::GitHub), "GitHub.com");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub auth_type: AuthType,
    pub token: AccessToken,
}

impl User {
    /// Decrypt an untrusted user token to guarantee it was not modified by the user.
    pub fn from_encrypted_str(string: &str, key: &RsaPrivateKey) -> anyhow::Result<Self> {
        let bytes = base64::decode(string).with_context(|| "decode base64")?;
        let bytes = key
            .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &bytes)
            .with_context(|| "failed to decrypt")?;

        bincode::deserialize(&bytes).with_context(|| "serialize user data")
    }

    /// Encrypt the session so that it can be securely stored by the user.
    pub fn encrypt(&self, key: &RsaPrivateKey) -> anyhow::Result<String> {
        let bytes = bincode::serialize(self).with_context(|| "serialize user data")?;
        let bytes = key
            .encrypt(
                &mut OsRng,
                PaddingScheme::new_pkcs1v15_encrypt(),
                &bytes[..],
            )
            .with_context(|| "failed to encrypt")?;

        Ok(base64::encode(bytes))
    }

    /// Validate if a session should be considered active.
    pub async fn validate(&self) -> Result<String, anyhow::Error> {
        match self.auth_type {
            AuthType::GitHub => {
                let client = reqwest::Client::new();

                #[derive(Deserialize)]
                struct GitHubUser {
                    login: String,
                }

                #[derive(Deserialize)]
                struct GitHubError {
                    message: String,
                }

                let body = client
                    .get("https://api.github.com/user")
                    .header(USER_AGENT, "drawbridge")
                    .bearer_auth(self.token.secret())
                    .send()
                    .await?
                    .text()
                    .await?;

                match serde_json::from_str::<GitHubUser>(&body) {
                    Err(_) => match serde_json::from_str::<GitHubError>(&body) {
                        Err(e) => {
                            tracing::error!("Failed to parse GitHub response: {}: {}", e, body);
                            bail!("Failed to parse GitHub response")
                        }
                        Ok(error) => {
                            bail!(format!("GitHub request failed: {}", error.message))
                        }
                    },
                    Ok(user) => Ok(user.login),
                }
            }
        }
    }
}

#[test]
fn user_encrypt_decrypt() {
    use rsa::pkcs8::FromPrivateKey;

    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../rsa2048-priv.der")).unwrap();
    let user = User {
        auth_type: AuthType::GitHub,
        token: AccessToken::new("some_token".to_owned()),
    };

    assert_eq!(
        serde_json::to_string(
            &User::from_encrypted_str(&user.encrypt(&key).unwrap(), &key).unwrap()
        )
        .unwrap(),
        serde_json::to_string(&user).unwrap()
    );
}

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = AuthRedirect;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let key = Extension::<RsaPrivateKey>::from_request(req).await.unwrap();
        let cookies = TypedHeader::<headers::Cookie>::from_request(req)
            .await
            .map_err(|e| match *e.name() {
                header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {}", e),
                },
                _ => panic!("unexpected error getting cookies: {}", e),
            })?;

        let session_data = cookies.get(COOKIE_NAME).ok_or(AuthRedirect)?;
        User::from_encrypted_str(session_data, &key.0).map_err(|_| AuthRedirect)
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub code: String,
    pub state: String,
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        // TODO: redirect the user to a general purpose sign in page so they can choose what to login with
        Redirect::temporary(GITHUB.parse().unwrap()).into_response()
    }
}
