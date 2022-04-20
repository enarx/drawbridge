// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use drawbridge_store as store;
use drawbridge_tags as tag;
use drawbridge_tree as tree;

use axum::body::{Body, HttpBody};
use axum::extract::{FromRequest, RequestParts};
use axum::handler::Handler;
use axum::http::{Request, StatusCode, Uri};
use axum::{async_trait, Router};
use tokio::sync::RwLock;
use tower::Service;

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct Namespace {
    owner: String,
    groups: Vec<String>,
    name: String,
}

impl FromStr for Namespace {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-'))
                    .is_none()
        }

        let mut namespace = s.split('/').map(Into::into);
        let owner = namespace
            .next()
            .ok_or("Repository owner must be specified")?;
        let mut namespace = namespace.collect::<Vec<_>>();
        let name = namespace.pop().ok_or("Repository name must be specified")?;
        let groups = namespace;
        if !valid(&owner) || !valid(&name) || !groups.iter().all(valid) {
            Err("Invalid namespace")
        } else {
            Ok(Self {
                owner,
                groups,
                name,
            })
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for Namespace
where
    B: Send + HttpBody,
    B::Error: Sync + Send + std::error::Error + 'static,
    B::Data: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let uri = req.uri_mut();
        let path = uri.path().strip_prefix('/').expect("invalid URI");
        let (namespace, rest) = path.split_once("/_").unwrap_or((path, ""));
        let namespace = namespace
            .parse()
            .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

        let mut parts = uri.clone().into_parts();
        parts.path_and_query = Some(format!("/_{}", rest).parse().unwrap());
        *uri = Uri::from_parts(parts).unwrap();
        Ok(namespace)
    }
}

pub fn app() -> Router {
    let mut tags: HashMap<Namespace, Arc<RwLock<store::Memory<String>>>> = Default::default();
    let mut trees: HashMap<Namespace, Arc<RwLock<store::Memory<String>>>> = Default::default();
    Router::new().fallback(
        (|req: Request<Body>| async move {
            let mut parts = RequestParts::new(req);
            let namespace = parts.extract::<Namespace>().await?;
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
                        tag::app(tags.entry(namespace.clone()).or_default()),
                    )
                    .nest(
                        // TODO: Nest tree API under `/_tag/:name/`
                        // https://github.com/profianinc/drawbridge/issues/48
                        "/_tree",
                        tree::app(trees.entry(namespace.clone()).or_default()),
                    )
                    .fallback(
                        (|| async { (StatusCode::NOT_FOUND, "Route not found") }).into_service(),
                    )
                    .call(req)
                    .await,
            )
        })
        .into_service(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespace_from_str() {
        assert!("".parse::<Namespace>().is_err());
        assert!(" ".parse::<Namespace>().is_err());
        assert!("/".parse::<Namespace>().is_err());
        assert!("name".parse::<Namespace>().is_err());
        assert!("owner/".parse::<Namespace>().is_err());
        assert!("/name".parse::<Namespace>().is_err());
        assert!("owner//name".parse::<Namespace>().is_err());
        assert!("owner/name/".parse::<Namespace>().is_err());
        assert!("owner/group///name".parse::<Namespace>().is_err());
        assert!("owner/g%roup/name".parse::<Namespace>().is_err());
        assert!("owner/gяoup/name".parse::<Namespace>().is_err());
        assert!("owner /group/name".parse::<Namespace>().is_err());
        assert!("owner/gr☣up/name".parse::<Namespace>().is_err());
        assert!("o.wner/group/name".parse::<Namespace>().is_err());

        assert_eq!(
            "owner/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec![],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec!["group".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "owner/group/subgroup/name".parse(),
            Ok(Namespace {
                owner: "owner".into(),
                groups: vec!["group".into(), "subgroup".into()],
                name: "name".into(),
            })
        );
        assert_eq!(
            "0WnEr/gr0up/subgr0up/-n4mE".parse(),
            Ok(Namespace {
                owner: "0WnEr".into(),
                groups: vec!["gr0up".into(), "subgr0up".into()],
                name: "-n4mE".into(),
            })
        );
    }
}
