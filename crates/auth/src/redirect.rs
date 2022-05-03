// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0
use crate::providers::github;

use axum::response::{IntoResponse, Redirect, Response};

pub struct AuthRedirect {
    pub error: Option<String>,
}

impl AuthRedirect {
    pub const fn no_error() -> Self {
        AuthRedirect { error: None }
    }
}

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        // TODO: redirect the user to a general purpose sign in page so they can choose what to login with, show the user the message in the error field: https://github.com/profianinc/drawbridge/issues/50
        Redirect::temporary(github::LOGIN_URI).into_response()
    }
}
