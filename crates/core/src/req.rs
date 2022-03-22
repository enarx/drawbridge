// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::{convert::Infallible, future::Future};

use async_trait::async_trait;
use http_types::{Body, Mime, Request, Response, StatusCode};

use crate::res::IntoResponse;

#[async_trait]
pub trait FromRequest: Sized {
    type Error: IntoResponse;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error>;
}

#[async_trait]
impl FromRequest for Body {
    type Error = Infallible;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        Ok(req.take_body())
    }
}

#[async_trait]
impl FromRequest for Mime {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let mime = req.header("Content-Type").ok_or(StatusCode::BadRequest)?;
        mime.as_str().parse().or(Err(StatusCode::BadRequest))
    }
}

macro_rules! mkfr {
    ($($arg:ident),*) => {
        #[async_trait]
        impl<$($arg),*> FromRequest for ($($arg,)*)
        where
            $($arg: Send + FromRequest),*
        {
            type Error = Response;

            #[allow(non_snake_case)]
            async fn from_request(_req: &mut Request) -> Result<Self, Self::Error> {
                $(
                    let $arg = match $arg::from_request(_req).await {
                        Err(e) => return Err(e.into_response().await),
                        Ok(x) => x,
                    };
                )*

                Ok(($($arg,)*))
            }
        }
    };
}

mkfr!();
mkfr!(A0);
mkfr!(A0, A1);
mkfr!(A0, A1, A2);
mkfr!(A0, A1, A2, A3);
mkfr!(A0, A1, A2, A3, A4);
mkfr!(A0, A1, A2, A3, A4, A5);
mkfr!(A0, A1, A2, A3, A4, A5, A6);
mkfr!(A0, A1, A2, A3, A4, A5, A6, A7);

#[async_trait]
pub trait Handler<T: FromRequest> {
    type Response: IntoResponse;

    async fn handle(self, req: Request) -> Self::Response;
}

macro_rules! mkh {
    ($($arg:ident),*) => {
        #[async_trait]

        impl<T, F, O, $($arg),*> Handler<($($arg,)*)> for T
        where
            T: Send + FnOnce($($arg),*) -> F,
            F: Send + Future<Output = O>,
            O: Send + IntoResponse,
            $($arg: Send + FromRequest),*
        {
            type Response = Response;

            #[allow(non_snake_case)]
            async fn handle(self, mut _req: Request) -> Self::Response {
                $(
                    let $arg = match $arg::from_request(&mut _req).await {
                        Err(e) => return e.into_response().await,
                        Ok(x) => x,
                    };
                )*

                self($($arg,)*).await.into_response().await
            }
        }
    };
}

mkh!();
mkh!(A0);
mkh!(A0, A1);
mkh!(A0, A1, A2);
mkh!(A0, A1, A2, A3);
mkh!(A0, A1, A2, A3, A4);
mkh!(A0, A1, A2, A3, A4, A5);
mkh!(A0, A1, A2, A3, A4, A5, A6);
mkh!(A0, A1, A2, A3, A4, A5, A6, A7);
