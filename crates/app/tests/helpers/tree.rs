// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::parse_header;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::TreeContext;

use bytes::Bytes;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub async fn create_path(
    cl: &reqwest::Client,
    addr: &str,
    TreeContext { tag, path }: &TreeContext,
    mime: Mime,
    body: Vec<u8>,
) {
    let url = format!("{addr}/{}/_tag/{}/tree/{path}", tag.repository, tag.name);

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

    let body_digest = Algorithms::default().read(&body[..]).await.unwrap();
    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, mime.to_string())
        .header("content-digest", &body_digest.to_string())
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

    // TODO: Reenable assertion, once server-side issue is fixed
    //let content_length = res.content_length().unwrap();
    //assert_eq!(content_length, body.len() as u64);

    let content_type: Mime = parse_header(res.headers(), CONTENT_TYPE.as_str());
    assert_eq!(content_type, mime);

    let content_digest: ContentDigest = parse_header(res.headers(), "content-digest");
    assert_eq!(body_digest, content_digest);

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );

    let content_length = res.content_length().unwrap();
    assert_eq!(content_length, body.len() as u64);

    let content_type: Mime = parse_header(res.headers(), CONTENT_TYPE.as_str());
    assert_eq!(content_type, mime);

    let content_digest: ContentDigest = parse_header(res.headers(), "content-digest");
    assert_eq!(body_digest, content_digest);

    assert_eq!(res.bytes().await.unwrap(), body);

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, mime.to_string())
        .header("content-digest", &body_digest.to_string())
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

pub async fn get_path(
    cl: &reqwest::Client,
    addr: &str,
    TreeContext { tag, path }: &TreeContext,
) -> (Bytes, Mime) {
    let url = format!("{addr}/{}/_tag/{}/tree/{path}", tag.repository, tag.name);

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let content_length = res.content_length().unwrap();
    let content_type = parse_header(res.headers(), CONTENT_TYPE.as_str());
    let content_digest: ContentDigest = parse_header(res.headers(), "content-digest");

    let body = res.bytes().await.unwrap();
    assert_eq!(body.len() as u64, content_length);

    let body_digest = Algorithms::default().read(&body[..]).await.unwrap();
    assert_eq!(body_digest, content_digest);

    (body, content_type)
}
