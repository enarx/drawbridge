// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};
use drawbridge_tree::storage::Entry;

use serde::Deserialize;

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Tag {
    body: Vec<u8>,
    kind: Kind,
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
// value of the HashMap, name is the Key
pub enum Kind {
    Unsigned(Entry),
    Signed(Signature),
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub enum JsonWebSignature {
    General {
        payload: Entry, // encoded as a base64 string
        signatures: Vec<Signature>,
    },

    Flattened {
        payload: Entry, // base64 encoded

        #[serde(flatten)]
        signature: Signature,
    },
}

// This type is extensible.
// We only validate the `cty` field, which MUST be the entry mime type.
#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Headers {
    cty: String, // Mime type of the contents
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct Signature {
    protected: Option<Headers>, // encoded as a base64 string
    header: Option<Headers>,    // not base64 encoded
    signature: String,
}

impl Tag {
    pub const TYPE_ENTRY: &'static str = "application/vnd.drawbridge.tag.v1+json";
    pub const TYPE_JOSE: &'static str = "application/jose+json";
}

impl Deref for Tag {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl DerefMut for Tag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

impl FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Tag {
            body: s.as_bytes().to_vec(),
            kind: serde_json::from_str(s).unwrap(),
        })
    }
}

#[async_trait]
impl FromRequest for Tag {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.url().path()[1..]
            .parse()
            .or(Err(StatusCode::BadRequest))
    }
}

#[cfg(test)]
mod test {
    use crate::tag::{Kind, Tag};
    use std::str::FromStr;

    const NAKED_ENTRY: &'static str =
        "{\"hash\":\"sha-256=:4REjxQ4yrqUVicfSKYNO/cF9zNj5ANbzgDZt3/h3Qxo=:\"}";

    const FLATTENED_ENTRY: &'static str = "{
        \"payload\": \"e+KAnGhhc2jigJ06InNoYS0yNTY9OjRSRWp4UTR5cnFVVmljZlNLWU5PL2NGOXpOajVBTmJ6Z0RadDMvaDNReG89OiJ9\",
        \"header\": {\"cty\":\"application/vnd.drawbridge.entry.v1+json\"},
        \"signature\": \"ltGzhlEG0f7cyNJVjoiVECE6RJINln5rmrLmMqyAOno=\"\
    }";

    const FLATTENED_PROTECTED: &'static str = "{
      \"payload\": \"e+KAnGhhc2jigJ06InNoYS0yNTY9OjRSRWp4UTR5cnFVVmljZlNLWU5PL2NGOXpOajVBTmJ6Z0RadDMvaDNReG89OiJ9\",
      \"protected\": \"eyJjdHkiOiJhcHBsaWNhdGlvbi92bmQuZHJhd2JyaWRnZS5lbnRyeS52MStqc29uIn0K\",
      \"signature\": \"ltGzhlEG0f7cyNJVjoiVECE6RJINln5rmrLmMqyAOno=\"
    }";

    fn test_deserialization() {
        let naked_kind = Tag::from_str(NAKED_ENTRY).unwrap();
    }
}
