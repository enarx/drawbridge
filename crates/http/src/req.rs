// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::res::IntoResponse;

use std::{fmt::Display, future::Future, str::FromStr};

use async_trait::async_trait;
use http_types::{Body, Error, Mime, Request, Response, Result, StatusCode};

#[async_trait]
pub trait FromRequest: Sized {
    async fn from_request(req: &mut Request) -> Result<Self>;
}

#[async_trait]
impl FromRequest for Body {
    async fn from_request(req: &mut Request) -> Result<Self> {
        Ok(req.take_body())
    }
}

pub fn parse_header<T>(req: &Request, name: &str) -> Result<T>
where
    T: FromStr,
    T::Err: Display,
{
    req.header(name)
        .ok_or_else(|| {
            Error::from_str(StatusCode::BadRequest, format!("`{}` header not set", name))
        })?
        .as_str()
        .parse()
        .map_err(|e| {
            Error::from_str(
                StatusCode::BadRequest,
                format!("Could not parse `{}` header value: {}", name, e),
            )
        })
}

#[async_trait]
impl FromRequest for Mime {
    async fn from_request(req: &mut Request) -> Result<Self> {
        parse_header(req, "Content-Type")
    }
}

macro_rules! mkfr {
    ($($arg:ident),*) => {
        #[async_trait]
        impl<$($arg),*> FromRequest for ($($arg,)*)
        where
            $($arg: Send + FromRequest),*
        {
            #[allow(non_snake_case)]
            async fn from_request(_req: &mut Request) -> Result<Self> {
                $(
                    let $arg = $arg::from_request(_req).await?;
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

    async fn handle(self, req: Request) -> Result<Self::Response>;
}

macro_rules! mkh {
    ($($arg:ident),*) => {
        #[async_trait]

        impl<T, F, O, $($arg),*> Handler<($($arg,)*)> for T
        where
            T: Send + FnOnce($($arg),*) -> F,
            F: Send + Future<Output = Result<O>>,
            O: Send + IntoResponse,
            $($arg: Send + FromRequest),*
        {
            type Response = Response;

            #[allow(non_snake_case)]
            async fn handle(self, mut _req: Request) -> Result<Self::Response> {
                $(
                    let $arg = $arg::from_request(&mut _req).await?;
                )*

                let res = self($($arg,)*).await?;
                Ok(res.into_response().await)
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
