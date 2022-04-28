// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]
#![feature(str_split_as_str)]

mod app;
mod repos;
mod tags;
mod trees;

pub use app::*;
pub use repos::*;
pub use tags::*;
pub use trees::*;
