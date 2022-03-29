// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{self, Error, Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct Hash(drawbridge_hash::Hash);

impl Deref for Hash {
    type Target = drawbridge_hash::Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for Hash {
    type Err = drawbridge_hash::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[async_trait]
impl FromRequest for Hash {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        req.body_string()
            .await
            .map_err(|_| Error::from_str(StatusCode::BadRequest, ""))?
            .parse()
            .map_err(|_| Error::from_str(StatusCode::BadRequest, ""))
    }
}
