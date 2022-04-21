// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod namespace;

use namespace::*;

use std::collections::HashMap;
use std::sync::Arc;

use drawbridge_store::{self as store, Create, CreateCopyError, CreateError, Get, GetError};
use drawbridge_tags as tag;
use drawbridge_tree as tree;
use drawbridge_type::Meta;

use axum::body::Body;
use axum::extract::{BodyStream, RequestParts};
use axum::handler::Handler;
use axum::http::{Request, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{any, get, head, put};
use axum::Router;
use futures::{io, AsyncRead, AsyncReadExt, AsyncWrite, TryStreamExt};
use tokio::sync::RwLock;
use tower::layer::layer_fn;
use tower::Service;

struct App;

impl App {
    async fn head<S>(s: Arc<RwLock<S>>, repo: Namespace) -> impl IntoResponse
    where
        S: Sync + Get<Namespace>,
        for<'a> &'a S::Item: AsyncRead,
    {
        s.read()
            .await
            .get_meta(repo)
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

    async fn get<S>(s: Arc<RwLock<S>>, repo: Namespace) -> impl IntoResponse
    where
        S: Sync + Get<Namespace>,
        for<'a> &'a S::Item: AsyncRead,
    {
        let s = s.read().await;

        let (meta, mut br) = s.get(repo).await.map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Repository does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get repository: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
        // TODO: Stream body https://github.com/profianinc/drawbridge/issues/56
        let mut body = vec![];
        br.read_to_end(&mut body).await.map_err(|e| {
            eprintln!("Failed to read repository contents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
        })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        repo: Namespace,
        body: BodyStream,
        meta: Meta,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Create<Namespace>,
        for<'a> &'a mut S::Item: AsyncWrite,
    {
        // TODO: Allow incomplete meta (compute length of body and digests) https://github.com/profianinc/drawbridge/issues/55
        let body = body.map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        s.write()
            .await
            .create_copy(repo, meta, body.into_async_read())
            .await
            .map_err(|e| match e {
                CreateCopyError::IO(e) => {
                    eprintln!("Failed to stream repository contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                CreateCopyError::Create(CreateError::Occupied) => {
                    (StatusCode::CONFLICT, "Repository already exists")
                }
                CreateCopyError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create repository: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|_| ())
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
    let mut trees: HashMap<Namespace, Arc<RwLock<store::Memory<String>>>> = Default::default();
    Router::new().route(
        "/*path",
        any(|req: Request<Body>| async move {
            let mut parts = RequestParts::new(req);
            let repo = parts.extract::<Namespace>().await?;
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
                        tag::app(tags.entry(repo.clone()).or_default())
                            .nest(
                                "/:tag/tree",
                                tree::app(trees.entry(repo.clone()).or_default()),
                            )
                            .route_layer(layer_fn(|inner| RepoExists {
                                repos: repos.clone(),
                                repo: repo.clone(),
                                inner,
                            })),
                    )
                    .route(
                        "/",
                        head({
                            let repos = repos.clone();
                            let repo = repo.clone();
                            move || App::head(repos, repo)
                        }),
                    )
                    .route(
                        "/",
                        get({
                            let repos = repos.clone();
                            let repo = repo.clone();
                            move || App::get(repos, repo)
                        }),
                    )
                    .route(
                        "/",
                        put({
                            let repo = repo.clone();
                            |body, meta| App::put(repos, repo, body, meta)
                        }),
                    )
                    .fallback(
                        (|uri: Uri| async move {
                            (
                                StatusCode::NOT_FOUND,
                                format!("Route {} not found for repository {}", uri, repo),
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
