// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// A user record
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// OpenID Connect identity subject uniquely identifying the user
    pub subject: String,
}
