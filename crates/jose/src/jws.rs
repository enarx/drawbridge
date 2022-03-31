// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::b64::{Bytes, Json};

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Jws<T = Bytes, P = BTreeMap<String, Value>, H = P>
where
    Json<P>: for<'a> Deserialize<'a>,
{
    General(General<T, P, H>),
    Flattened(Flattened<T, P, H>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct General<T = Bytes, P = BTreeMap<String, Value>, H = P>
where
    Json<P>: for<'a> Deserialize<'a>,
{
    pub payload: T,
    pub signatures: Vec<Signature<P, H>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flattened<T = Bytes, P = BTreeMap<String, Value>, H = P>
where
    Json<P>: for<'a> Deserialize<'a>,
{
    pub payload: T,

    #[serde(flatten)]
    pub signature: Signature<P, H>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature<P = BTreeMap<String, Value>, H = P>
where
    Json<P>: for<'a> Deserialize<'a>,
{
    pub protected: Option<Json<P>>,
    pub header: Option<H>,
    pub signature: Bytes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Header {
        kid: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Protected {
        alg: String,
    }

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
            header: Some(Header {
                kid: "2010-12-29".to_string(),
            }),
            protected: Some(Json(Protected {
                alg: "RS256".to_string(),
            })),
            signature: signature0.parse().unwrap(),
        };

        let sig1 = Signature {
            header: Some(Header {
                kid: "e9bc097a-ce51-4036-9562-d2ade882db0d".to_string(),
            }),
            protected: Some(Json(Protected {
                alg: "ES256".to_string(),
            })),
            signature: signature1.parse().unwrap(),
        };

        let exp = Jws::<Bytes, Protected, Header>::General(General {
            payload: payload.parse().unwrap(),
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

        let exp = Jws::<Bytes, Protected, Header>::Flattened(Flattened {
            payload: payload.parse().unwrap(),
            signature: Signature {
                header: Some(Header {
                    kid: "e9bc097a-ce51-4036-9562-d2ade882db0d".to_string(),
                }),
                protected: Some(Json(Protected {
                    alg: "ES256".to_string(),
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

        let exp = Jws::<Option<()>, Protected, Header>::Flattened(Flattened {
            payload: None,
            signature: Signature {
                header: Some(Header {
                    kid: "e9bc097a-ce51-4036-9562-d2ade882db0d".to_string(),
                }),
                protected: Some(Json(Protected {
                    alg: "ES256".to_string(),
                })),
                signature: signature.parse().unwrap(),
            },
        });

        assert_eq!(exp, serde_json::from_value(raw).unwrap());
    }
}
