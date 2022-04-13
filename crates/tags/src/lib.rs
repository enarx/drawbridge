// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod storage;

pub use storage::Memory;

use drawbridge_jose::jws::Jws;
use drawbridge_type::Entry;

use std::str::FromStr;

use axum::body::{Body, HttpBody};
use axum::extract::{FromRequest, RequestParts};
use axum::headers::ContentType;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Json, Router, TypedHeader};
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

#[async_trait]
impl FromRequest<Body> for Name {
    type Rejection = (StatusCode, <Self as FromStr>::Err);

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let uri = req.uri_mut();
        let path = uri.path().strip_prefix('/').expect("invalid URI");
        let (name, path) = path.split_once('/').unwrap_or((path, ""));
        let name = name.parse().map_err(|e| (StatusCode::BAD_REQUEST, e))?;

        let path = path.to_string();
        *uri = format!("/{}", path).parse().unwrap();
        Ok(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tag {
    Signed(Jws),
    Unsigned(Entry),
}

#[async_trait]
impl<B> FromRequest<B> for Tag
where
    B: Send + HttpBody,
    B::Error: Sync + Send + std::error::Error + 'static,
    B::Data: Send,
{
    type Rejection = (StatusCode, Response);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        // TODO: Rely on `Meta`
        let TypedHeader(content_type) = req
            .extract::<TypedHeader<ContentType>>()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response()))?;
        match content_type.to_string().as_str() {
            Entry::TYPE => req
                .extract()
                .await
                .map(|Json(v)| Tag::Unsigned(v))
                .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response())),
            Jws::TYPE => req
                .extract()
                .await
                .map(|Json(v)| Tag::Signed(v))
                .map_err(|e| (StatusCode::BAD_REQUEST, e.into_response())),
            _ => Err((
                StatusCode::BAD_REQUEST,
                "Invalid content type".into_response(),
            )),
        }
    }
}

pub fn app() -> Router {
    use axum::routing::*;

    Router::new()
}

