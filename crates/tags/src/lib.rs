// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod name;
mod tag;

pub use name::*;
pub use tag::*;

use std::sync::Arc;

use drawbridge_store::{Create, CreateCopyError, CreateError, Get, GetError, Keys};
use drawbridge_type::Meta;

use axum::body::StreamBody;
use axum::extract::{BodyStream, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use futures::io;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, TryStream, TryStreamExt};
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

    async fn head<S>(s: Arc<RwLock<S>>, Path(tag): Path<Name>) -> impl IntoResponse
    where
        S: Sync + Get<String>,
        for<'a> &'a S::Item: AsyncRead,
    {
        s.read()
            .await
            .get_meta(tag.0)
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

    async fn get<S>(s: Arc<RwLock<S>>, Path(tag): Path<Name>) -> impl IntoResponse
    where
        S: Sync + Get<String>,
        for<'a> &'a S::Item: AsyncRead,
    {
        let s = s.read().await;

        let (meta, mut br) = s.get(tag.0).await.map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get tag: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
        // TODO: Stream body https://github.com/profianinc/drawbridge/issues/56
        // probably there should be a way to write body within the closure
        let mut body = vec![];
        br.read_to_end(&mut body).await.map_err(|e| {
            eprintln!("Failed to read tag contents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
        })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        Path(tag): Path<Name>,
        body: BodyStream,
        _: Tag, // validate body contents
        meta: Meta,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<String>,
        for<'a> &'a mut S::Item: AsyncWrite,
    {
        // TODO: Introduce tag validation middleware https://github.com/profianinc/drawbridge/issues/57
        // TODO: Allow incomplete meta (compute length of body and digests) https://github.com/profianinc/drawbridge/issues/55
        let body = body.map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        s.write()
            .await
            .create_copy(tag.0, meta, body.into_async_read())
            .await
            .map_err(|e| match e {
                CreateCopyError::IO(e) => {
                    eprintln!("Failed to stream tag contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                CreateCopyError::Create(CreateError::Occupied) => {
                    (StatusCode::CONFLICT, "Tag already exists")
                }
                CreateCopyError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create tag: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|_| ())
    }
}

pub fn app<S>(s: &mut Arc<RwLock<S>>) -> Router
where
    S: Sync + Send + Get<String> + Create<String> + Keys<String> + 'static,
    for<'a> &'a <S as Get<String>>::Item: AsyncRead,
    for<'a> &'a mut <S as Create<String>>::Item: AsyncWrite,
    S::Stream: TryStream<Ok = String>,
{
    use axum::routing::*;

    Router::new()
        .route(
            "/",
            get({
                let s = s.clone();
                move || App::query(s)
            }),
        )
        .route(
            "/:tag",
            head({
                let s = s.clone();
                move |tag| App::head(s, tag)
            }),
        )
        .route(
            "/:tag",
            get({
                let s = s.clone();
                move |tag| App::get(s, tag)
            }),
        )
        .route(
            "/:tag",
            put({
                let s = s.clone();
                move |tag, body, body_validate, meta| App::put(s, tag, body, body_validate, meta)
            }),
        )
}

#[derive(Clone)]
pub struct TagExists<S, I> {
    pub tags: Arc<RwLock<S>>,
    pub inner: I,
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
