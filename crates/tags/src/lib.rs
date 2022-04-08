// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod storage;

pub use storage::Memory;

use storage::Storage;

use std::str::FromStr;

use drawbridge_http::http::{self, Error, Method, Request, Response, StatusCode};
use drawbridge_http::{async_trait, FromRequest, Handler, IntoResponse, Json};
use drawbridge_jose::jws::Jws;
use drawbridge_type::Entry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct Service<T: Clone + Storage>(T);

impl<T: Clone + Storage> From<T> for Service<T> {
    fn from(storage: T) -> Self {
        Self(storage)
    }
}

impl<T: Clone + Storage> Service<T> {
    async fn names(&self) -> http::Result<impl IntoResponse> {
        self.0.names().await.map(Json)
    }

    async fn head(&self, name: String) -> http::Result<impl IntoResponse> {
        // TODO: Set headers
        self.0.get(name).await.map(|_| ())
    }

    async fn get(&self, name: String) -> http::Result<impl IntoResponse> {
        // TODO: Set headers
        self.0.get(name).await.map(Json)
    }

    async fn put(&self, name: String, tag: Tag) -> http::Result<impl IntoResponse> {
        self.0.put(name, tag).await
    }
}

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
impl FromRequest for Name {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        let url = req.url_mut();
        let path = url.path().strip_prefix('/').expect("invalid URI");
        let (name, path) = path.split_once('/').unwrap_or((path, ""));
        let name = name
            .parse()
            .map_err(|e| Error::from_str(StatusCode::BadRequest, e))?;

        let path = path.to_string();
        url.set_path(&format!("/{}", path));
        Ok(name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tag {
    Signed(Jws),
    Unsigned(Entry),
}

#[async_trait]
impl FromRequest for Tag {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        let content_type = req
            .content_type()
            .ok_or_else(|| Error::from_str(StatusCode::BadRequest, "Content type must be set"))?;
        match content_type.to_string().as_str() {
            Entry::TYPE => req
                .body_json()
                .await
                .map(Tag::Unsigned)
                .map_err(|e| Error::from_str(StatusCode::BadRequest, e)),
            Jws::TYPE => req
                .body_json()
                .await
                .map(Tag::Signed)
                .map_err(|e| Error::from_str(StatusCode::BadRequest, e)),
            _ => Err(Error::from_str(
                StatusCode::BadRequest,
                "Invalid content type",
            )),
        }
    }
}

#[async_trait]
impl<T: Clone + Storage> Handler<()> for Service<T> {
    type Response = Response;