//// TODO: Reenable
//#[cfg(test)]
//mod tests {
//    use super::*;
//
//    use std::collections::{BTreeMap, HashMap};
//
//    use axum::http::{Request, Method};
//    use drawbridge_jose::b64::Json;
//    use drawbridge_jose::jws::{Flattened, General, Jws, Signature};
//    use serde_json::json;
//
//    #[tokio::test]
//    async fn name_from_request() {
//        fn new_request(path: impl AsRef<str>) -> Request {
//            let mut req = Request::new(Method::Put, "https://example.com/");
//            req.url_mut().set_path(path.as_ref());
//            req
//        }
//
//        for path in ["/", "//", "/\\/", "//test", "/=/", "/ы", "/?"] {
//            assert!(
//                Name::from_request(&mut new_request(path)).await.is_err(),
//                "path '{}' should fail",
//                path
//            );
//        }
//
//        for (path, expected, rest) in [
//            ("/1.2.3/", "1.2.3", "/"),
//            ("/v1.2.3/foo/bar", "v1.2.3", "/foo/bar"),
//            ("/v1.2.3-rc1", "v1.2.3-rc1", "/"),
//            ("/test", "test", "/"),
//        ] {
//            let mut req = new_request(path);
//            assert_eq!(
//                Name::from_request(&mut req).await.unwrap(),
//                Name(expected.into()),
//                "path '{}' should pass",
//                path
//            );
//            assert_eq!(req.url().path(), rest);
//        }
//    }
//
//    #[tokio::test]
//    async fn tag_from_request() {
//        async fn from_request(
//            content_type: Option<&str>,
//            body: impl Into<Body>,
//        ) -> Result<Tag> {
//            let mut req = Request::new(Method::Put, "https://example.com/");
//            if let Some(content_type) = content_type {
//                req.set_content_type(content_type.into());
//            }
//            req.set_body(body);
//            Tag::from_request(&mut req).await
//        }
//
//        assert!(from_request(None, "").await.is_err());
//
//        const ALGORITHM: &str = "sha-256";
//        const HASH: &str = "4REjxQ4yrqUVicfSKYNO/cF9zNj5ANbzgDZt3/h3Qxo=";
//
//        assert!(from_request(Some(Entry::TYPE), "").await.is_err());
//        assert!(from_request(Some(Entry::TYPE), "}{").await.is_err());
//        assert!(from_request(Some(Entry::TYPE), "test").await.is_err());
//        assert!(from_request(Some(Entry::TYPE), json!({})).await.is_err());
//
//        assert!(from_request(
//            Some(Entry::TYPE),
//            json!({
//                "foo": "bar",
//            }),
//        )
//        .await
//        .is_err());
//
//        assert_eq!(
//            from_request(
//                Some(Entry::TYPE),
//                json!({
//                    "digest": {
//                        ALGORITHM: HASH,
//                    },
//                }),
//            )
//            .await
//            .unwrap(),
//            Tag::Unsigned(Entry {
//                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
//                custom: Default::default(),
//            }),
//        );
//
//        assert_eq!(
//            from_request(
//                Some(Entry::TYPE),
//                json!({
//                    "digest": {
//                        ALGORITHM: HASH,
//                    },
//                    "foo": "bar",
//                }),
//            )
//            .await
//            .unwrap(),
//            Tag::Unsigned(Entry {
//                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
//                custom: {
//                    let mut custom = HashMap::new();
//                    custom.insert("foo".into(), json!("bar"));
//                    custom
//                },
//            }),
//        );
//
//        assert!(from_request(Some(Jws::TYPE), "").await.is_err());
//        assert!(from_request(Some(Jws::TYPE), "}{").await.is_err());
//        assert!(from_request(Some(Jws::TYPE), "test").await.is_err());
//        assert!(from_request(Some(Jws::TYPE), json!({})).await.is_err());
//        assert!(from_request(
//            Some(Jws::TYPE),
//            json!({
//                "foo": "bar",
//            }),
//        )
//        .await
//        .is_err());
//
//        const KID: &str = "e9bc097a-ce51-4036-9562-d2ade882db0d";
//        const PAYLOAD: &str = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
//        const PROTECTED: &str = "eyJhbGciOiJFUzI1NiJ9";
//        const SIGNATURE: &str = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";
//
//        let protected = || {
//            let mut protected = BTreeMap::new();
//            protected.insert("alg".into(), "ES256".into());
//            Some(Json(protected))
//        };
//
//        assert_eq!(
//            from_request(
//                Some(Jws::TYPE),
//                json!({
//                    "header": {
//                        "kid": KID,
//                    },
//                    "payload": PAYLOAD,
//                    "protected": PROTECTED,
//                    "signature": SIGNATURE,
//                })
//            )
//            .await
//            .unwrap(),
//            Tag::Signed(Jws::Flattened(Flattened {
//                payload: PAYLOAD.parse().unwrap(),
//                signature: Signature {
//                    header: {
//                        let mut header = BTreeMap::new();
//                        header.insert("kid".into(), KID.into());
//                        Some(header)
//                    },
//                    protected: protected(),
//                    signature: SIGNATURE.parse().unwrap(),
//                },
//            })),
//        );
//
//        assert_eq!(
//            from_request(
//                Some(Jws::TYPE),
//                json!({
//                    "payload": PAYLOAD,
//                    "protected": PROTECTED,
//                    "signature": SIGNATURE,
//                })
//            )
//            .await
//            .unwrap(),
//            Tag::Signed(Jws::Flattened(Flattened {
//                payload: PAYLOAD.parse().unwrap(),
//                signature: Signature {
//                    header: None,
//                    protected: protected(),
//                    signature: SIGNATURE.parse().unwrap(),
//                },
//            })),
//        );
//
//        assert_eq!(
//            from_request(
//                Some(Jws::TYPE),
//                json!({
//                    "payload": PAYLOAD,
//                    "signatures": [
//                        {
//                            "protected": PROTECTED,
//                            "signature": SIGNATURE,
//                        }
//                    ]
//                })
//            )
//            .await
//            .unwrap(),
//            Tag::Signed(Jws::General(General {
//                payload: PAYLOAD.parse().unwrap(),
//                signatures: vec![Signature {
//                    header: None,
//                    protected: protected(),
//                    signature: SIGNATURE.parse().unwrap(),
//                }],
//            })),
//        );
//    }
//}
