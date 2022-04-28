// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{repos, tags, trees};

use std::fmt::Display;
use std::sync::Arc;

use drawbridge_store::{Get, Memory};
use drawbridge_type::{repository, tag, tree};

use axum::body::Body;
use axum::handler::Handler;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::IntoMakeService;
use axum::{Extension, Router};
use tokio::sync::RwLock;
use tower::Service;

pub(crate) type RepoStore = RwLock<Memory<repository::Name>>;
pub(crate) type TagStore = RwLock<Memory<(repository::Name, tag::Name)>>;
pub(crate) type TreeStore = RwLock<Memory<(repository::Name, tag::Name, tree::Path)>>;

pub(crate) async fn assert_repo(
    repos: Arc<RepoStore>,
    repo: repository::Name,
) -> Result<(), (StatusCode, &'static str)> {
    #[inline]
    fn err_map(e: impl Display) -> (StatusCode, &'static str) {
        eprintln!("failed to check repository existence: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to check repository existence",
        )
    }
    if !repos
        .read()
        .await
        .contains(repo.clone())
        .await
        .map_err(err_map)?
    {
        Err((StatusCode::NOT_FOUND, "Repository does not exist"))
    } else {
        Ok(())
    }
}

pub(crate) async fn assert_tag(
    tags: Arc<TagStore>,
    repo: repository::Name,
    tag: tag::Name,
) -> Result<(), (StatusCode, &'static str)> {
    #[inline]
    fn err_map(e: impl Display) -> (StatusCode, &'static str) {
        eprintln!("failed to check tag existence: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to check tag existence",
        )
    }
    if !tags
        .read()
        .await
        .contains((repo.clone(), tag.clone()))
        .await
        .map_err(err_map)?
    {
        Err((StatusCode::NOT_FOUND, "Tag does not exist"))
    } else {
        Ok(())
    }
}

/// Parses the URI of `req` and routes it to respective component.
async fn handle(mut req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().strip_prefix('/').expect("invalid URI");
    let (head, tail) = path
        .split_once("/_")
        .map(|(left, right)| (left, format!("_{}", right)))
        .unwrap_or((path, "".into()));
    if head.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            format!("Route `/{}` not found", path),
        ));
    }
    let repo = head.parse::<repository::Name>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to parse repository name: {}", e),
        )
    })?;
    assert_eq!(
        req.extensions_mut().insert(repo),
        None,
        "duplicate repository name"
    );

    let mut tail = tail.split('/');
    match (tail.next(), tail.next(), tail.next()) {
        (None | Some(""), None, None) => match *req.method() {
            Method::HEAD => Ok(repos::head.into_service().call(req).await.into_response()),
            Method::GET => Ok(repos::get.into_service().call(req).await.into_response()),
            Method::PUT => Ok(repos::put.into_service().call(req).await.into_response()),
            _ => Err((
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed for repository endpoint".into(),
            )),
        },
        (Some("_tag"), None, None) => match *req.method() {
            Method::GET => Ok(tags::query.into_service().call(req).await.into_response()),
            _ => Err((
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed for repository tag query endpoint".into(),
            )),
        },
        (Some("_tag"), Some(tag), prop @ (None | Some("tree"))) => {
            let tag = tag.parse::<tag::Name>().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse tag name: {}", e),
                )
            })?;
            assert_eq!(req.extensions_mut().insert(tag), None, "duplicate tag name");

            if prop.is_none() {
                return match *req.method() {
                    Method::HEAD => Ok(tags::head.into_service().call(req).await.into_response()),
                    Method::GET => Ok(tags::get.into_service().call(req).await.into_response()),
                    Method::PUT => Ok(tags::put.into_service().call(req).await.into_response()),
                    _ => Err((
                        StatusCode::METHOD_NOT_ALLOWED,
                        "Method not allowed for tag endpoint".into(),
                    )),
                };
            }

            let path = tail.as_str().parse::<tree::Path>().map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Failed to parse tree path: {}", e),
                )
            })?;
            assert_eq!(
                req.extensions_mut().insert(path),
                None,
                "duplicate tree path"
            );
            match *req.method() {
                Method::HEAD => Ok(trees::head.into_service().call(req).await.into_response()),
                Method::GET => Ok(trees::get.into_service().call(req).await.into_response()),
                Method::PUT => Ok(trees::put.into_service().call(req).await.into_response()),
                _ => Err((
                    StatusCode::METHOD_NOT_ALLOWED,
                    "Method not allowed for tag tree endpoint".into(),
                )),
            }
        }
        _ => Err((
            StatusCode::NOT_FOUND,
            "Route not found on repository".into(),
        )),
    }
}

#[derive(Default)]
pub struct Builder;

impl Builder {
    pub fn new() -> Self {
        Self
    }

    // TODO: Add configuration functionality

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> IntoMakeService<Router> {
        let repos: Arc<RepoStore> = Default::default();
        let tags: Arc<TagStore> = Default::default();
        let trees: Arc<TreeStore> = Default::default();
        Router::new()
            .fallback(handle.into_service())
            .layer(Extension(repos))
            .layer(Extension(tags))
            .layer(Extension(trees))
            .into_make_service()
    }
}
