// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod name;

pub use name::*;

use std::sync::Arc;

use drawbridge_jose::jws::Jws;
use drawbridge_store::{
    Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError, Keys,
};
use drawbridge_type::{Entry, Meta, RequestMeta, Tag};

use axum::body::{Body, StreamBody};
use axum::extract::RequestParts;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use axum::{routing::*, Json};
use futures::TryStream;
use tokio::sync::RwLock;
use tower::Service;

struct App;

impl App {
    async fn query<S>(s: Arc<RwLock<S>>) -> impl IntoResponse
    where
        S: Keys<String> + 'static,
    {
        s.read()
            .await
            .keys()
            .await
            .map_err(|e| {
                eprintln!("Failed to query tags: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "")
            })
            .map(StreamBody::new)
    }

    async fn head<S>(s: Arc<RwLock<S>>, name: Name) -> impl IntoResponse
    where
        S: Sync + Get<String>,
    {
        s.read()
            .await
            .get_meta(name.0)
            .await
            .map_err(|e| match e {
                GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
                GetError::Internal(e) => {
                    eprintln!("Failed to get tag: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|meta| (meta, ()))
    }

    async fn get<S>(s: Arc<RwLock<S>>, name: Name) -> impl IntoResponse
    where
        S: Sync + Get<String> + 'static,
    {
        let s = s.read().await;

        // TODO: Stream body https://github.com/profianinc/drawbridge/issues/56
        let mut body = vec![];
        let meta = s
            .get_to_writer(name.0, &mut body)
            .await
            .map_err(|e| match e {
                GetToWriterError::Get(GetError::NotFound) => {
                    (StatusCode::NOT_FOUND, "Tag does not exist")
                }
                GetToWriterError::Get(GetError::Internal(e)) => {
                    eprintln!("Failed to get tag: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                GetToWriterError::IO(e) => {
                    eprintln!("Failed to read tag contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        name: Name,
        RequestMeta { hash, size, mime }: RequestMeta,
        req: Request<Body>,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<String> + 'static,
    {
        let mut req = RequestParts::new(req);
        let tag = match mime.to_string().as_str() {
            Entry::TYPE => req.extract().await.map(|Json(v)| Tag::Unsigned(v)),
            Jws::TYPE => req.extract().await.map(|Json(v)| Tag::Signed(v)),
            _ => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid content type".into_response(),
                ))
            }
        }
        .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;

        let buf = serde_json::to_vec(&tag).unwrap();
        if let Some(size) = size {
            if buf.len() as u64 != size {
                return Err((
                    StatusCode::BAD_REQUEST,
                    (
                        Meta {
                            hash: Default::default(), // TODO: compute
                            size: buf.len() as _,
                            mime,
                        },
                        buf,
                    )
                        .into_response(),
                ));
            }
        }
        s.write()
            .await
            .create_from_reader(name.0, mime.clone(), hash.verifier(buf.as_slice()))
            .await
            .map_err(|e| match e {
                CreateFromReaderError::IO(e) => {
                    eprintln!("Failed to stream tag contents: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Storage backend failure".into_response(),
                    )
                }
                CreateFromReaderError::Create(CreateError::Occupied) => {
                    (StatusCode::CONFLICT, "Tag already exists".into_response())
                }
                CreateFromReaderError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create tag: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Storage backend failure".into_response(),
                    )
                }
            })
            .map(|(size, hash)| Json(Meta { hash, size, mime }))
    }
}

pub fn app<S>(s: Arc<RwLock<S>>) -> Router
where
    S: Sync + Send + Get<String> + Create<String> + Keys<String> + 'static,
    S::Stream: TryStream<Ok = String>,
{
    Router::new()
        .route(
            "/",
            get({
                let s = s.clone();
                move || App::query(s)
            }),
        )
        .route(
            "/:name",
            head({
                let s = s.clone();
                move |name| App::head(s, name)
            }),
        )
        .route(
            "/:name",
            get({
                let s = s.clone();
                move |name| App::get(s, name)
            }),
        )
        .route(
            "/:name",
            put({
                let s = s.clone();
                move |name, meta, req| App::put(s, name, meta, req)
            }),
        )
}

pub struct TagExists<S, I> {
    pub tags: Arc<RwLock<S>>,
    pub inner: I,
}

impl<S, I> Clone for TagExists<S, I>
where
    I: Clone,
{
    fn clone(&self) -> Self {
        Self {
            tags: self.tags.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<R, S, I> Service<R> for TagExists<S, I>
where
    I: Service<R>,
{
    type Response = I::Response;
    type Error = I::Error;
    type Future = I::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: R) -> Self::Future {
        // TODO: Check existence of a tag before call
        // https://github.com/profianinc/drawbridge/issues/72
        let _ = self.tags;
        self.inner.call(req)
    }
}
