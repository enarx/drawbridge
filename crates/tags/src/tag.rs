// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{self, Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};
use drawbridge_jose::jws::Jws;
use drawbridge_type::Entry;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Name(String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    pub body: Vec<u8>,
    pub kind: Kind,
    pub name: Name,
}

impl Value {
    pub const TYPE_ENTRY: &'static str = "application/vnd.drawbridge.tag.v1+json";
    pub const TYPE_JOSE: &'static str = "application/jose+json";
}

// value of the HashMap, name is the Key
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Kind {
    Signed(Jws),
    Unsigned(Entry),
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

impl FromStr for Name {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Name(s.to_string()))
    }
}

#[async_trait]
impl FromRequest for Name {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        // TODO: properly get the tag name from the url
        // if not a fully-qualified URL, the tests will fail
        let tag_name = match req.url().as_str().split("/").last() {
            None => {
                return Err(http_types::Error::from_str(
                    StatusCode::BadRequest,
                    "invalid URL",
                ))
            }
            Some(x) => x,
        };
        Ok(Name(tag_name.into()))
    }
}

impl Deref for Value {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl DerefMut for Value {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

#[async_trait]
impl FromRequest for Value {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        let body = req.body_string().await.unwrap();
        Ok(Value {
            body: body.clone().into_bytes(),
            kind: serde_json::from_str(&body).unwrap(),
            name: Name::from_request(req).await.unwrap(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use drawbridge_http::http::{Method, Request, Url};
    use drawbridge_http::FromRequest;
    use drawbridge_jose::b64::Json;
    use drawbridge_jose::jws::{Flattened, General, Jws, Signature};
    use drawbridge_type::Entry;
    use serde_json::json;

    const PAYLOAD: &str = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
    const SIGNATURE: &str =
        "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";
    const PROTECTED: &str = "eyJhbGciOiJFUzI1NiJ9";
    const ALGORITHM: &str = "sha-256";
    const HASH: &str = "4REjxQ4yrqUVicfSKYNO/cF9zNj5ANbzgDZt3/h3Qxo=";

    async fn test_from_request(body: serde_json::Value, kind: Kind) {
        let mut req = Request::new(
            Method::Put,
            Url::parse("http://example.com/_tag/TestTag").unwrap(),
        );
        req.set_content_type(
            match kind {
                Kind::Signed(_) => Value::TYPE_JOSE,
                Kind::Unsigned(_) => Value::TYPE_ENTRY,
            }
            .into(),
        );
        req.set_body(body.clone());
        assert_eq!(
            Value::from_request(&mut req).await.unwrap(),
            Value {
                name: Name("TestTag".into()),
                body: body.to_string().into(),
                kind,
            }
        );
    }

    #[async_std::test]
    async fn tag_value_naked_from_request() {
        test_from_request(
            json!({
                "digest": {
                    ALGORITHM: HASH
                }
            }),
            Kind::Unsigned(Entry {
                digest: format!("{}=:{}:", ALGORITHM, HASH).parse().unwrap(),
            }),
        )
        .await;
    }

    #[async_std::test]
    async fn tag_value_flattened_from_request() {
        let mut header = BTreeMap::new();
        header.insert("kid".into(), "e9bc097a-ce51-4036-9562-d2ade882db0d".into());

        let mut protected = BTreeMap::new();
        protected.insert("alg".into(), "ES256".into());

        test_from_request(
            json!({
                "payload": PAYLOAD,
                "header": { "kid": "e9bc097a-ce51-4036-9562-d2ade882db0d" },
                "protected": PROTECTED,
                "signature": SIGNATURE,
            }),
            Kind::Signed(Jws::Flattened(Flattened {
                payload: PAYLOAD.parse().unwrap(),
                signature: Signature {
                    header: Some(header),
                    protected: Some(Json(protected)),
                    signature: SIGNATURE.parse().unwrap(),
                },
            })),
        )
        .await;
    }

    #[async_std::test]
    async fn tag_value_protected_from_request() {
        let mut header = BTreeMap::new();
        header.insert("alg".into(), "ES256".into());

        test_from_request(
            json!({
                "payload": PAYLOAD,
                "protected": PROTECTED,
                "signature": SIGNATURE,
            }),
            Kind::Signed(Jws::Flattened(Flattened {
                payload: PAYLOAD.parse().unwrap(),
                signature: Signature {
                    protected: Some(Json(header)),
                    header: None,
                    signature: SIGNATURE.parse().unwrap(),
                },
            })),
        )
        .await;
    }

    #[async_std::test]
    async fn tag_value_signatures_from_request() {
        let mut header = BTreeMap::new();
        header.insert("alg".into(), "ES256".into());

        test_from_request(
            json!({
                "payload": PAYLOAD,
                "signatures": [
                    {
                        "protected": PROTECTED,
                        "signature": SIGNATURE,
                    }
                ]
            }),
            Kind::Signed(Jws::General(General {
                payload: PAYLOAD.parse().unwrap(),
                signatures: vec![Signature {
                    protected: Some(Json(header)),
                    header: None,
                    signature: SIGNATURE.parse().unwrap(),
                }],
            })),
        )
        .await;
    }
}
