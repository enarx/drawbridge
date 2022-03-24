// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod hash;
mod storage;
mod tag;

pub use storage::Memory;

use drawbridge_http::http::{Method, Request, Response, StatusCode};
use drawbridge_http::{async_trait, Handler, IntoResponse, Json};

use self::hash::Hash;
use self::storage::Storage;
use self::tag::Tag;

#[derive(Clone, Default)]
pub struct Service<T: Clone + Storage>(T);

impl<T: Clone + Storage> From<T> for Service<T> {
    fn from(storage: T) -> Self {
        Self(storage)
    }
}

impl<T: Clone + Storage> Service<T> {
    async fn tags(&self) -> Result<impl IntoResponse, T::Error> {
        Ok(Json(self.0.tags().await?))
    }

    async fn delete(&self, tag: Tag) -> Result<impl IntoResponse, T::Error> {
        self.0.del(tag).await
    }

    async fn head(&self, tag: Tag) -> Result<impl IntoResponse, T::Error> {
        self.0.get(tag).await.map(|_| ())
    }

    async fn get(&self, tag: Tag) -> Result<impl IntoResponse, T::Error> {
        // TODO: Perform a GET
        Ok(())
    }

    async fn put(&self, tag: Tag, hash: Hash) -> Result<impl IntoResponse, T::Error> {
        self.0.put(tag, hash).await
    }
}

#[async_trait]
impl<T: Clone + Storage> Handler<()> for Service<T> {
    type Response = Response;

    async fn handle(self, req: Request) -> Self::Response {
        let path = req.url().path().trim_start_matches('/');
        let meth = req.method();

        match (path, meth) {
            ("", Method::Get) => (|| self.tags()).handle(req).await,

            (.., Method::Delete) => (|t| self.delete(t)).handle(req).await,
            (.., Method::Head) => (|t| self.head(t)).handle(req).await,
            (.., Method::Get) => (|t| self.get(t)).handle(req).await,
            (.., Method::Put) => (|t, h| self.put(t, h)).handle(req).await,

            _ => StatusCode::MethodNotAllowed.into(),
        }
    }
}
