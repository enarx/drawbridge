use super::Node;

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{self, Error, Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

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
impl FromRequest for Path {
    async fn from_request(req: &mut Request) -> http::Result<Self> {
        req.url().path().parse().map_err(|e| {
            if let Some(e) = e {
                Error::from_str(
                    StatusCode::BadRequest,
                    format!("Could not parse tree path: {}", e),
                )
            } else {
                Error::from_str(StatusCode::BadRequest, "Tree path cannot be empty")
            }
        })
    }
}
