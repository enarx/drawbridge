use drawbridge_common::{endpoint::PROTECTED, COOKIE_NAME};
use http::{Request, StatusCode};
use hyper::Body;
use oauth2::AccessToken;
use rsa::{pkcs8::FromPrivateKey, RsaPrivateKey};
use tower::util::ServiceExt;

use std::str;

use crate::{
    app,
    types::{AuthType, User},
};

#[tokio::test]
async fn protected_authenticated() {
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../../../rsa2048-priv.der")).unwrap();
    let user = User {
        auth_type: AuthType::GitHub,
        token: AccessToken::new("BAD TOKEN".to_owned()),
    };
    let app = app("localhost");
    let response = app
        .oneshot(
            Request::builder()
                .uri(PROTECTED)
                .header(
                    "Cookie",
                    format!("{}={}", COOKIE_NAME, user.encrypt(&key).unwrap()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        r#"Welcome to the protected area
Here's your info:
User { auth_type: GitHub, token: AccessToken([redacted]) }"#
    );
}

#[tokio::test]
async fn protected_redirect() {
    let app = app("localhost");
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
        "/auth/github"
    );
}
