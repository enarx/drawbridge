// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Algorithm, Reader, Writer};

use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

/// A set of hashing algorithms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Algorithms(BTreeSet<Algorithm>);

impl Default for Algorithms {
    fn default() -> Self {
        let mut set = BTreeSet::new();
        set.insert(Algorithm::Sha224);
        set.insert(Algorithm::Sha256);
        set.insert(Algorithm::Sha384);
        set.insert(Algorithm::Sha512);
        Self(set)
    }
}

impl From<BTreeSet<Algorithm>> for Algorithms {
    fn from(value: BTreeSet<Algorithm>) -> Self {
        Self(value)
    }
}

impl Deref for Algorithms {
    type Target = BTreeSet<Algorithm>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Algorithms {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Algorithms {
    /// Creates a reader instance
    pub fn reader<T>(&self, reader: T) -> Reader<T> {
        Reader::new(reader, self.iter().cloned())
    }

    /// Creates a writer instance
    pub fn writer<T>(&self, writer: T) -> Writer<T> {
        Writer::new(writer, self.iter().cloned())
    }
}
