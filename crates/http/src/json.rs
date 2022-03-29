// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{res::BodyType, Appender, FromRequest};

use async_trait::async_trait;
use http_types::{Error, Request, Response, Result, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

pub struct Json<T>(pub T);

#[async_trait]
impl<T: DeserializeOwned> FromRequest for Json<T> {
    async fn from_request(req: &mut Request) -> Result<Self> {
        let buf = req.take_body().into_bytes().await.map_err(|e| {
            Error::from_str(
                StatusCode::BadRequest,
                format!("Could not read request body: {}", e),
            )
        })?;
        serde_json::from_slice(&buf).map(Self).map_err(|e| {
            Error::from_str(
                StatusCode::BadRequest,
                format!("Could not parse request body: {}", e),
            )
        })
    }
}

impl<T: Serialize> Appender<Json<T>, BodyType> for Response {
    fn append(mut self, item: Json<T>) -> Result<Self> {
        serde_json::to_vec(&item.0)
            .map(|buf| {
                self.append_header("Content-Length", buf.len().to_string());
                self.append_header("Content-Type", "application/json");
                self.set_body(buf);
                self
            })
            .map_err(|_| {
                Error::from_str(StatusCode::InternalServerError, "Could not encode response")
            })
    }
}
