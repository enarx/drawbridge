// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod storage;
mod tag;

pub use storage::Memory;

use drawbridge_http::http::{Error, Method, Request, Response, Result, StatusCode};
use drawbridge_http::{async_trait, Handler, IntoResponse, Json};

use self::storage::Storage;
use self::tag::{Name, Value};

#[derive(Clone, Default)]
pub struct Service<T: Clone + Storage>(T);

impl<T: Clone + Storage> From<T> for Service<T> {
    fn from(storage: T) -> Self {
        Self(storage)
    }
}

impl<T: Clone + Storage> Service<T> {
    async fn tags(&self) -> Result<impl IntoResponse> {
        self.0.tags().await.map(Json)
    }

    async fn delete(&self, tag: Name) -> Result<impl IntoResponse> {
        self.0.del(tag).await
    }

    async fn head(&self, tag: Name) -> Result<impl IntoResponse> {
        self.0.get(tag).await.map(|_| ())
    }

    async fn get(&self, tag: Name) -> Result<impl IntoResponse> {
        self.0.get(tag).await.map(Json)
    }

    async fn put(&self, tag: Name, data: Value) -> Result<impl IntoResponse> {
        self.0.put(tag, data).await
    }
}

#[async_trait]
impl<T: Clone + Storage> Handler<()> for Service<T> {
    type Response = Response;

    async fn handle(self, req: Request) -> Result<Self::Response> {
        let path = req.url().path().trim_start_matches('/');
        let meth = req.method();

        match (path, meth) {
            ("", Method::Get) => (|| self.tags()).handle(req).await,

            (.., Method::Delete) => (|t| self.delete(t)).handle(req).await,
            (.., Method::Head) => (|t| self.head(t)).handle(req).await,
            (.., Method::Get) => (|t| self.get(t)).handle(req).await,
            (.., Method::Put) => (|t, h| self.put(t, h)).handle(req).await,

            _ => Err(Error::from_str(StatusCode::MethodNotAllowed, "")),
        }
    }
}
