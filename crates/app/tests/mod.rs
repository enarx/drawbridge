// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod helpers;

use helpers::*;

use std::time::Duration;

use drawbridge_app::{App, TlsConfig};
use drawbridge_type::digest::Algorithms;
use drawbridge_type::{
    Meta, RepositoryConfig, RepositoryContext, TagContext, TagEntry, TreeContext, TreeEntry,
    UserConfig,
};

use async_std::net::{Ipv4Addr, TcpListener};
use futures::channel::oneshot::channel;
use futures::StreamExt;
use mime::TEXT_PLAIN;
use reqwest::StatusCode;
use tempfile::tempdir;

// TODO: Migrate the test to `ureq` or some http client compatible
// with async-std and reenable the test.
//#[async_std::test]
#[allow(dead_code)]
async fn app() {
    env_logger::builder().is_test(true).init();

    let lis = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).await.unwrap();
    let addr = format!("https://{}", lis.local_addr().unwrap());
    let store = tempdir().expect("failed to create temporary store directory");

    let (tx, rx) = channel::<()>();
    let srv = async_std::task::spawn(async move {
        let tls = TlsConfig::read(
            include_bytes!("../../../testdata/server.crt").as_slice(),
            include_bytes!("../../../testdata/server.key").as_slice(),
            include_bytes!("../../../testdata/ca.crt").as_slice(),
        )
        .unwrap();
        let app = App::new(store.path(), tls).await.unwrap();
        lis.incoming()
            .take_until(rx)
            .for_each_concurrent(None, |stream| async {
                app.handle(stream.expect("failed to initialize stream"))
                    .await
                    .expect("failed to handle stream")
            })
            .await
    });

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
                    content: (),
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
    assert!(matches!(srv.await, ()));
}
