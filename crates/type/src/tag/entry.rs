// SPDX-License-Identifier: Apache-2.0

use super::super::TreeEntry;

use drawbridge_jose::jws::Jws;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Entry<E = TreeEntry> {
    Signed(Box<Jws>),
    Unsigned(E),
}
