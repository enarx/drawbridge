// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::{RepositoryName, TagName, TreePath};

use axum::http;
use http::header::HeaderMap;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub async fn create_path(
    cl: &reqwest::Client,
    addr: &str,
    repo: &RepositoryName,
    tag: &TagName,
    path: &TreePath,
    mime: Mime,
    body: Vec<u8>,
) {
    let url = format!("{addr}/{repo}/_tag/{tag}/tree{path}");

    let res = cl.head(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "{}",
        res.text().await.unwrap()
    );

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "{}",
        res.text().await.unwrap()
    );

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, mime.to_string())
        .body(body.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "{}",
        res.text().await.unwrap()
    );

    let res = cl.head(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );
    // TODO: Validate headers

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );
    assert_eq!(res.bytes().await.unwrap(), body);
    // TODO: Validate headers

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, mime.to_string())
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        // TODO: This should not result in conflict, since payload is the same.
        StatusCode::CONFLICT,
        "{}",
        res.text().await.unwrap()
    );
}

pub fn get_header<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    let mut iter = headers.get_all(name).iter();
    let (first, second) = (iter.next(), iter.next());
    assert!(first.is_some());
    assert!(second.is_none());
    first.unwrap().to_str().unwrap()
}

pub async fn get_path(
    cl: &reqwest::Client,
    addr: &str,
    repo: &RepositoryName,
    tag: &TagName,
    path: &TreePath,
) -> (Vec<u8>, Mime) {
    let url = format!("{addr}/{repo}/_tag/{tag}/tree{path}");

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let length = res.content_length().unwrap();

    let content_type: Mime = get_header(res.headers(), CONTENT_TYPE.as_str())
        .parse()
        .unwrap();

    let content_digest: ContentDigest =
        get_header(res.headers(), "content-digest").parse().unwrap();
    let resp_bytes = res.bytes().await.unwrap().to_vec();

    let calculated_digest = Algorithms::default()
        .read(resp_bytes.as_slice())
        .await
        .unwrap();

    assert_eq!(length as usize, resp_bytes.len());
    assert_eq!(content_digest, calculated_digest);
    (resp_bytes, content_type)
}
