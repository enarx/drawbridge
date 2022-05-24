// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod protected;
mod providers;
mod status;

use drawbridge_auth::{AuthRedirectRoot, Builder};

use axum::extract::Extension;
use axum::routing::get;
use axum::Router;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;

pub const STATUS: &str = "/status";
pub const PROTECTED: &str = "/protected";

pub fn test_app(host: String) -> Router {
    // TODO: generate this key at runtime or pull the path from the command line args: https://github.com/profianinc/drawbridge/issues/18
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../rsa2048-priv.der")).unwrap();

    Router::new()
        .nest(
            "/auth",
            Builder::new(host.clone())
                .github("unused".to_string(), "unused".to_string())
                .build(),
        )
        .route(STATUS, get(status::status))
        .route(PROTECTED, get(protected::protected))
        .layer(Extension(key))
        .layer(Extension(AuthRedirectRoot(host)))
}
