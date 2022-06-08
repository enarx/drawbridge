// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Entry;

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// A directory
///
/// A directory is simply a sorted name to `Entry` map.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Directory(BTreeMap<String, Entry>);

impl Directory {
    pub const TYPE: &'static str = "application/vnd.drawbridge.directory.v1+json";
}

impl From<BTreeMap<String, Entry>> for Directory {
    fn from(m: BTreeMap<String, Entry>) -> Self {
        Self(m)
    }
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
