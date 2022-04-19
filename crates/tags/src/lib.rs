// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod name;
mod tag;

pub use name::*;
pub use tag::*;

use drawbridge_store::{CreateCopyError, CreateError, GetError, Keys, Store};
use drawbridge_type::Meta;

use std::sync::Arc;

use axum::body::StreamBody;
use axum::extract::{BodyStream, Path};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use futures::io;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, TryStream, TryStreamExt};
use tokio::sync::RwLock;

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
        S: Sync + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
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
        S: Sync + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
    {
        let s = s.read().await;

        let (meta, mut br) = s.get(tag.0).await.map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get tag: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
        // TODO: Stream body
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
        S: Sync + Send + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
    {
        // TODO: Validate body as it's being read
        // TODO: Allow incomplete meta (compute length of body and digets)
        // TODO: Allow incomplete meta (compute length of body and digests)
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

pub fn app<S>(s: S) -> Router
where
    S: Sync + Send + Store<String> + Keys<String> + 'static,
    S::Stream: TryStream<Ok = String>,
    for<'a> &'a <S as Store<String>>::Read: AsyncRead,
    for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
{
    use axum::routing::*;

    let s = Arc::new(RwLock::new(s));

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
            put(move |tag, body, body_validate, meta| App::put(s, tag, body, body_validate, meta)),
        )
}
