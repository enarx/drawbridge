// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::parse_header;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::{RepositoryName, TagEntry, TagName, TreeEntry};

use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub async fn create(
    cl: &reqwest::Client,
    addr: &str,
    repo: &RepositoryName,
    name: &TagName,
    tag: TagEntry,
) {
    let url = format!("{addr}/{repo}/_tag/{name}");

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

    let body = serde_json::to_vec(&tag).unwrap();
    let body_digest = Algorithms::default().read(&body[..]).await.unwrap();

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, TreeEntry::TYPE)
        .header("content-digest", &body_digest.to_string())
        .body(serde_json::to_vec(&tag).unwrap())
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
    assert_eq!(content_type, TreeEntry::TYPE);

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
    assert_eq!(content_type, TreeEntry::TYPE);

    let content_digest: ContentDigest = parse_header(res.headers(), "content-digest");
    assert_eq!(body_digest, content_digest);

    assert_eq!(res.json::<TagEntry>().await.unwrap(), tag);

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, TreeEntry::TYPE)
        .header("content-digest", &body_digest.to_string())
        .body(serde_json::to_vec(&tag).unwrap())
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

pub async fn list(cl: &reqwest::Client, addr: &str, repo: &RepositoryName) -> Vec<TagName> {
    let res = cl.get(format!("{addr}/{repo}/_tag")).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );
    res.json().await.unwrap()
}
