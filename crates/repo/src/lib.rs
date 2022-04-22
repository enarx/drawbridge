// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod namespace;

use namespace::*;

use std::collections::HashMap;
use std::sync::Arc;

use drawbridge_store::{
    self as store, Create, CreateError, CreateFromReaderError, Get, GetError, GetToWriterError,
};
use drawbridge_tags::{self as tag, TagExists};
use drawbridge_tree::{self as tree, Path};
use drawbridge_type::{Meta, Repository, RequestMeta};

use axum::body::Body;
use axum::extract::RequestParts;
use axum::handler::Handler;
use axum::http::{Request, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{any, get, head, put};
use axum::{Json, Router};
use tokio::sync::RwLock;
use tower::layer::layer_fn;
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
        Json(repo): Json<Repository>,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<Namespace> + 'static,
    {
        let buf = serde_json::to_vec(&repo).unwrap();
        if let Some(size) = size {
            if buf.len() as u64 != size {
                return Err((StatusCode::BAD_REQUEST, "Content length mismatch"));
            }
        }
        s.write()
            .await
            .create_from_reader(name, mime.clone(), hash.verifier(buf.as_slice()))
            .await
            .map_err(|e| match e {
                CreateFromReaderError::IO(e) => {
                    eprintln!("Failed to stream repository contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                CreateFromReaderError::Create(CreateError::Occupied) => {
                    (StatusCode::CONFLICT, "Repository already exists")
                }
                CreateFromReaderError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create repository: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|(size, hash)| Json(Meta { hash, size, mime }))
    }
}

#[derive(Clone)]
struct RepoExists<I> {
    repos: Arc<RwLock<store::Memory<Namespace>>>,
    repo: Namespace,
    inner: I,
}

impl<R, I> Service<R> for RepoExists<I>
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
        // TODO: Check existence of a repository before call
        // https://github.com/profianinc/drawbridge/issues/67
        let _ = self.repos;
        let _ = self.repo;
        self.inner.call(req)
    }
}

pub fn app() -> Router {
    let repos: Arc<RwLock<store::Memory<Namespace>>> = Default::default();
    let mut tags: HashMap<Namespace, Arc<RwLock<store::Memory<String>>>> = Default::default();
    let mut trees: HashMap<Namespace, Arc<RwLock<store::Memory<Path>>>> = Default::default();
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
                    .nest("/_tag", {
                        let tags = tags.entry(name.clone()).or_default();
                        tag::app(Arc::clone(tags))
                            .nest("/:name/tree", {
                                tree::app(Arc::clone(trees.entry(name.clone()).or_default()))
                                    .route_layer(layer_fn(|inner| TagExists {
                                        tags: tags.clone(),
                                        inner,
                                    }))
                            })
                            .route_layer(layer_fn(|inner| RepoExists {
                                repos: Arc::clone(&repos),
                                repo: name.clone(),
                                inner,
                            }))
                    })
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
