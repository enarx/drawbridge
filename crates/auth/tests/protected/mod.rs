// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{test_app, PROTECTED};

use drawbridge_auth::{Provider, Session, COOKIE_NAME};

use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use hyper::Body;
use oauth2::AccessToken;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use tower::util::ServiceExt;

/// This is just an example of how to implement endpoints behind OAuth.
pub async fn protected(session: Session) -> impl IntoResponse {
    format!(
        "Welcome to the protected area\nHere's your info:\n{:?}",
        session
    )
}

#[tokio::test]
#[cfg_attr(not(has_github_token), ignore)]
async fn protected_authenticated() {
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../../rsa2048-priv.der")).unwrap();
    let session = Session::new(
        Provider::GitHub,
        Some(AccessToken::new(std::env::var("GITHUB_TOKEN").unwrap())),
    );
    let app = test_app("localhost/auth".to_owned());
    let response = app
        .oneshot(
            Request::builder()
                .uri(PROTECTED)
                .header(
                    "Cookie",
                    format!("{}={}", COOKIE_NAME, session.encrypt(&key).unwrap()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = std::str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        r#"Welcome to the protected area
Here's your info:
Session { provider: GitHub, token: AccessToken([redacted]) }"#
    );
}

#[tokio::test]
async fn protected_invalid_token() {
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../../rsa2048-priv.der")).unwrap();
    let session = Session::new(
        Provider::GitHub,
        Some(AccessToken::new("BAD TOKEN".to_owned())),
    );
    let app = test_app("localhost/auth".to_owned());
    let response = app
        .oneshot(
            Request::builder()
                .uri(PROTECTED)
                .header(
                    "Cookie",
                    format!("{}={}", COOKIE_NAME, session.encrypt(&key).unwrap()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
        "localhost/auth/github"
    );
}

#[tokio::test]
async fn protected_no_token() {
    let app = test_app("localhost/auth".to_owned());
    let response = app
        .oneshot(
            Request::builder()
                .uri(PROTECTED)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
        response
            .headers()
            .get("Location")
            .unwrap()
            .to_str()
            .unwrap(),
        "localhost/auth/github"
    );
}
