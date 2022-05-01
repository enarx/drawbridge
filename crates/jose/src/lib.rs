// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code, clippy::expect_used, clippy::panic)]

pub mod b64;
pub mod jws;

pub trait MediaTyped {
    const TYPE: &'static str;
}
