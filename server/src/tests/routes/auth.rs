use drawbridge_common::{
    endpoint::{GITHUB, GITHUB_AUTHORIZED, STATUS},
    COOKIE_NAME,
};
use http::{Request, StatusCode};
use hyper::Body;
use oauth2::AccessToken;
use regex::Regex;
use rsa::{pkcs8::FromPrivateKey, RsaPrivateKey};
use tower::util::ServiceExt;

use std::{env, str};

use crate::{
    app,
    types::{AuthType, User},
};

#[tokio::test]
async fn status_authenticated() {
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../../../rsa2048-priv.der")).unwrap();
    let user = User {
        auth_type: AuthType::GitHub,
        token: AccessToken::new(env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN env var")),
    };
    let app = app("localhost");
    let response = app
        .oneshot(
            Request::builder()
                .uri(STATUS)
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
    let body = std::str::from_utf8(&body).unwrap();
    let message = Regex::new(r#"Logged in as .+ via GitHub.com."#).unwrap();
    assert!(message.is_match(body));
}

#[tokio::test]
async fn status_bad_token() {
    let key = RsaPrivateKey::from_pkcs8_der(include_bytes!("../../../rsa2048-priv.der")).unwrap();
    let user = User {
        auth_type: AuthType::GitHub,
        token: AccessToken::new("BAD TOKEN".to_owned()),
    };
    let app = app("localhost");
    let response = app
        .oneshot(
            Request::builder()
                .uri(STATUS)
                .header(
                    "Cookie",
                    format!("{}={}", COOKIE_NAME, user.encrypt(&key).unwrap()),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        "Invalid session: GitHub request failed: Bad credentials"
    );
}

#[tokio::test]
async fn status_unauthenticated() {
    let app = app("localhost");
    let response = app
        .oneshot(Request::builder().uri(STATUS).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = str::from_utf8(&body).unwrap();
    assert_eq!(body, "Not logged in.");
}

#[tokio::test]
async fn github_login() {
    let app = app("localhost");
    let response = app
        .oneshot(Request::builder().uri(GITHUB).body(Body::empty()).unwrap())
        .await
        .unwrap();

    let url = Regex::new(
        r#"https://github\.com/login/oauth/authorize\?response_type=code&response_type=code&client_id=02b39cba05d94850948f&state=(.+)&redirect_uri=http%3A%2F%2Flocalhost%2Fauth%2Fgithub%2Fauthorized&scope=identify"#,
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
async fn github_login_authorized() {
    // TODO: write a successful test for this endpoint
    let app = app("localhost");
    let response = app
        .oneshot(
            Request::builder()
                .uri(GITHUB_AUTHORIZED)
                .header("Cookie", format!("SESSION={}", "bad_session"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = str::from_utf8(&body).unwrap();
    assert_eq!(
        body,
        "Failed to deserialize query string. Expected something of type `drawbridge::types::AuthRequest`. Error: missing field `code`"
    );
}
