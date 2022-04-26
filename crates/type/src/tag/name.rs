// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

#[cfg(feature = "axum")]
use axum::{
    extract::{FromRequest, RequestParts},
    http::{StatusCode, Uri},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name(String);

impl FromStr for Name {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-' | '.' ))
                .is_some()
        {
            Err("Invalid tag name")
        } else {
            Ok(Name(s.into()))
        }
    }
}

impl From<Name> for String {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B> FromRequest<B> for Name
where
    B: Send,
{
    type Rejection = (StatusCode, <Self as FromStr>::Err);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let uri = req.uri_mut();
        let path = uri.path().strip_prefix('/').expect("invalid URI");
        let (name, rest) = path.split_once('/').unwrap_or((path, ""));
        let name = name.parse().map_err(|e| (StatusCode::BAD_REQUEST, e))?;

        let mut parts = uri.clone().into_parts();
        parts.path_and_query = Some(format!("/{}", rest).parse().unwrap());
        *uri = Uri::from_parts(parts).unwrap();
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "axum")]
    #[tokio::test]
    async fn from_request() {
        use super::*;

        use axum::http::Request;

        fn new_request(path: impl AsRef<str>) -> RequestParts<()> {
            RequestParts::new(Request::builder().uri(path.as_ref()).body(()).unwrap())
        }

        for path in ["/", "//", "/\\/", "//test", "/=/", "/?"] {
            assert!(
                Name::from_request(&mut new_request(path)).await.is_err(),
                "path '{}' should fail",
                path
            );
        }

        for (path, expected, rest) in [
            ("/1.2.3/", "1.2.3", "/"),
            ("/v1.2.3/foo/bar", "v1.2.3", "/foo/bar"),
            ("/v1.2.3-rc1", "v1.2.3-rc1", "/"),
            ("/test", "test", "/"),
        ] {
            let mut req = new_request(path);
            assert_eq!(
                Name::from_request(&mut req).await.unwrap(),
                Name(expected.into()),
                "path '{}' should pass",
                path
            );
            assert_eq!(req.uri().path(), rest);
        }
    }
}
