// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Node;

use drawbridge_http::http::{Mime, Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Meta {
    #[serde(rename = "contentLength")]
    pub size: u64,

    #[serde(deserialize_with = "deserialize")]
    #[serde(serialize_with = "serialize")]
    #[serde(rename = "contentType")]
    pub mime: Mime,

    #[serde(rename = "eTag")]
    pub hash: Node,
}

fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Mime, D::Error> where {
    let err = D::Error::custom("invalid mime type");
    let mime = String::deserialize(deserializer)?;
    let mime = mime.parse().map_err(|_| err)?;
    Ok(mime)
}

fn serialize<S: Serializer>(mime: &Mime, serializer: S) -> Result<S::Ok, S::Error> {
    mime.to_string().serialize(serializer)
}

#[async_trait]
impl FromRequest for Meta {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let size = req.header("Content-Length").ok_or(StatusCode::BadRequest)?;
        let size = size.as_str().parse().or(Err(StatusCode::BadRequest))?;

        let hash = Node::from_request(req).await?;
        let mime = Mime::from_request(req).await?;

        Ok(Meta { hash, mime, size })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn serialization() {
        const HASH: &str =
            "sha384:mqVuAfXRKap7bdgcCY5uykM6-R9GqQ8K_uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC";
        const SIZE: u64 = 42;
        const MIME: &str = "text/plain";

        assert_eq!(
            serde_json::to_string(&Meta {
                hash: HASH.parse().unwrap(),
                size: SIZE,
                mime: MIME.parse().unwrap(),
            })
            .unwrap(),
            json!({
                "contentLength": SIZE,
                "contentType": MIME,
                "eTag": HASH,
            })
            .to_string(),
        )
    }
}
