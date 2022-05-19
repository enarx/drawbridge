// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

extern crate core;

mod helpers;

use helpers::*;

use std::net::{Ipv4Addr, TcpListener};
use std::time::Duration;

use drawbridge_app::Builder;
use drawbridge_jose::b64::{Bytes, Json};
use drawbridge_jose::jws::{Flattened, Jws, Parameters, Signature};
use drawbridge_type::digest::{Algorithm, Algorithms};
use drawbridge_type::tree::Directory;
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
        TreeEntry::TYPE,
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

    println!("Put testing file in foo tag");

    let (test_resp, test_type) =
        tree::get_path(&cl, &addr, &foo, &tag, &"/".parse().unwrap()).await;

    assert_eq!(test_type, mime::TEXT_PLAIN);
    assert_eq!(&test_resp[..], b"testing");

    let random_bytes = [
        0x8Bu8, 0x3Eu8, 0x7Eu8, 0x57u8, 0x05u8, 0x73u8, 0xEEu8, 0xDFu8, 0xE6u8, 0xD2u8, 0xCBu8,
        0xAAu8, 0xD0u8, 0x19u8, 0x3Fu8, 0xABu8, 0xDDu8, 0x38u8, 0x34u8, 0x11u8, 0x7Du8, 0x51u8,
        0xC5u8,
    ];

    let random_bytes_digest = Algorithms::default()
        .read(random_bytes.as_ref())
        .await
        .unwrap();

    let random_bytes_sha256 = random_bytes_digest
        .get(&Algorithm::Sha256)
        .unwrap()
        .to_vec();

    let tag_2 = "0.2.0".parse().unwrap();
    println!("Creating signed tag.");
    tag::create(
        &cl,
        &addr,
        &foo,
        &tag_2,
        Jws::TYPE,
        TagEntry::Signed(Jws::Flattened(Flattened {
            payload: Some(Bytes::from(random_bytes_sha256)),
            signature: Signature {
                protected: Some(Json(Parameters {
                    alg: Some("RS256".to_string()),
                    ..Default::default()
                })),
                header: Some(Parameters {
                    kid: Some("e9bc097a-ce51-4036-9562-d2ade882db0d".to_string()),
                    ..Default::default()
                }),
                signature: Default::default(),
            },
        })),
    )
    .await;
    /*tag::create(
        &cl,
        &addr,
        &foo,
        &tag_2,
        Jws::TYPE,
        TagEntry::Signed(Jws::Flattened(Flattened {
            payload: None,
            signature: Signature {
                protected: None,
                header: None,
                signature: Default::default(),
            },
        })),
    )
    .await;*/

    println!("Created tag2");

    let two_tags = tag::list(&cl, &addr, &foo).await;
    assert!(two_tags.contains(&tag));
    assert!(two_tags.contains(&tag_2));

    assert_eq!(tag::list(&cl, &addr, &bar).await, vec![]);

    println!("Get two tags assertions passed.");

    tree::create_path(
        &cl,
        &addr,
        &foo,
        &tag_2,
        &"/".parse().unwrap(),
        Directory::TYPE.parse().unwrap(),
        b"/sub_directory".to_vec(),
    )
    .await;
    println!("Created directory");

    tree::create_path(
        &cl,
        &addr,
        &foo,
        &tag_2,
        &"/sub_directory/testing_file.bin".parse().unwrap(),
        mime::APPLICATION_OCTET_STREAM,
        random_bytes.to_vec(),
    )
    .await;
    println!("Created testing file in dir");

    let (test_resp, test_type) = tree::get_path(
        &cl,
        &addr,
        &foo,
        &tag_2,
        &"/sub_directory/testing_file.bin".parse().unwrap(),
    )
    .await;

    assert_eq!(test_type, mime::APPLICATION_OCTET_STREAM);
    //assert_eq!(test_resp, random_bytes);

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
