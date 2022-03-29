// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;

use async_trait::async_trait;
use http_types::headers::{HeaderName, ToHeaderValues};
use http_types::{Body, Response, Result, StatusCode};

pub use sealed::{BodyType, HeadType};

mod sealed {
    pub trait Type {}

    pub struct HeadType(());
    impl Type for HeadType {}

    pub struct BodyType(());
    impl Type for BodyType {}
}

pub trait Appender<I, T: sealed::Type>: Sized {
    fn append(self, item: I) -> Result<Self>;
}

impl<N: Copy, V: Copy> Appender<&[(N, V)], HeadType> for Response
where
    N: Into<HeaderName>,
    V: ToHeaderValues,
{
    fn append(mut self, item: &[(N, V)]) -> Result<Self> {
        for (n, v) in item {
            self.append_header(*n, *v);
        }

        Ok(self)
    }
}

impl<N, V, const X: usize> Appender<[(N, V); X], HeadType> for Response
where
    N: Into<HeaderName>,
    V: ToHeaderValues,
{
    fn append(mut self, item: [(N, V); X]) -> Result<Self> {
        for (n, v) in item {
            self.append_header(n, v);
        }

        Ok(self)
    }
}

impl<N, V> Appender<Vec<(N, V)>, HeadType> for Response
where
    N: Into<HeaderName>,
    V: ToHeaderValues,
{
    fn append(mut self, item: Vec<(N, V)>) -> Result<Self> {
        for (n, v) in item {
            self.append_header(n, v);
        }

        Ok(self)
    }
}

impl<N, V> Appender<HashMap<N, V>, HeadType> for Response
where
    N: Into<HeaderName>,
    V: ToHeaderValues,
{
    fn append(mut self, item: HashMap<N, V>) -> Result<Self> {
        for (n, v) in item {
            self.append_header(n, v);
        }

        Ok(self)
    }
}

impl<N, V> Appender<BTreeMap<N, V>, HeadType> for Response
where
    N: Into<HeaderName>,
    V: ToHeaderValues,
{
    fn append(mut self, item: BTreeMap<N, V>) -> Result<Self> {
        for (n, v) in item {
            self.append_header(n, v);
        }

        Ok(self)
    }
}

impl Appender<Body, BodyType> for Response {
    fn append(mut self, item: Body) -> Result<Self> {
        self.set_body(item);
        Ok(self)
    }
}

impl Appender<(), BodyType> for Response {
    fn append(self, _item: ()) -> Result<Self> {
        Ok(self)
    }
}

impl Appender<&[u8], BodyType> for Response {
    fn append(mut self, item: &[u8]) -> Result<Self> {
        self.set_body(item);
        Ok(self)
    }
}

impl<const X: usize> Appender<[u8; X], BodyType> for Response {
    fn append(mut self, item: [u8; X]) -> Result<Self> {
        self.set_body(&item[..]);
        Ok(self)
    }
}

impl Appender<Vec<u8>, BodyType> for Response {
    fn append(mut self, item: Vec<u8>) -> Result<Self> {
        self.set_body(item);
        Ok(self)
    }
}

impl Appender<&str, BodyType> for Response {
    fn append(mut self, item: &str) -> Result<Self> {
        self.set_body(item);
        Ok(self)
    }
}

impl Appender<String, BodyType> for Response {
    fn append(mut self, item: String) -> Result<Self> {
        self.set_body(item);
        Ok(self)
    }
}

#[async_trait]
pub trait IntoResponse: Send {
    async fn into_response(self) -> Response;
}

#[async_trait]
impl IntoResponse for Response {
    async fn into_response(self) -> Response {
        self
    }
}

#[async_trait]
impl IntoResponse for Infallible {
    async fn into_response(self) -> Response {
        StatusCode::Ok.into()
    }
}

#[async_trait]
impl<H: Send, B: Send> IntoResponse for (StatusCode, H, B)
where
    Response: Appender<H, HeadType>,
    Response: Appender<B, BodyType>,
{
    async fn into_response(self) -> Response {
        let response = Response::new(self.0);

        // Append body first to allow manual header overrides.
        match response.append(self.2) {
            Err(e) => return e.into_response().await,
            Ok(r) => match r.append(self.1) {
                Err(e) => e.into_response().await,
                Ok(r) => r,
            },
        }
    }
}

#[async_trait]
impl<B: Send> IntoResponse for (StatusCode, B)
where
    Response: Appender<B, BodyType>,
{
    async fn into_response(self) -> Response {
        (self.0, [("", ""); 0], self.1).into_response().await
    }
}

#[async_trait]
impl<H: Send, B: Send> IntoResponse for (H, B)
where
    Response: Appender<H, HeadType>,
    Response: Appender<B, BodyType>,
{
    async fn into_response(self) -> Response {
        (StatusCode::Ok, self.0, self.1).into_response().await
    }
}

#[async_trait]
impl IntoResponse for StatusCode {
    async fn into_response(self) -> Response {
        Response::new(self)
    }
}

#[async_trait]
impl IntoResponse for http_types::Error {
    async fn into_response(self) -> Response {
        let mut response = Response::new(self.status());
        response.set_body(self.into_inner().to_string());
        response
    }
}

#[async_trait]
impl<B: Send> IntoResponse for B
where
    Response: Appender<B, BodyType>,
{
    async fn into_response(self) -> Response {
        (StatusCode::Ok, [("", ""); 0], self).into_response().await
    }
}

#[async_trait]
impl<O: IntoResponse> IntoResponse for Result<O> {
    async fn into_response(self) -> Response {
        match self {
            Ok(x) => x.into_response().await,
            Err(x) => x.into_response().await,
        }
    }
}
