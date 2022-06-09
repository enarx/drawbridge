// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, HashMap};
use std::net::{Ipv4Addr, TcpListener};

use drawbridge_app::Builder;
use drawbridge_client::mime::TEXT_PLAIN;
use drawbridge_client::types::digest::Algorithms;
use drawbridge_client::types::{
    Meta, RepositoryConfig, TagEntry, TreeDirectory, TreeEntry, UserConfig,
};
use drawbridge_client::{Client, Url};

use futures::channel::oneshot::channel;
use hyper::Server;
use serde_json::json;
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
    let cl = tokio::task::spawn_blocking(move || {
        let cl = Client::builder(addr.parse::<Url>().unwrap()).build();

        let user_name = "user".parse().unwrap();
        let user = cl.user(&user_name);
        assert!(user.get().is_err());
        assert_eq!(user.create(&UserConfig {}).unwrap(), true);

        let foo_repo = "foo".parse().unwrap();
        let foo = user.repository(&foo_repo);

        let bar_repo = "bar".parse().unwrap();
        let bar = user.repository(&bar_repo);

        assert!(foo.get().is_err());
        assert!(bar.get().is_err());

        assert_eq!(foo.create(&RepositoryConfig {}).unwrap(), true);
        assert_eq!(bar.create(&RepositoryConfig {}).unwrap(), true);

        assert_eq!(foo.tags().unwrap(), vec![]);
        assert_eq!(bar.tags().unwrap(), vec![]);

        let test_meta = Meta {
            hash: Algorithms::default().read_sync(b"test".as_slice()).unwrap(),
            size: "test".len() as _,
            mime: TEXT_PLAIN,
        };

        let root = TreeDirectory::from({
            let mut m = BTreeMap::new();
            m.insert(
                "test-file".into(),
                TreeEntry {
                    meta: test_meta,
                    custom: {
                        let mut m = HashMap::new();
                        m.insert("custom_field".into(), json!("custom_value"));
                        m
                    },
                },
            );
            m
        });
        let root_json = serde_json::to_vec(&root).unwrap();
        let root_meta = Meta {
            hash: Algorithms::default()
                .read_sync(root_json.as_slice())
                .unwrap(),
            size: root_json.len() as _,
            mime: TreeDirectory::TYPE.parse().unwrap(),
        };

        let tag = TagEntry::Unsigned(TreeEntry {
            meta: root_meta,
            custom: Default::default(),
        });

        let v0_1_0 = "0.1.0".parse().unwrap();
        let foo_v0_1_0 = foo.tag(&v0_1_0);

        assert!(foo_v0_1_0.get().is_err());
        assert_eq!(foo_v0_1_0.create(&tag).unwrap(), true);
        assert_eq!(foo_v0_1_0.get().unwrap(), tag);

        let root_path = "/".parse().unwrap();
        let root_node = foo_v0_1_0.path(&root_path);
        assert!(root_node.get_string().is_err());
        assert_eq!(root_node.create_directory(&root).unwrap(), true);

        let test_path = "/test".parse().unwrap();
        let test_node = foo_v0_1_0.path(&test_path);
        assert!(test_node.get_string().is_err());
        assert_eq!(test_node.create_bytes(&TEXT_PLAIN, b"test").unwrap(), true);
        assert_eq!(test_node.get_string().unwrap(), (TEXT_PLAIN, "test".into()));
    });
    assert!(matches!(cl.await, Ok(())));

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
