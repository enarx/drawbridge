// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

pub mod repo;
pub mod tag;
pub mod tree;

use std::fmt::Debug;
use std::str::FromStr;

use axum::http::header::HeaderMap;

pub fn parse_header<'a, T>(headers: &'a HeaderMap, name: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    let mut iter = headers.get_all(name).iter();
    let (first, second) = (iter.next(), iter.next());
    assert!(first.is_some());
    assert!(second.is_none());
    first.unwrap().to_str().unwrap().parse().expect(&format!(
        "failed to parse `{}` header from `{:?}`",
        name, headers
    ))
}
