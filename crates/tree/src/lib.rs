// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

mod crypto;
mod meta;
mod node;
mod path;
mod storage;

pub use storage::Memory;

use async_std::io::{copy, sink};
use drawbridge_core::http::{Body, Method, Request, Response, StatusCode};
use drawbridge_core::{async_trait, Handler, IntoResponse, Json};

use self::crypto::Hash;
use self::meta::Meta;
use self::node::Node;
use self::path::Path;
use self::storage::Storage;

pub const DIRECTORY: &str = "application/vnd.drawbridge.directory.v1+json";

#[derive(Clone, Default)]
pub struct Service<T: Clone + Storage>(T);

impl<T: Clone + Storage> From<T> for Service<T> {
    fn from(storage: T) -> Self {
        Self(storage)
    }
}

impl<T: Clone + Storage> Service<T> {
    async fn roots(&self) -> Result<impl IntoResponse, T::Error> {
        Ok(Json(self.0.roots().await?))
    }

    async fn options(&self, path: Path) -> Result<impl IntoResponse, T::Error> {
        Ok(Json(self.0.wants(path).await?))
    }

    async fn delete(&self, path: Path) -> Result<impl IntoResponse, T::Error> {
        self.0.del(path).await
    }

    async fn head(&self, path: Path) -> Result<impl IntoResponse, T::Error> {
        let (meta, ..) = self.0.get(path).await?;
        let head = [
            ("Content-Length", meta.size.to_string()),
            ("Content-Type", meta.mime.to_string()),
        ];

        Ok((head, ()))
    }

    async fn get(&self, path: Path) -> Result<impl IntoResponse, T::Error> {
        let (meta, data) = self.0.get(path).await?;
        let head = [
            ("Content-Length", meta.size.to_string()),
            ("Content-Type", meta.mime.to_string()),
        ];

        Ok((head, data))
    }

    async fn put(&self, path: Path, meta: Meta, body: Body) -> Result<impl IntoResponse, T::Error> {
        // Validate that the final Node is the measurement of the Meta.
        let buf = serde_json::to_vec(&meta).map_err(|_| StatusCode::InternalServerError)?;
        let mut rdr = (*path[path.len() - 1]).clone().reader(&buf[..]);
        copy(&mut rdr, &mut sink())
            .await
            .map_err(|_| StatusCode::BadRequest)?;

        // Validate the measurement of the body as it is read.
        let body = (*meta.hash).clone().reader(body);

        self.0.put(path, meta, body).await
    }
}

#[async_trait]
impl<T: Clone + Storage> Handler<()> for Service<T> {
    type Response = Response;

    async fn handle(self, req: Request) -> Self::Response {
        let path = req.url().path().trim_start_matches('/');
        let meth = req.method();

        match (path, meth) {
            ("", Method::Get) => (|| self.roots()).handle(req).await,

            (.., Method::Options) => (|p| self.options(p)).handle(req).await,
            (.., Method::Delete) => (|p| self.delete(p)).handle(req).await,
            (.., Method::Head) => (|p| self.head(p)).handle(req).await,
            (.., Method::Get) => (|p| self.get(p)).handle(req).await,
            (.., Method::Put) => (|p, m, b| self.put(p, m, b)).handle(req).await,

            _ => StatusCode::MethodNotAllowed.into(),
        }
    }
}
