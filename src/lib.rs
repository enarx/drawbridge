// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![forbid(unsafe_code)]
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
    unused_import_braces,
    unused_lifetimes,
    unused_results,
    variant_size_differences
)]

pub mod bytes {
    pub use drawbridge_byte::*;
}
#[cfg(feature = "client")]
pub mod client {
    pub use drawbridge_client::*;
}
pub mod jose {
    pub use drawbridge_jose::*;
}
#[cfg(feature = "server")]
pub mod server {
    pub use drawbridge_server::*;
}
pub mod types {
    pub use drawbridge_type::*;
}
