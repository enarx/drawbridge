// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::Meta;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A directory entry
///
/// Note that this type is designed to be extensible. Therefore, the fields
/// here represent the minimum required fields. Other fields may be present.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Entry<C: ?Sized = ()> {
    /// The metadata of this entry
    #[serde(flatten)]
    pub meta: Meta,

    /// Custom fields
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,

    #[serde(skip)]
    pub content: C,
}

impl<C> Entry<C> {
    pub const TYPE: &'static str = "application/vnd.drawbridge.entry.v1+json";
}
