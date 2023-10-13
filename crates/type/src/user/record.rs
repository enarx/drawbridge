// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// A user record
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// OpenID Connect identity subject uniquely identifying the user
    pub subject: String,
}
