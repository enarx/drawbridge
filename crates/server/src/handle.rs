// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{repos, tags, trees, users};

use drawbridge_type::{RepositoryName, TagName, TreeName, TreePath, UserName};

use std::str::FromStr;

use axum::body::Body;
use axum::handler::Handler;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use lazy_static::lazy_static;
use log::trace;
use tower::Service;

lazy_static! {
    static ref API_VERSION: semver::Version = env!("CARGO_PKG_VERSION").parse().expect(&format!(
        "failed to parse CARGO_PKG_VERSION `{}`",
        env!("CARGO_PKG_VERSION")
    ));
}

/// Parses the URI of `req` and routes it to respective component.
pub async fn handle(mut req: Request<Body>) -> impl IntoResponse {
    #[inline]
    fn not_found(path: &str) -> (StatusCode, String) {
        (StatusCode::NOT_FOUND, format!("Route `/{path}` not found"))
    }

    trace!(target: "app::handle", "begin HTTP request handling {:?}", req);
    let path = req.uri().path().trim_matches('/');
    let (ver, path) = path
        .strip_prefix("api")
        .ok_or_else(|| not_found(path))?
        .trim_start_matches('/')
        .strip_prefix('v')
        .ok_or_else(|| not_found(path))?
        .split_once('/')
        .ok_or_else(|| not_found(path))?;
    let ver = ver.parse::<semver::Version>().map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to parse SemVer version from {path}: {e}"),
        )
    })?;
    if ver > *API_VERSION {
        return Err((
            StatusCode::NOT_IMPLEMENTED,
            format!("Unsupported API version `{ver}`"),
        ));
    }
    let (head, tail) = path
        .trim_start_matches('/')
        .split_once("/_")
        .map(|(left, right)| (left.trim_end_matches('/').to_string(), format!("_{right}")))
        .unwrap_or((path.to_string(), "".to_string()));
    if head.is_empty() {
        return Err(not_found(path));
    }

    let extensions = req.extensions_mut();

    let (user, head) = head
        .split_once('/')
        .unwrap_or((head.trim_start_matches('/'), ""));
    let user = user
        .trim_end_matches('/')
        .parse::<UserName>()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse user name: {e}"),
            )
        })?;
    trace!(target: "app::handle", "parsed user name: `{user}`");
    assert_eq!(extensions.insert(user), None, "duplicate user name");
    let head = head.trim_start_matches('/');
    if head.is_empty() {
        return match *req.method() {
            Method::HEAD => Ok(users::head.into_service().call(req).await.into_response()),
            Method::GET => Ok(users::get.into_service().call(req).await.into_response()),
            Method::PUT => Ok(users::put.into_service().call(req).await.into_response()),
            _ => Err((
                StatusCode::METHOD_NOT_ALLOWED,
                "Method not allowed for user endpoint".into(),
            )),
        };
    }

    let repo = head
        .trim_end_matches('/')
        .parse::<RepositoryName>()
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Failed to parse repository name: {e}"),
            )
        })?;
    trace!(target: "app::handle", "parsed repository name: `{repo}`");
    assert_eq!(extensions.insert(repo), None, "duplicate repository name");

    let mut tail = tail.split('/').filter(|x| !x.is_empty());

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
            let tag = tag
                .trim_start_matches('/')
                .trim_end_matches('/')
                .parse::<TagName>()
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to parse tag name: {e}"),
                    )
                })?;
            trace!(target: "app::handle", "parsed tag name: `{tag}`");
            assert_eq!(extensions.insert(tag), None, "duplicate tag name");

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

            let path = tail
                .map(TreeName::from_str)
                .collect::<Result<TreePath, _>>()
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Failed to parse tree path: {e}"),
                    )
                })?;
            trace!(target: "app::handle", "parsed tree path: `{path}`");
            assert_eq!(extensions.insert(path), None, "duplicate tree path");
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

#[async_std::test]
async fn multiple_slashes_missing() {
    let request = Request::builder()
        .uri("https://www.rust-lang.org/")
        .header("User-Agent", "my-awesome-agent/1.0")
        .body(hyper::Body::empty());
    println!("{:?}", request);

    let res = handle(request.unwrap()).await;

    // Temporary print to ensure test is working as intended.
    // println!("{}", res.into_response().status());
    assert_eq!(res.into_response().status(), 404);

    let request = Request::builder()
        .uri("https://www.rust-lang.org///")
        .header("User-Agent", "my-awesome-agent///1.0")
        .body(hyper::Body::empty());
    println!("{:?}", request);

    let res = handle(request.unwrap()).await;

    // Temporary print to ensure test is working as intended.
    // println!("{}", res.into_response().status());
    assert_eq!(res.into_response().status(), 404);
    return ();
}

/// Unit test to handle multiple slash path parsing
#[async_std::test]
async fn multiple_slashes_found() {
    let request = Request::builder()
        .uri("http://httpbin.org/ip")
        .body(hyper::Body::empty());
    println!("{:?}", request);

    let res = handle(request.unwrap()).await;

    // Temporary print to ensure test is working as intended.
    // println!("{}", res.into_response().status());
    assert_eq!(res.into_response().status(), 404);

    let request = Request::builder()
        .uri("http://httpbin.org///ip")
        .body(hyper::Body::empty());
    println!("{:?}", request);

    let res = handle(request.unwrap()).await;

    // Temporary print to ensure test is working as intended.
    // println!("{}", res.into_response().status());
    assert_eq!(res.into_response().status(), 404);
    return ();
}
