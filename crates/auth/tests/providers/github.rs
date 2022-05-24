// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::super::test_app;

use std::str;

use axum::http::{Request, StatusCode};
use hyper::Body;
use regex::Regex;
use tower::ServiceExt;

#[tokio::test]
async fn login() {
    let app = test_app("localhost/auth".to_owned());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/github")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let url = Regex::new(
        r#"https://github.com/login/oauth/authorize\?response_type=code&response_type=code&client_id=(.+)&state=(.+)&redirect_uri=http%3A%2F%2Flocalhost%2Fauth%2Fgithub%2Fauthorized&scope=identify"#
    ).unwrap();

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(url.is_match(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap()
    ));
}

#[tokio::test]
async fn authorized() {
    // TODO: write a successful test for this endpoint
    let app = test_app("localhost/auth".to_owned());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/github/authorized")
                .header("Cookie", format!("SESSION={}", "bad_session"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        "Failed to deserialize query string. Expected something of type `drawbridge_auth::providers::github::routes::AuthRequest`. Error: missing field `code`"
    );
}
