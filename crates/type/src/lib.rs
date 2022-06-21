// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![deny(unsafe_code)]

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
