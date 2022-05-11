// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

fn main() {
    if option_env!("GITHUB_TOKEN").is_some() {
        println!("cargo:rustc-cfg=has_github_token");
    }
}
