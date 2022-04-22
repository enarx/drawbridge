use axum::async_trait;
use axum::body::HttpBody;
use axum::extract::{FromRequest, RequestParts};
use axum::http::StatusCode;

use std::fmt::Display;
use std::str::FromStr;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Path(Vec<String>);

impl FromStr for Path {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn valid(part: impl AsRef<str>) -> bool {
            let part = part.as_ref();
            !part.is_empty()
                && part
                    .find(|c| !matches!(c, '0'..='9' | 'a'..='z' | 'A'..='Z' | '-'))
                    .is_none()
        }

        let path = s.split_terminator('/').map(Into::into).collect::<Vec<_>>();
        if !path.iter().all(valid) {
            Err("Invalid path")
        } else {
            Ok(Self(path))
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for Path
where
    B: Send + HttpBody,
    B::Error: Sync + Send + std::error::Error + 'static,
    B::Data: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        req.uri()
            .path()
            .strip_prefix('/')
            .expect("invalid URI")
            .parse()
            .map_err(|e| (StatusCode::BAD_REQUEST, e))
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}
