// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::parse_header;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::{RepositoryConfig, RepositoryContext};

use mime::{Mime, APPLICATION_JSON};
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub async fn create(
    cl: &reqwest::Client,
    addr: &str,
    repo: &RepositoryContext,
    conf: RepositoryConfig,
) {
    let url = format!("{addr}/{repo}");

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

    let body = serde_json::to_vec(&conf).unwrap();
    let (size, body_digest) = Algorithms::default().read(&body[..]).await.unwrap();
    assert_eq!(size, body.len() as u64);

    let res = cl
        .put(&url)
        .header("content-digest", &body_digest.to_string())
        .json(&conf)
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
    assert_eq!(content_type, APPLICATION_JSON);

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
    assert_eq!(content_type, APPLICATION_JSON);

    let content_digest: ContentDigest = parse_header(res.headers(), "content-digest");
    assert_eq!(body_digest, content_digest);

    assert_eq!(res.json::<RepositoryConfig>().await.unwrap(), conf);

    let res = cl
        .put(&url)
        .header("content-digest", &body_digest.to_string())
        .json(&conf)
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
