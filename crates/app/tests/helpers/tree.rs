use drawbridge_type::{RepositoryName, TagName, TreePath};

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
    let url = format!("{}/{}/_tag/{}/tree{}", addr, repo, tag, path);

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
