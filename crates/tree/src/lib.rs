// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use axum::body::Body;
use axum::extract::RequestParts;
use axum::http::Request;
use axum::Json;

use std::sync::Arc;

use drawbridge_store::{
    Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError,
};
use drawbridge_type::tree::{Directory, Path};
use drawbridge_type::{Meta, RequestMeta};

use axum::extract::BodyStream;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use futures::io;
use futures::TryStreamExt;
use tokio::sync::RwLock;

struct App;

impl App {
    async fn head<S>(s: Arc<RwLock<S>>, path: Path) -> impl IntoResponse
    where
        S: Sync + Get<Path>,
    {
        s.read()
            .await
            .get_meta(path)
            .await
            .map_err(|e| match e {
                GetError::NotFound => (StatusCode::NOT_FOUND, "Path does not exist"),
                GetError::Internal(e) => {
                    eprintln!("Failed to get path: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|meta| (meta, ()))
    }

    async fn get<S>(s: Arc<RwLock<S>>, path: Path) -> impl IntoResponse
    where
        S: Sync + Get<Path> + 'static,
    {
        let s = s.read().await;

        // TODO: Return a list of missing paths depending on `Accept` header https://github.com/profianinc/drawbridge/issues/29
        // TODO: Stream body https://github.com/profianinc/drawbridge/issues/56
        let mut body = vec![];
        let meta = s
            .get_to_writer(path, &mut body)
            .await
            .map_err(|e| match e {
                GetToWriterError::Get(GetError::NotFound) => {
                    (StatusCode::NOT_FOUND, "Tree does not exist")
                }
                GetToWriterError::Get(GetError::Internal(e)) => {
                    eprintln!("Failed to get tree: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                GetToWriterError::IO(e) => {
                    eprintln!("Failed to read tree contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        path: Path,
        RequestMeta { hash, size, mime }: RequestMeta,
        req: Request<Body>,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<Path> + 'static,
    {
        let mut req = RequestParts::new(req);
        match mime.to_string().as_str() {
            Directory::TYPE => {
                let dir = req
                    .extract()
                    .await
                    .map(|Json(v): Json<Directory>| v)
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;
                let buf = serde_json::to_vec(&dir).unwrap();
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
                    .create_from_reader(path, mime.clone(), hash.verifier(buf.as_slice()))
                    .await
            }
            _ => {
                let body = req
                    .extract::<BodyStream>()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e));
                // TODO: Validate body size
                s.write()
                    .await
                    .create_from_reader(path, mime.clone(), hash.verifier(body.into_async_read()))
                    .await
            }
        }
        .map_err(|e| match e {
            CreateFromReaderError::IO(e) => {
                eprintln!("Failed to stream path contents: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Storage backend failure".into_response(),
                )
            }
            CreateFromReaderError::Create(CreateError::Occupied) => {
                (StatusCode::CONFLICT, "Path already exists".into_response())
            }
            CreateFromReaderError::Create(CreateError::Internal(e)) => {
                eprintln!("Failed to create path: {}", e);
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
    S: Sync + Send + Get<Path> + Create<Path> + 'static,
{
    use axum::routing::*;

    Router::new()
        .route(
            "/*path",
            head({
                let s = s.clone();
                move |path| App::head(s, path)
            }),
        )
        .route(
            "/*path",
            get({
                let s = s.clone();
                move |path| App::get(s, path)
            }),
        )
        .route(
            "/*path",
            put(move |path, body, meta| App::put(s, path, body, meta)),
        )
}
