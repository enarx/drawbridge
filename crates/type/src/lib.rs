// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

pub mod digest;

mod meta;

pub use self::meta::Meta;

use digest::ContentDigest;

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// A directory
///
/// A directory is simply a sorted name to `Entry` map.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Directory(BTreeMap<String, Entry>);

impl Directory {
    pub const TYPE: &'static str = "application/vnd.drawbridge.directory.v1+json";
}

impl Deref for Directory {
    type Target = BTreeMap<String, Entry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Directory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A directory entry
///
/// Note that this type is designed to be extensible. Therefore, the fields
/// here represent the minimum required fields. Other fields may be present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The hash of this entry
    pub digest: ContentDigest,
}

impl Entry {
    #[allow(dead_code)]
    pub const TYPE: &'static str = "application/vnd.drawbridge.entry.v1+json";
}
