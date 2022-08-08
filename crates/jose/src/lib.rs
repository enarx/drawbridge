// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![forbid(unsafe_code, clippy::expect_used, clippy::panic)]
#![deny(
    clippy::all,
    absolute_paths_not_starting_with_crate,
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    noop_method_call,
    rust_2018_compatibility,
    rust_2018_idioms,
    rust_2021_compatibility,
    single_use_lifetimes,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub,
    unstable_features,
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]

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
