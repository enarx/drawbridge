// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod helpers;

use helpers::*;

use std::net::{Ipv4Addr, TcpListener};
use std::time::Duration;

use drawbridge_app::Builder;
use drawbridge_type::digest::Algorithms;
use drawbridge_type::{
    Meta, RepositoryConfig, RepositoryContext, TagContext, TagEntry, TreeContext, TreeEntry,
    UserConfig,
};

use axum::Server;
use futures::channel::oneshot::channel;
use mime::TEXT_PLAIN;
use reqwest::StatusCode;
use tempfile::tempdir;

#[tokio::test]
async fn app() {
    let lis = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
    let addr = format!("http://{}", lis.local_addr().unwrap());
    let store = tempdir().expect("failed to create temporary store directory");

    let (tx, rx) = channel::<()>();
    let srv = tokio::spawn(
        Server::from_tcp(lis)
            .unwrap()
            .serve(Builder::new(store.path()).build().unwrap())
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

    let user = "user".parse().unwrap();
    user::create(&cl, &addr, &user, UserConfig {}).await;

    let foo = RepositoryContext {
        owner: user.clone(),
        name: "foo".parse().unwrap(),
    };
    repo::create(&cl, &addr, &foo, RepositoryConfig {}).await;

    let bar = RepositoryContext {
        owner: user,
        name: "bar".parse().unwrap(),
    };
    repo::create(&cl, &addr, &bar, RepositoryConfig {}).await;

    assert_eq!(tag::list(&cl, &addr, &foo).await, vec![]);
    assert_eq!(tag::list(&cl, &addr, &bar).await, vec![]);

    let v0_1_0 = TagContext {
        repository: foo.clone(),
        name: "0.1.0".parse().unwrap(),
    };
    tag::create(
        &cl,
        &addr,
        &v0_1_0,
        Algorithms::default()
            .read(b"test".as_slice())
            .await
            .map(|(size, hash)| {
                TagEntry::Unsigned(TreeEntry {
                    meta: Meta {
                        hash,
                        size,
                        mime: TEXT_PLAIN,
                    },
                    custom: Default::default(),
                })
            })
            .unwrap(),
    )
    .await;

    assert_eq!(tag::list(&cl, &addr, &foo).await, vec![v0_1_0.name.clone()]);
    assert_eq!(tag::list(&cl, &addr, &bar).await, vec![]);

    let root = TreeContext {
        tag: v0_1_0,
        path: "/".parse().unwrap(),
    };
    tree::create_path(&cl, &addr, &root, TEXT_PLAIN, b"test".to_vec()).await;

    let (test_resp, test_type) = tree::get_path(&cl, &addr, &root).await;

    assert_eq!(test_type, TEXT_PLAIN);
    assert_eq!(&test_resp[..], b"test");

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
