// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Node;

use drawbridge_web::http::{Mime, Request, StatusCode};
use drawbridge_web::{async_trait, FromRequest};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Meta {
    pub hash: Node,

    #[serde(deserialize_with = "deserialize")]
    #[serde(serialize_with = "serialize")]
    pub mime: Mime,

    pub size: u64,
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
