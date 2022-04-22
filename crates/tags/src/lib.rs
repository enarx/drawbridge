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
        // TODO: Validate node hash against parents' expected values https://github.com/profianinc/drawbridge/issues/77

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
                            hash: Default::default(), // TODO: Compute https://github.com/profianinc/drawbridge/issues/76
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
            put(move |tag, meta, req| App::put(s, tag, meta, req)),
        )
}
