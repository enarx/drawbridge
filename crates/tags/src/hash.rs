use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_hash::Error;
use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};

#[derive(Clone)]
pub struct Hash(drawbridge_hash::Hash);

impl Deref for Hash {
    type Target = drawbridge_hash::Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Hash {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for Hash {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

#[async_trait]
impl FromRequest for Hash {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.body_string()
            .await
            .or(Err(StatusCode::BadRequest))?
            .parse()
            .or(Err(StatusCode::BadRequest))
    }
}
