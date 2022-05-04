// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::providers::github;

use axum::response::{self, IntoResponse, Response};

#[derive(Clone)]
pub struct AuthRedirectRoot(pub String);

impl AuthRedirectRoot {
    pub fn error(&self, error: String) -> AuthRedirect {
        AuthRedirect {
            root: self.0.clone(),
            error: Some(error),
        }
    }

    pub fn no_error(&self) -> AuthRedirect {
        AuthRedirect {
            root: self.0.clone(),
            error: None,
        }
    }
}

pub struct AuthRedirect {
    pub root: String,
    pub error: Option<String>,
}

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        // TODO: redirect the user to a general purpose sign in page so they can choose what to login with, show the user the message in the error field: https://github.com/profianinc/drawbridge/issues/50
        response::Redirect::temporary(&format!("{}{}", self.root, github::LOGIN_URI))
            .into_response()
    }
}
