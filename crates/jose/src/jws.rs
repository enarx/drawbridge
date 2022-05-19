// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::b64::{Bytes, Json};
use crate::{MediaTyped, Thumbprint};

use mediatype::MediaTypeBuf;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameters {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alg: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub jku: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub jwk: Option<crate::jwk::Jwk<crate::jwk::Parameters>>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kid: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub x5u: Option<Url>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub x5c: Option<Vec<drawbridge_byte::Bytes<Vec<u8>>>>, // base64, not base64url

    #[serde(flatten)]
    pub x5t: Thumbprint,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub typ: Option<MediaTypeBuf>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cty: Option<MediaTypeBuf>,

    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub crit: Option<Vec<String>>,
}

impl MediaTyped for Jws {
    const TYPE: &'static str = "application/jose+json";
}

impl Jws {
    pub const TYPE: &'static str = "application/jose+json";
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "P: DeserializeOwned, H: Deserialize<'de>"))]
#[serde(untagged)]
pub enum Jws<P = Parameters, H = P> {
    General(General<P, H>),
    Flattened(Flattened<P, H>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "P: DeserializeOwned, H: Deserialize<'de>"))]
pub struct General<P = Parameters, H = P> {
    pub payload: Option<Bytes>,
    pub signatures: Vec<Signature<P, H>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "P: DeserializeOwned, H: Deserialize<'de>"))]
pub struct Flattened<P = Parameters, H = P> {
    pub payload: Option<Bytes>,

    #[serde(flatten)]
    pub signature: Signature<P, H>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "P: DeserializeOwned, H: Deserialize<'de>"))]
pub struct Signature<P = Parameters, H = P> {
    pub protected: Option<Json<P>>,
    pub header: Option<H>,
    pub signature: Bytes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Example from RFC 7515 A.6.4
    #[test]
    fn general() {
        let payload = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        let signature0 = "cC4hiUPoj9Eetdgtv3hF80EGrhuB__dzERat0XF9g2VtQgr9PJbu3XOiZj5RZmh7AAuHIm4Bh-0Qc_lF5YKt_O8W2Fp5jujGbds9uJdbF9CUAr7t1dnZcAcQjbKBYNX4BAynRFdiuB--f_nZLgrnbyTyWzO75vRK5h6xBArLIARNPvkSjtQBMHlb1L07Qe7K0GarZRmB_eSN9383LcOLn6_dO--xi12jzDwusC-eOkHWEsqtFZESc6BfI7noOPqvhJ1phCnvWh6IeYI2w9QOYEUipUTI8np6LbgGY9Fs98rqVt5AXLIhWkWywlVmtVrBp0igcN_IoypGlUPQGe77Rw";
        let signature1 = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

        let raw = json!({
            "payload": payload,
            "signatures": [
                {
                    "protected":"eyJhbGciOiJSUzI1NiJ9",
                    "header": { "kid": "2010-12-29" },
                    "signature": signature0,
                },
                {
                    "protected":"eyJhbGciOiJFUzI1NiJ9",
                    "header": { "kid":"e9bc097a-ce51-4036-9562-d2ade882db0d" },
                    "signature": signature1,
                }
            ]
        });

        let sig0 = Signature {
            header: Some(Parameters {
                kid: Some("2010-12-29".to_string()),
                ..Default::default()
            }),
            protected: Some(Json(Parameters {
                alg: Some("RS256".to_string()),
                ..Default::default()
            })),
            signature: signature0.parse().unwrap(),
        };

        let sig1 = Signature {
            header: Some(Parameters {
                kid: Some("e9bc097a-ce51-4036-9562-d2ade882db0d".to_string()),
                ..Default::default()
            }),
            protected: Some(Json(Parameters {
                alg: Some("ES256".to_string()),
                ..Default::default()
            })),
            signature: signature1.parse().unwrap(),
        };

        let exp = Jws::General(General {
            payload: Some(payload.parse().unwrap()),
            signatures: vec![sig0, sig1],
        });

        assert_eq!(exp, serde_json::from_value(raw).unwrap());
    }

    // Example from RFC 7515 A.7
    #[test]
    fn flattened() {
        let payload = "eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ";
        let signature = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

        let raw = json!({
            "payload": payload,
            "protected": "eyJhbGciOiJFUzI1NiJ9",
            "header": { "kid": "e9bc097a-ce51-4036-9562-d2ade882db0d" },
            "signature": signature,
        });

        let exp = Jws::Flattened(Flattened {
            payload: Some(payload.parse().unwrap()),
            signature: Signature {
                header: Some(Parameters {
                    kid: Some("e9bc097a-ce51-4036-9562-d2ade882db0d".to_string()),
                    ..Default::default()
                }),
                protected: Some(Json(Parameters {
                    alg: Some("ES256".to_string()),
                    ..Default::default()
                })),
                signature: signature.parse().unwrap(),
            },
        });

        assert_eq!(exp, serde_json::from_value(raw).unwrap());
    }

    // Example from RFC 7515 A.7
    #[test]
    fn detached() {
        let signature = "DtEhU3ljbEg8L38VWAfUAqOyKAM6-Xx-F4GawxaepmXFCgfTjDxw5djxLa8ISlSApmWQxfKTUJqPP3-Kg6NU1Q";

        let raw = json!({
            "protected": "eyJhbGciOiJFUzI1NiJ9",
            "header": { "kid": "e9bc097a-ce51-4036-9562-d2ade882db0d" },
            "signature": signature,
        });

        let exp = Jws::Flattened(Flattened {
            payload: None,
            signature: Signature {
                header: Some(Parameters {
                    kid: Some("e9bc097a-ce51-4036-9562-d2ade882db0d".to_string()),
                    ..Default::default()
                }),
                protected: Some(Json(Parameters {
                    alg: Some("ES256".to_string()),
                    ..Default::default()
                })),
                signature: signature.parse().unwrap(),
            },
        });

        assert_eq!(exp, serde_json::from_value(raw).unwrap());
    }
}
