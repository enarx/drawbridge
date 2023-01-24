// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Entry, Name};

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// A directory
///
/// A directory is simply a sorted name to `E` map.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Directory<E = Entry>(BTreeMap<Name, E>);

impl<E> Directory<E> {
    pub const TYPE: &'static str = "application/vnd.drawbridge.directory.v1+json";
}

impl<E> IntoIterator for Directory<E> {
    type Item = (Name, E);
    type IntoIter = std::collections::btree_map::IntoIter<Name, E>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<E> From<BTreeMap<Name, E>> for Directory<E> {
    fn from(m: BTreeMap<Name, E>) -> Self {
        Self(m)
    }
}

impl<E> FromIterator<(Name, E)> for Directory<E> {
    fn from_iter<T: IntoIterator<Item = (Name, E)>>(iter: T) -> Self {
        Self(BTreeMap::from_iter(iter))
    }
}

impl<E> Deref for Directory<E> {
    type Target = BTreeMap<Name, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E> DerefMut for Directory<E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
