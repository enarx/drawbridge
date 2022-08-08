// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![forbid(clippy::expect_used, clippy::panic)]
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
    unsafe_code,
    unstable_features,
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_results,
    variant_size_differences
)]

pub mod digest;
pub mod repository;
pub mod tag;
pub mod tree;
pub mod user;

mod meta;

pub use meta::*;
pub use repository::{
    Config as RepositoryConfig, Context as RepositoryContext, Name as RepositoryName,
};
pub use tag::{Context as TagContext, Entry as TagEntry, Name as TagName};
pub use tree::{
    Content as TreeContent, Context as TreeContext, Directory as TreeDirectory, Entry as TreeEntry,
    Name as TreeName, Path as TreePath, Tree,
};
pub use user::{Context as UserContext, Name as UserName, Record as UserRecord};
