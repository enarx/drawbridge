// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

fn main() {
    if option_env!("GITHUB_TOKEN").is_some() {
        println!("cargo:rustc-cfg=has_github_token");
    }
}
