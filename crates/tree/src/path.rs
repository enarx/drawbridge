use super::Node;

use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_core::http::{Request, StatusCode};
use drawbridge_core::{async_trait, FromRequest};

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

#[derive(Debug)]
pub enum Error {
    Empty,
    Node(<Node as FromStr>::Err),
}

impl From<<Node as FromStr>::Err> for Error {
    fn from(err: <Node as FromStr>::Err) -> Self {
        Self::Node(err)
    }
}

impl FromStr for Path {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hashes = s
            .trim_matches('/')
            .split('/')
            .map(|s| s.parse())
            .collect::<Result<Vec<_>, _>>()?;

        if hashes.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Self(hashes))
    }
}

#[async_trait]
impl FromRequest for Path {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.url().path().parse().map_err(|_| StatusCode::BadRequest)
    }
}