    async fn handle(self, req: Request) -> http::Result<Self::Response> {
        let path = req.url().path().trim_start_matches('/');
        let meth = req.method();

        match (path, meth) {
            ("", Method::Get) => (|| self.names()).handle(req).await,
            (.., Method::Head) => (|name: Name| self.head(name.0)).handle(req).await,
            (.., Method::Get) => (|name: Name| self.get(name.0)).handle(req).await,
            (.., Method::Put) => (|name: Name, tag| self.put(name.0, tag)).handle(req).await,
            _ => Err(Error::from_str(StatusCode::MethodNotAllowed, "")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::{BTreeMap, HashMap};

    use drawbridge_http::http::{Body, Method, Request};
    use drawbridge_http::FromRequest;
    use drawbridge_jose::b64::Json;
    use drawbridge_jose::jws::{Flattened, General, Jws, Signature};
    use serde_json::json;

    #[async_std::test]
    async fn name_from_request() {
        fn new_request(path: impl AsRef<str>) -> Request {
            let mut req = Request::new(Method::Put, "https://example.com/");
            req.url_mut().set_path(path.as_ref());
            req
        }

        for path in ["/", "//", "/\\/", "//test", "/=/", "/ы", "/?"] {
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
            assert_eq!(req.url().path(), rest);
        }
    }

    #[async_std::test]
    async fn tag_from_request() {
        async fn from_request(
            content_type: Option<&str>,
            body: impl Into<Body>,
        ) -> http::Result<Tag> {
            let mut req = Request::new(Method::Put, "https://example.com/");
            if let Some(content_type) = content_type {
                req.set_content_type(content_type.into());
            }
            req.set_body(body);
            Tag::from_request(&mut req).await
        }

        assert!(from_request(None, "").await.is_err());

        const ALGORITHM: &str = "sha-256";
        const HASH: &str = "4REjxQ4yrqUVicfSKYNO/cF9zNj5ANbzgDZt3/h3Qxo=";

        assert!(from_request(Some(Entry::TYPE), "").await.is_err());
        assert!(from_request(Some(Entry::TYPE), "}{").await.is_err());
        assert!(from_request(Some(Entry::TYPE), "test").await.is_err());
        assert!(from_request(Some(Entry::TYPE), json!({})).await.is_err());

        assert!(from_request(
            Some(Entry::TYPE),
            json!({
                "foo": "bar",
            }),
        )
        .await
        .is_err());

        assert_eq!(
            from_request(
                Some(Entry::TYPE),
                json!({
                    "digest": {
                        ALGORITHM: HASH,
                    },
                }),
            )
            .await
            .unwrap(),
            Tag::Unsigned(Entry {
                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
                custom: Default::default(),
            }),
        );

        assert_eq!(
            from_request(
                Some(Entry::TYPE),
                json!({
                    "digest": {
                        ALGORITHM: HASH,
                    },
                    "foo": "bar",
                }),
            )
            .await
            .unwrap(),
            Tag::Unsigned(Entry {
                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
                custom: {
                    let mut custom = HashMap::new();
                    custom.insert("foo".into(), json!("bar"));
                    custom
                },
            }),
        );

        assert!(from_request(Some(Jws::TYPE), "").await.is_err());
        assert!(from_request(Some(Jws::TYPE), "}{").await.is_err());
        assert!(from_request(Some(Jws::TYPE), "test").await.is_err());
        assert!(from_request(Some(Jws::TYPE), json!({})).await.is_err());
        assert!(from_request(
            Some(Jws::TYPE),
            json!({
                "foo": "bar",
            }),
        )
        .await
        .is_err());

        const KID: &str = "e9bc097a-ce51-4036-9562-d2ade882db0d";
        const PAYLOAD: &str = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        const PROTECTED: &str = "eyJhbGciOiJFUzI1NiJ9";
        const SIGNATURE: &str = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

        let protected = || {
            let mut protected = BTreeMap::new();
            protected.insert("alg".into(), "ES256".into());
            Some(Json(protected))
        };

        assert_eq!(
            from_request(
                Some(Jws::TYPE),
                json!({
                    "header": {
                        "kid": KID,
                    },
                    "payload": PAYLOAD,
                    "protected": PROTECTED,
                    "signature": SIGNATURE,
                })
            )
            .await
            .unwrap(),
            Tag::Signed(Jws::Flattened(Flattened {
                payload: PAYLOAD.parse().unwrap(),
                signature: Signature {
                    header: {
                        let mut header = BTreeMap::new();
                        header.insert("kid".into(), KID.into());
                        Some(header)
                    },
                    protected: protected(),
                    signature: SIGNATURE.parse().unwrap(),
                },
            })),
        );

        assert_eq!(
            from_request(
                Some(Jws::TYPE),
                json!({
                    "payload": PAYLOAD,
                    "protected": PROTECTED,
                    "signature": SIGNATURE,
                })
            )
            .await
            .unwrap(),
            Tag::Signed(Jws::Flattened(Flattened {
                payload: PAYLOAD.parse().unwrap(),
                signature: Signature {
                    header: None,
                    protected: protected(),
                    signature: SIGNATURE.parse().unwrap(),
                },
            })),
        );

        assert_eq!(
            from_request(
                Some(Jws::TYPE),
                json!({
                    "payload": PAYLOAD,
                    "signatures": [
                        {
                            "protected": PROTECTED,
                            "signature": SIGNATURE,
                        }
                    ]
                })
            )
            .await
            .unwrap(),
            Tag::Signed(Jws::General(General {
                payload: PAYLOAD.parse().unwrap(),
                signatures: vec![Signature {
                    header: None,
                    protected: protected(),
                    signature: SIGNATURE.parse().unwrap(),
                }],
            })),
        );
    }
}
