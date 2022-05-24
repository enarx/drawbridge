// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod helpers;

use helpers::*;

use std::net::{Ipv4Addr, TcpListener};
use std::time::Duration;

use drawbridge_app::Builder;
use drawbridge_type::digest::Algorithms;
use drawbridge_type::{RepositoryConfig, TagEntry, TreeEntry};

use axum::Server;
use futures::channel::oneshot::channel;
use reqwest::StatusCode;

#[tokio::test]
async fn app() {
    let lis = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let addr = format!("http://{}", lis.local_addr().unwrap());

    let (tx, rx) = channel::<()>();
    let srv = tokio::spawn(
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

    let foo = "user/foo".parse().unwrap();
    repo::create(&cl, &addr, &foo, RepositoryConfig {}).await;

    let bar = "user/bar".parse().unwrap();
    repo::create(&cl, &addr, &bar, RepositoryConfig {}).await;

    assert_eq!(tag::list(&cl, &addr, &foo).await, vec![]);
    assert_eq!(tag::list(&cl, &addr, &bar).await, vec![]);

    let tag = "0.1.0".parse().unwrap();
    tag::create(
        &cl,
        &addr,
        &foo,
        &tag,
        TagEntry::Unsigned(TreeEntry {
            digest: Algorithms::default()
                .read(b"testing".as_slice())
                .await
                .unwrap(),
            custom: Default::default(),
        }),
    )
    .await;

    assert_eq!(tag::list(&cl, &addr, &foo).await, vec![tag.clone()]);
    assert_eq!(tag::list(&cl, &addr, &bar).await, vec![]);

    tree::create_path(
        &cl,
        &addr,
        &foo,
        &tag,
        &"/".parse().unwrap(),
        mime::TEXT_PLAIN,
        b"testing".to_vec(),
    )
    .await;

    let (test_resp, test_type) =
        tree::get_path(&cl, &addr, &foo, &tag, &"/".parse().unwrap()).await;

    assert_eq!(test_type, mime::TEXT_PLAIN);
    assert_eq!(&test_resp[..], b"testing");

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
