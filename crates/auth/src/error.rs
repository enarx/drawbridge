// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

#[derive(Debug)]
pub enum Error {
    Request(reqwest::Error),
    Serde(String),
    OAuth(String),
    TokenDecode(base64::DecodeError),
    TokenEncrypt(rsa::errors::Error),
    TokenDecrypt(rsa::errors::Error),
    TokenSerialize(bincode::Error),
    TokenDeserialize(bincode::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Error::Request(e) => format!("reqwest error: {}", e),
                Error::Serde(e) => format!("Serde error: {}", e),
                Error::OAuth(e) => format!("OAuth error: {}", e),
                Error::TokenDecode(e) => format!("Token decode error: {}", e),
                Error::TokenEncrypt(e) => format!("Token encryption error: {}", e),
                Error::TokenDecrypt(e) => format!("Token decryption error: {}", e),
                Error::TokenSerialize(e) => format!("Token serialization error: {}", e),
                Error::TokenDeserialize(e) => format!("Token deserialization error: {}", e),
            }
        )
    }
}

impl std::error::Error for Error {}
