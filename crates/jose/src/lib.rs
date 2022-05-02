// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code, clippy::expect_used, clippy::panic)]

pub mod b64;
pub mod jwk;
pub mod jws;

use b64::Bytes;
use serde::{Deserialize, Serialize};

pub trait MediaTyped {
    const TYPE: &'static str;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thumbprint {
    #[serde(skip_serializing_if = "Option::is_none", default, rename = "x5t")]
    s1: Option<Bytes>,

    #[serde(skip_serializing_if = "Option::is_none", default, rename = "x5t#S256")]
    s256: Option<Bytes>,
}
