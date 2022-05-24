// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use drawbridge_type::{RepositoryName, TagEntry, TagName, TreeEntry};

use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub async fn create(
    cl: &reqwest::Client,
    addr: &str,
    repo: &RepositoryName,
    name: &TagName,
    tag: TagEntry,
) {
    let url = format!("{}/{}/_tag/{}", addr, repo, name);

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
        .header(CONTENT_TYPE, TreeEntry::TYPE)
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
    // TODO: Validate headers

    let res = cl.get(&url).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );
    assert_eq!(res.json::<TagEntry>().await.unwrap(), tag);
    // TODO: Validate headers

    let res = cl
        .put(&url)
        .header(CONTENT_TYPE, TreeEntry::TYPE)
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
    let res = cl
        .get(format!("{}/{}/_tag", addr, repo))
        .send()
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::OK,
        "{}",
        res.text().await.unwrap()
    );
    res.json().await.unwrap()
}
