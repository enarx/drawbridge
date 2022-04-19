// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use drawbridge_jose::jws::Jws;
use drawbridge_type::Entry;

use axum::body::HttpBody;
use axum::extract::{FromRequest, RequestParts};
use axum::headers::ContentType;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, Json, TypedHeader};
use serde::{Deserialize, Serialize};

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
    async fn from_request() {
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
