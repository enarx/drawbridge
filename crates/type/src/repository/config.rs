// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// A repository config
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[allow(missing_copy_implementations)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub public: bool,
}
