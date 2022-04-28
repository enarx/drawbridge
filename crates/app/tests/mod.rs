use std::net::{Ipv4Addr, TcpListener};
use std::time::Duration;

use drawbridge_app::Builder;
use drawbridge_type::{RepositoryConfig, RepositoryName, TagEntry, TagName, TreeEntry, TreePath};

use axum::Server;
use futures::channel::oneshot::channel;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

async fn create_repo(
    cl: &reqwest::Client,
    addr: &str,
    name: &RepositoryName,
    conf: RepositoryConfig,
) {
    let url = format!("{}/{}", addr, name);

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

    let res = cl.put(&url).json(&conf).send().await.unwrap();
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
    assert_eq!(res.json::<RepositoryConfig>().await.unwrap(), conf);
    // TODO: Validate headers

    let res = cl.put(&url).json(&conf).send().await.unwrap();
    assert_eq!(
        res.status(),
        // TODO: This should not result in conflict, since payload is the same.
        StatusCode::CONFLICT,
        "{}",
        res.text().await.unwrap()
    );
}

async fn create_tag(cl: &reqwest::Client, addr: &str, name: &TagName, tag: TagEntry) {
    let url = format!("{}/_tag/{}", addr, name);

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

async fn create_path(cl: &reqwest::Client, addr: &str, path: &TreePath, mime: Mime, body: Vec<u8>) {
    let url = format!("{}/tree{}", addr, path);

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

#[tokio::test]
async fn app() {
    let lis = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let addr = format!("http://{}", lis.local_addr().unwrap());

    let (tx, rx) = channel::<()>();
    tokio::spawn(
        Server::from_tcp(lis)
            .unwrap()
            .serve(Builder::new().build())
            .with_graceful_shutdown(async { rx.await.ok().unwrap() }),
    );

    let cl = reqwest::Client::builder()
        .timeout(Duration::new(1, 0))
        .build()
        .unwrap();

    let res = cl.get(&addr).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "{}",
        res.text().await.unwrap()
    );

    let res = cl.get(format!("{}/", addr)).send().await.unwrap();
    assert_eq!(
        res.status(),
        StatusCode::NOT_FOUND,
        "{}",
        res.text().await.unwrap()
    );

    let repo = "user/repo".parse().unwrap();
    create_repo(&cl, &addr, &repo, RepositoryConfig {}).await;

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
    assert_eq!(res.json::<Vec<TagName>>().await.unwrap(), vec![]);

    let tag = "0.1.0".parse().unwrap();
    create_tag(
        &cl,
        &format!("{}/{}", addr, repo),
        &tag,
        TagEntry::Unsigned(TreeEntry {
            digest: Default::default(), // TODO: Set and require digest
            custom: Default::default(),
        }),
    )
    .await;

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
    assert_eq!(res.json::<Vec<TagName>>().await.unwrap(), vec![tag.clone()]);

    let path = "/".parse().unwrap();
    create_path(
        &cl,
        &format!("{}/{}/_tag/{}", addr, repo, tag),
        &path,
        mime::TEXT_PLAIN,
        b"test".into(),
    )
    .await;

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
}
