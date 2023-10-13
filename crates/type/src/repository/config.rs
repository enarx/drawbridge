// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// A repository config
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[allow(missing_copy_implementations)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub public: bool,
}
