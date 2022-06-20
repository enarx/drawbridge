// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::providers::{github, Provider};
use super::redirect::{AuthRedirect, AuthRedirectRoot};

use std::fmt;

use axum::async_trait;
use axum::extract::rejection::TypedHeaderRejectionReason;
use axum::extract::{Extension, FromRequest, RequestParts, TypedHeader};
use axum::headers;
use axum::http::header::COOKIE;
use oauth2::AccessToken;
use rand::rngs::OsRng;
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey};
use serde::{Deserialize, Serialize};

pub const COOKIE_NAME: &str = "SESSION";

#[derive(Debug)]
pub enum EncryptError {
    Encrypt(rsa::errors::Error),
    Serialize(bincode::Error),
}

impl fmt::Display for EncryptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "session {}",
            match self {
                EncryptError::Encrypt(e) => format!("encryption error: {}", e),
                EncryptError::Serialize(e) => format!("serialization error: {}", e),
            }
        )
    }
}

impl std::error::Error for EncryptError {}

#[derive(Debug)]
pub enum DecryptError {
    Decode(base64::DecodeError),
    Decrypt(rsa::errors::Error),
    Deserialize(bincode::Error),
}

impl fmt::Display for DecryptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "session {}",
            match self {
                DecryptError::Decode(e) => format!("decode error: {}", e),
                DecryptError::Decrypt(e) => format!("decryption error: {}", e),
                DecryptError::Deserialize(e) => format!("deserialization error: {}", e),
            }
        )
    }
}

impl std::error::Error for DecryptError {}

// TODO: consider protections against potential replay attacks: https://github.com/profianinc/drawbridge/issues/112
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub provider: Provider,
    pub token: AccessToken,
    pub username: Option<String>,
    pub user_id: Option<String>,
}

impl Session {
    /// Create a new session instance.
    pub fn new(provider: Provider, token: AccessToken) -> Self {
        Self {
            provider,
            token,
            username: None,
            user_id: None,
        }
    }

    pub fn set_user_info(mut self, username: String, user_id: String) -> Self {
        self.username = Some(username);
        self.user_id = Some(user_id);
        self
    }

    /// Encrypt the session so that it can be securely stored by the user.
    pub fn encrypt(&self, key: &RsaPrivateKey) -> Result<String, EncryptError> {
        let bytes = bincode::serialize(self).map_err(EncryptError::Serialize)?;
        key.encrypt(
            &mut OsRng,
            PaddingScheme::new_pkcs1v15_encrypt(),
            &bytes[..],
        )
        .map_err(EncryptError::Encrypt)
        .map(base64::encode)
    }

    /// Decrypt an untrusted user token to guarantee it was not modified by the user.
    pub fn decrypt(string: &str, key: &RsaPrivateKey) -> Result<Self, DecryptError> {
        let bytes = base64::decode(string).map_err(DecryptError::Decode)?;
        let bytes = key
            .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &bytes)
            .map_err(DecryptError::Decrypt)?;
        bincode::deserialize(&bytes).map_err(DecryptError::Deserialize)
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(Session via {}, username: {:?}, user_id: {:?})",
            self.provider, self.username, self.user_id
        )
    }
}

#[async_trait]
impl<B> FromRequest<B> for Session
where
    B: Send,
{
    type Rejection = AuthRedirect;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let redirect = Extension::<AuthRedirectRoot>::from_request(req)
            .await
            .unwrap();
        let key = Extension::<RsaPrivateKey>::from_request(req).await.unwrap();
        let cookies = TypedHeader::<headers::Cookie>::from_request(req)
            .await
            .map_err(|e| match *e.name() {
                COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => redirect.no_error(),
                    TypedHeaderRejectionReason::Error(e) => {
                        redirect.error(format!("Failed to parse HTTP headers: {}", e))
                    }
                    _ => redirect.no_error(),
                },
                _ => redirect.no_error(),
            })?;

        let session_data = cookies
            .get(COOKIE_NAME)
            .ok_or_else(|| redirect.no_error())?;
        let session = Session::decrypt(session_data, &key.0).map_err(|_| redirect.no_error())?;

        match session.provider {
            Provider::GitHub => github::validate(&session)
                .await
                .map(|github| session.set_user_info(github.username, github.id.to_string()))
                .map_err(|_| redirect.no_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::providers::Provider;
    use super::Session;

    use oauth2::AccessToken;
    use rsa::RsaPrivateKey;

    #[test]
    fn session_display() {
        assert_eq!(
            format!(
                "{}",
                Session::new(Provider::GitHub, AccessToken::new("some_token".to_owned()),)
                    .set_user_info("some_user".to_string(), "123".to_string())
            ),
            "(Session via GitHub.com, username: Some(\"some_user\"), user_id: Some(\"123\"))"
        );
    }

    #[test]
    fn session_encrypt_decrypt() {
        use rsa::pkcs8::DecodePrivateKey;

        let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../rsa2048-priv.der")).unwrap();
        let session = Session::new(Provider::GitHub, AccessToken::new("some_token".to_owned()));

        assert_eq!(
            serde_json::to_string(
                &Session::decrypt(&session.encrypt(&key).unwrap(), &key).unwrap()
            )
            .unwrap(),
            serde_json::to_string(&session).unwrap()
        );
    }
}
