// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::digest::ContentDigest;

use mime::Mime;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "axum")]
use axum::{
    extract::{rejection::TypedHeaderRejection, FromRequest, RequestParts, TypedHeader},
    headers::{ContentLength, ContentType},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
        let hash = TypedHeader::<ContentDigest>::from_request(req).await?.0;
        let mime = TypedHeader::<ContentType>::from_request(req).await?.0;
        let size = TypedHeader::<ContentLength>::from_request(req).await?.0;

        Ok(Meta {
            hash,
            mime: mime.into(),
            size: size.0,
        })
    }
}

impl IntoIterator for Meta {
    type IntoIter = std::array::IntoIter<(&'static str, String), 3>;
    type Item = (&'static str, String);

    fn into_iter(self) -> Self::IntoIter {
        [
            ("Content-Length", self.size.to_string()),
            ("Content-Digest", self.hash.to_string()),
            ("Content-Type", self.mime.to_string()),
        ]
        .into_iter()
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
