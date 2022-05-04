// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::providers;

use axum::routing::get;
use axum::{Extension, Router};

#[derive(Default)]
pub struct Builder {
    host: String,
    github: Option<(String, String)>,
}

impl Builder {
    /// Constructs a new [Builder].
    /// The host is the URL the server will run on (used for redirects).
    pub fn new(host: String) -> Self {
        Self { host, github: None }
    }

    /// The github client id and client secret to use.
    pub fn github(self, client_id: String, client_secret: String) -> Self {
        Self {
            github: Some((client_id, client_secret)),
            ..self
        }
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> Router {
        let mut router = Router::new();

        if let Some((client_id, client_secret)) = self.github {
            router = router
                .route(
                    providers::github::AUTHORIZED_URI,
                    get(providers::github::routes::authorized),
                )
                .route(
                    providers::github::LOGIN_URI,
                    get(providers::github::routes::login),
                )
                .layer(Extension(providers::github::OAuthClient::new(
                    &self.host,
                    client_id,
                    client_secret,
                )));
        }
        router
    }
}
