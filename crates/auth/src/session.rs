// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::providers::{github, Provider};
use crate::redirect::{AuthRedirect, AuthRedirectRoot};

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

// TODO: consider protections against potential replay attacks: https://github.com/profianinc/drawbridge/issues/112
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub provider: Provider,
    pub token: AccessToken,
}

impl Session {
    /// Create a new session instance.
    pub fn new(provider: Provider, token: AccessToken) -> Self {
        Self { provider, token }
    }

    /// Decrypt an untrusted user token to guarantee it was not modified by the user.
    pub fn from_encrypted_str(string: &str, key: &RsaPrivateKey) -> Result<Self, Error> {
        let bytes = base64::decode(string).map_err(Error::TokenDecode)?;
        let bytes = key
            .decrypt(PaddingScheme::new_pkcs1v15_encrypt(), &bytes)
            .map_err(Error::TokenDecrypt)?;
        bincode::deserialize(&bytes).map_err(Error::TokenDeserialize)
    }

    /// Encrypt the session so that it can be securely stored by the user.
    pub fn encrypt(&self, key: &RsaPrivateKey) -> Result<String, Error> {
        let bytes = bincode::serialize(self).map_err(Error::TokenSerialize)?;
        key.encrypt(
            &mut OsRng,
            PaddingScheme::new_pkcs1v15_encrypt(),
            &bytes[..],
        )
        .map_err(Error::TokenEncrypt)
        .map(base64::encode)
    }
}

impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Session via {})", self.provider)
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
        let session =
            Session::from_encrypted_str(session_data, &key.0).map_err(|_| redirect.no_error())?;

        match session.provider {
            Provider::GitHub => github::validate(&session)
                .await
                .map(|_| session)
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
                Session::new(Provider::GitHub, AccessToken::new("some_token".to_owned()))
            ),
            "(Session via GitHub.com)"
        );
    }

    #[test]
    fn session_encrypt_decrypt() {
        use rsa::pkcs8::DecodePrivateKey;

        let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../rsa2048-priv.der")).unwrap();
        let session = Session::new(Provider::GitHub, AccessToken::new("some_token".to_owned()));

        assert_eq!(
            serde_json::to_string(
                &Session::from_encrypted_str(&session.encrypt(&key).unwrap(), &key).unwrap()
            )
            .unwrap(),
            serde_json::to_string(&session).unwrap()
        );
    }
}
