// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0
mod error;
mod providers;
mod redirect;
mod session;

use axum::{extract::Extension, routing::get, Router};
use providers::github;

pub use providers::Provider;
pub use session::{Session, COOKIE_NAME};

pub fn app(host: &str, client_id: String, client_secret: String) -> Router {
    Router::new()
        .route(github::AUTHORIZED_URI, get(github::routes::authorized))
        .route(github::LOGIN_URI, get(github::routes::login))
        .layer(Extension(github::OAuthClient::new(
            host,
            client_id,
            client_secret,
        )))
}
