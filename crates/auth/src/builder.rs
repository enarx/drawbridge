// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, providers};

use axum::routing::get;
use axum::{Extension, Router};

#[derive(Default)]
pub struct Builder {
    host: Option<String>,
    github: Option<(String, String)>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            host: None,
            github: None,
        }
    }

    /// The host the server will run on (used for redirects).
    pub fn host(mut self, host: String) -> Self {
        self.host = Some(host);
        self
    }

    /// The github client id and client secret to use.
    pub fn github(mut self, client_id: String, client_secret: String) -> Self {
        self.github = Some((client_id, client_secret));
        self
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build(self) -> Result<Router, Error> {
        let mut router = Router::new();

        let host = self.host.ok_or(Error::BuilderMissingProperty("host"))?;

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
                    &host,
                    client_id,
                    client_secret,
                )));
        }

        Ok(router)
    }
}
