// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

mod helpers;

use helpers::*;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use drawbridge_app::Builder;
use drawbridge_test_helper::TestHelper;
use drawbridge_type::digest::Algorithms;
use drawbridge_type::{RepositoryConfig, TagEntry, TreeEntry};

use reqwest::{Client, StatusCode};
use tempfile::tempdir;

async fn app_tests(th: &TestHelper, cl: Client) {
    let lis = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    let addr = format!("https://{}", lis);
    let store = tempdir().expect("failed to create temporary store directory");

    let graceful_shutdown = Arc::new(AtomicBool::new(false));
    let graceful_shutdown_clone = graceful_shutdown.clone();
    let pem_certificates = th.server_pem.clone();
    let pem_private_key = th.server_private_pem.clone();
    let srv = tokio::spawn(async move {
        Builder::new(store.path())
            .serve(
                &lis,
                &pem_certificates,
                &pem_private_key,
                // th.ca_pem,
                None,
                graceful_shutdown_clone,
            )
            .await
            .map_err(|e| format!("{}", e))
    });
    // Wait for the server to start
    tokio::time::sleep(Duration::from_secs(1)).await;
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
    graceful_shutdown.store(true, Ordering::Relaxed);
    assert!(matches!(srv.await, Ok(Ok(()))));
}

#[tokio::test]
async fn app() {
    let th = TestHelper::default();

    // All of these tests should behave the same as none of them use authentication at the moment
    app_tests(&th, th.unauthenticated_client()).await;
    app_tests(&th, th.certificate_auth_client()).await;
    app_tests(&th, th.invalid_certificate_auth_client()).await;
}
