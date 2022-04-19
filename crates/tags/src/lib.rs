// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

use drawbridge_jose::jws::Jws;
use drawbridge_store::{CreateCopyError, CreateError, GetError, Keys, Store};
use drawbridge_type::{Entry, Meta};

use std::str::FromStr;
use std::sync::Arc;

use axum::body::{HttpBody, StreamBody};
use axum::extract::{BodyStream, FromRequest, Path, RequestParts};
use axum::headers::ContentType;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Json, Router, TypedHeader};
use futures::io;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, TryStream, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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
impl<B> FromRequest<B> for Name
where
    B: Send,
{
    type Rejection = (StatusCode, <Self as FromStr>::Err);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
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

struct App;

impl App {
    async fn query<S>(s: Arc<RwLock<S>>) -> impl IntoResponse
    where
        S: Keys<String> + 'static,
    {
        s.read()
            .await
            .keys()
            .await
            .map_err(|e| {
                eprintln!("Failed to query tags: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "")
            })
            .map(StreamBody::new)
    }

    async fn head<S>(s: Arc<RwLock<S>>, Path(tag): Path<Name>) -> impl IntoResponse
    where
        S: Sync + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
    {
        s.read()
            .await
            .get_meta(tag.0)
            .await
            .map_err(|e| match e {
                GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
                GetError::Internal(e) => {
                    eprintln!("Failed to get tag: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|meta| (meta, ()))
    }

    async fn get<S>(s: Arc<RwLock<S>>, Path(tag): Path<Name>) -> impl IntoResponse
    where
        S: Sync + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
    {
        let s = s.read().await;

        let (meta, mut br) = s.get(tag.0).await.map_err(|e| match e {
            GetError::NotFound => (StatusCode::NOT_FOUND, "Tag does not exist"),
            GetError::Internal(e) => {
                eprintln!("Failed to get tag: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
            }
        })?;
        // TODO: Stream body
        // probably there should be a way to write body within the closure
        let mut body = vec![];
        br.read_to_end(&mut body).await.map_err(|e| {
            eprintln!("Failed to read tag contents: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
        })?;
        Ok::<_, (_, _)>((meta, body))
    }

    async fn put<S>(
        s: Arc<RwLock<S>>,
        Path(tag): Path<Name>,
        body: BodyStream,
        _: Tag, // validate body contents
        meta: Meta,
    ) -> impl IntoResponse
    where
        S: Sync + Send + Store<String>,
        for<'a> &'a <S as Store<String>>::Read: AsyncRead,
        for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
    {
        // TODO: Validate body as it's being read
        // TODO: Allow incomplete meta (compute length of body and digets)
        // TODO: Allow incomplete meta (compute length of body and digests)
        let body = body.map_err(|e| io::Error::new(io::ErrorKind::Other, e));
        s.write()
            .await
            .create_copy(tag.0, meta, body.into_async_read())
            .await
            .map_err(|e| match e {
                CreateCopyError::IO(e) => {
                    eprintln!("Failed to stream tag contents: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
                CreateCopyError::Create(CreateError::Occupied) => {
                    (StatusCode::CONFLICT, "Tag already exists")
                }
                CreateCopyError::Create(CreateError::Internal(e)) => {
                    eprintln!("Failed to create tag: {}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, "Storage backend failure")
                }
            })
            .map(|_| ())
    }
}

pub fn app<S>(s: S) -> Router
where
    S: Sync + Send + Store<String> + Keys<String> + 'static,
    S::Stream: TryStream<Ok = String>,
    for<'a> &'a <S as Store<String>>::Read: AsyncRead,
    for<'a> &'a mut <S as Store<String>>::Write: AsyncWrite,
{
    use axum::routing::*;

    let s = Arc::new(RwLock::new(s));

    Router::new()
        .route(
            "/",
            get({
                let s = s.clone();
                move || App::query(s)
            }),
        )
        .route(
            "/:tag",
            head({
                let s = s.clone();
                move |tag| App::head(s, tag)
            }),
        )
        .route(
            "/:tag",
            get({
                let s = s.clone();
                move |tag| App::get(s, tag)
            }),
        )
        .route(
            "/:tag",
            put(move |tag, body, body_validate, meta| App::put(s, tag, body, body_validate, meta)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::{BTreeMap, HashMap};

    use axum::body::Body;
    use drawbridge_jose::b64::Json;
    use drawbridge_jose::jws::{Flattened, General, Jws, Signature};

    use axum::http::Request;
    use serde_json::json;

    #[tokio::test]
    async fn name_from_request() {
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

    #[tokio::test]
    async fn tag_from_request() {
        async fn from_request(
            content_type: Option<&str>,
            body: impl ToString,
        ) -> Result<Tag, <Tag as FromRequest<Body>>::Rejection> {
            let mut req = Request::builder().uri("https://example.com/").method("PUT");
            if let Some(content_type) = content_type {
                req = req.header("Content-Type", content_type)
            }
            Tag::from_request(&mut RequestParts::new(
                req.body(Body::from(body.to_string())).unwrap(),
            ))
            .await
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
