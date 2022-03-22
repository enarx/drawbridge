// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{res::BodyType, Appender, FromRequest};

use async_trait::async_trait;
use http_types::{Request, Response, StatusCode};
use serde::{de::DeserializeOwned, Serialize};

pub struct Json<T>(pub T);

#[async_trait]
impl<'de, T: DeserializeOwned> FromRequest for Json<T> {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let err = StatusCode::BadRequest;
        let buf = req.take_body().into_bytes().await.or(Err(err))?;
        let x = serde_json::from_slice::<T>(&buf);
        let y: T = x.or(Err(err))?;
        Ok(Json(y))
    }
}

impl<T: Serialize> Appender<Json<T>, BodyType> for Response {
    fn append(mut self, item: Json<T>) -> Result<Self, Self> {
        serde_json::to_vec(&item.0)
            .map_err(|_| StatusCode::InternalServerError.into())
            .map(|buf| {
                self.append_header("Content-Length", buf.len().to_string());
                self.append_header("Content-Type", "application/json");
                self.set_body(buf);
                self
            })
    }
}
