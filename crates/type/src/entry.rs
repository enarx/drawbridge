// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::digest::ContentDigest;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A directory entry
///
/// Note that this type is designed to be extensible. Therefore, the fields
/// here represent the minimum required fields. Other fields may be present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The hash of this entry
    pub digest: ContentDigest,

    /// Custom fields
    #[serde(flatten)]
    pub custom: HashMap<String, Value>,
}

impl Entry {
    pub const TYPE: &'static str = "application/vnd.drawbridge.entry.v1+json";
}
