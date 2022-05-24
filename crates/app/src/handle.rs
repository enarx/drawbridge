// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{repos, tags, trees};

use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::body::Body;
use axum::handler::Handler;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use tower::Service;

/// Parses the URI of `req` and routes it to respective component.
pub async fn handle(mut req: Request<Body>) -> impl IntoResponse {
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
    let repo = head.parse::<RepositoryName>().map_err(|e| {
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

    let mut tail = tail.split_terminator('/');
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
            let tag = tag.parse::<TagName>().map_err(|e| {
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

            let path = tail.as_str().parse::<TreePath>().map_err(|e| {
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
