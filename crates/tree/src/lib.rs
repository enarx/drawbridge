// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod node;
mod path;
mod storage;

pub use storage::Memory;

use node::Node;

use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;

pub fn app<S>(_: &mut Arc<RwLock<S>>) -> Router
where
    S: std::any::Any,
{
    use axum::routing::*;

    Router::new()
    // TODO: Add routes
    //.route("/*path", put(|p, m, b| self::put(&s, p, m, b)))
    //.route("/*path", head(|p| self::head(&s, p)))
    //.route("/*path", get(|p| self::get(&s, p)))
}
