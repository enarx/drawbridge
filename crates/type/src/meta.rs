// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::digest::ContentDigest;

use mime::Mime;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "axum")]
use axum::{
    extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason},
    extract::{FromRequest, RequestParts, TypedHeader},
    headers::{ContentLength, ContentType},
    response::{IntoResponseParts, Response, ResponseParts},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Meta {
    #[serde(rename = "digest")]
    pub hash: ContentDigest<Box<[u8]>>,

    #[serde(rename = "length")]
    pub size: u64,

    #[serde(deserialize_with = "deserialize")]
    #[serde(serialize_with = "serialize")]
    #[serde(rename = "type")]
    pub mime: Mime,
}

fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Mime, D::Error> {
    String::deserialize(deserializer)?
        .parse()
        .map_err(|_| D::Error::custom("invalid mime type"))
}

fn serialize<S: Serializer>(mime: &Mime, serializer: S) -> Result<S::Ok, S::Error> {
    mime.to_string().serialize(serializer)
}

#[cfg(feature = "axum")]
#[axum::async_trait]
impl<B: Send> FromRequest<B> for Meta {
    type Rejection = TypedHeaderRejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let hash = match req.extract().await {
            Ok(TypedHeader(hash)) => hash,
            Err(e) if matches!(e.reason(), TypedHeaderRejectionReason::Missing) => {
                Default::default()
            }
            Err(e) => return Err(e),
        };
        let size = req.extract::<TypedHeader<ContentLength>>().await?.0 .0;
        let mime = req.extract::<TypedHeader<ContentType>>().await?.0.into();
        Ok(Meta { hash, size, mime })
    }
}

#[cfg(feature = "axum")]
impl IntoResponseParts for Meta {
    type Error = Response;

    fn into_response_parts(self, res: ResponseParts) -> Result<ResponseParts, Self::Error> {
        let hash = TypedHeader(self.hash);
        let size = TypedHeader(ContentLength(self.size));
        let mime = TypedHeader(ContentType::from(self.mime));
        (hash, size, mime).into_response_parts(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn serialization() {
        let meta = Meta {
            hash: "sha-384=:mqVuAfXRKap7bdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8w=:"
                .parse()
                .unwrap(),
            size: 42,
            mime: "text/plain".parse().unwrap(),
        };

        let json = json!({
            "digest": {"sha-384": "mqVuAfXRKap7bdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8w="},
            "length": 42,
            "type": "text/plain",
        });

        assert_eq!(serde_json::to_string(&meta).unwrap(), json.to_string());
    }
}
