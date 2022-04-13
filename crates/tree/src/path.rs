use axum::async_trait;
use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use axum::http::StatusCode;

use super::Node;

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

#[derive(Clone)]
pub struct Path(Vec<Node>);

impl Deref for Path {
    type Target = Vec<Node>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Path {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for Path {
    type Err = Option<drawbridge_hash::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hashes = s
            .trim_matches('/')
            .split('/')
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()?;

        if hashes.is_empty() {
            return Err(None);
        }

        Ok(Self(hashes))
    }
}

#[async_trait]
impl FromRequest<Body> for Path {
    type Rejection = (StatusCode, &'static str);

    async fn from_request(_: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        todo!()
    }
}
