// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Arc;

use drawbridge_store::{
    self as store, Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError,
};
use drawbridge_tags as tag;
use drawbridge_tree::{self as tree, Path};
use drawbridge_type::repository::{Config, Namespace};
use drawbridge_type::{Meta, RequestMeta};

use axum::body::Body;
use axum::extract::RequestParts;
use axum::handler::Handler;
use axum::http::{Request, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{any, get, head, put};
use axum::{Json, Router};
use tokio::sync::RwLock;
use tower::Service;

struct App;

impl App {
    async fn head<S>(s: Arc<RwLock<S>>, name: Namespace) -> impl IntoResponse
    where
        S: Sync + Get<Namespace>,
    {
        s.read()
            .await
            .get_meta(name)
            .await
            .map_err(|e| match e {
                GetError::NotFound => (StatusCode::NOT_FOUND, "Repository does not exist"),
                GetError::Internal(e) => {
                    eprintln!("Failed to get repository metadata: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|meta| (meta, ()))
    }

    async fn get<S>(s: Arc<RwLock<S>>, name: Namespace) -> impl IntoResponse
    where
        S: Sync + Get<Namespace> + 'static,
    {
        let s = s.read().await;

        // TODO: Stream body https://github.com/profianinc/drawbridge/issues/56
        let mut body = vec![];
        let meta = s
            .get_to_writer(name, &mut body)
            .await
            .map_err(|e| match e {
                GetToWriterError::Get(GetError::NotFound) => {
                    (StatusCode::NOT_FOUND, "Repository does not exist")
                }
                GetToWriterError::Get(GetError::Internal(e)) => {
                    eprintln!("Failed to get repository: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                GetToWriterError::IO(e) => {
                    eprintln!("Failed to read repository contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        name: Namespace,
        RequestMeta { hash, size, mime }: RequestMeta,
        Json(repo): Json<Config>,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<Namespace> + 'static,
    {
        let buf = serde_json::to_vec(&repo).unwrap();
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
            .create_from_reader(name.clone(), mime.clone(), hash.verifier(buf.as_slice()))
            .await
            .map_err(|e| match e {
                CreateFromReaderError::IO(e) => {
                    eprintln!("Failed to stream repository contents: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Storage backend failure".into_response(),
                    )
                }
                CreateFromReaderError::Create(CreateError::Occupied) => (
                    StatusCode::CONFLICT,
                    "Repository already exists".into_response(),
                ),
                CreateFromReaderError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create repository: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Storage backend failure".into_response(),
                    )
                }
            })
            .map(|(size, hash)| Json(Meta { hash, size, mime }))
    }
}

type Repos = store::Memory<Namespace>;
type Tags = HashMap<Namespace, Arc<RwLock<store::Memory<String>>>>;
type Trees = HashMap<(Namespace, String), Arc<RwLock<store::Memory<Path>>>>;

pub fn app() -> Router {
    let repos: Arc<RwLock<Repos>> = Default::default();
    let tags: Arc<RwLock<Tags>> = Default::default();
    let trees: Arc<RwLock<Trees>> = Default::default();
    Router::new().route(
        "/*path",
        any(|req: Request<Body>| async move {
            let mut parts = RequestParts::new(req);
            let name = parts.extract::<Namespace>().await?;
            let req = parts
                .try_into_request()
                .or(Err::<_, (StatusCode, &'static str)>((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "",
                )))?;
            Ok::<_, (_, _)>(
                Router::new()
                    .nest(
                        "/_tag",
                        any({
                            let repos = Arc::clone(&repos);
                            let name = name.clone();
                            |req: Request<Body>| async move {
                                if !repos.read().await.contains(name.clone()).await.or(Err((
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to check repository existence",
                                )))? {
                                    return Err((
                                        StatusCode::NOT_FOUND,
                                        "Repository does not exist",
                                    ));
                                }
                                Ok(tag::app(Arc::clone(
                                    // TODO: Try `read()` lock first
                                    tags.write().await.entry(name.clone()).or_default(),
                                ))
                                .nest(
                                    "/:tag/tree",
                                    any(|tag: drawbridge_tags::Name, req: Request<Body>| async move {
                                        // TODO: Check that tag exists https://github.com/profianinc/drawbridge/issues/72
                                        tree::app(Arc::clone(
                                            // TODO: Try `read()` lock first
                                            trees
                                                .write()
                                                .await
                                                .entry((name, tag.into()))
                                                .or_default(),
                                        ))
                                        .call(req)
                                        .await
                                    }),
                                )
                                .call(req)
                                .await)
                            }
                        }),
                    )
                    .route(
                        "/",
                        head({
                            let repos = Arc::clone(&repos);
                            let name = name.clone();
                            move || App::head(repos, name)
                        }),
                    )
                    .route(
                        "/",
                        get({
                            let repos = Arc::clone(&repos);
                            let name = name.clone();
                            move || App::get(repos, name)
                        }),
                    )
                    .route(
                        "/",
                        put({
                            let name = name.clone();
                            |meta, repo| App::put(repos, name, meta, repo)
                        }),
                    )
                    .fallback(
                        (|uri: Uri| async move {
                            (
                                StatusCode::NOT_FOUND,
                                format!("Route {} not found for repository {}", uri, name),
                            )
                        })
                        .into_service(),
                    )
                    .call(req)
                    .await,
            )
        }),
    )
}
