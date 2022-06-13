// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::{create_dir, write};
use std::net::{Ipv4Addr, TcpListener};

use drawbridge_app::Builder;
use drawbridge_client::types::{RepositoryConfig, TreePath, UserConfig};
use drawbridge_client::Client;

use futures::channel::oneshot::channel;
use hyper::Server;
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
        let cl = Client::builder(addr.parse().unwrap()).build();

        let user = cl.user(&"user".parse().unwrap());
        assert!(user.get().is_err());
        assert_eq!(user.create(&UserConfig {}).unwrap(), true);

        let foo = user.repository(&"foo".parse().unwrap());
        let bar = user.repository(&"bar".parse().unwrap());

        assert!(foo.get().is_err());
        assert!(bar.get().is_err());

        assert_eq!(foo.create(&RepositoryConfig {}).unwrap(), true);
        assert_eq!(bar.create(&RepositoryConfig {}).unwrap(), true);

        assert_eq!(foo.tags().unwrap(), vec![]);
        assert_eq!(bar.tags().unwrap(), vec![]);

        let pkg = tempdir().expect("failed to create temporary package directory");

        write(pkg.path().join("test-file"), "no extension").unwrap();
        write(pkg.path().join("test-file.txt"), "text").unwrap();
        write(pkg.path().join("test-file.json"), "not valid json").unwrap();
        write(pkg.path().join("tEst-file..__.foo.42."), "invalidext").unwrap();

        create_dir(pkg.path().join("test-dir-1")).unwrap();
        write(pkg.path().join("test-dir-1").join("test-file.txt"), "text").unwrap();
        write(pkg.path().join("test-dir-1").join("test-file"), "test").unwrap();

        create_dir(pkg.path().join("test-dir-1").join("test-subdir-1")).unwrap();
        create_dir(pkg.path().join("test-dir-1").join("test-subdir-2")).unwrap();
        write(
            pkg.path()
                .join("test-dir-1")
                .join("test-subdir-2")
                .join("test-file"),
            "test",
        )
        .unwrap();

        let v0_1_0 = "0.1.0".parse().unwrap();

        let (tag_created, tree_created) = foo
            .tag(&v0_1_0)
            .create_from_path_unsigned(pkg.path())
            .expect("failed to create a tag and upload the tree");
        assert!(tag_created);
        assert_eq!(
            tree_created.clone().into_iter().collect::<Vec<_>>(),
            vec![
                (TreePath::ROOT, true),
                ("tEst-file..__.foo.42.".parse().unwrap(), true),
                ("test-dir-1".parse().unwrap(), true),
                ("test-dir-1/test-file".parse().unwrap(), true),
                ("test-dir-1/test-file.txt".parse().unwrap(), true),
                ("test-dir-1/test-subdir-1".parse().unwrap(), true),
                ("test-dir-1/test-subdir-2".parse().unwrap(), true),
                ("test-dir-1/test-subdir-2/test-file".parse().unwrap(), true),
                ("test-file".parse().unwrap(), true),
                ("test-file.json".parse().unwrap(), true),
                ("test-file.txt".parse().unwrap(), true),
            ]
        );

        assert_eq!(foo.tags().unwrap(), vec![v0_1_0.clone()]);
        assert_eq!(bar.tags().unwrap(), vec![]);

        let (tag_created, tree_created) = bar
            .tag(&v0_1_0)
            .create_from_path_unsigned(pkg.path())
            .expect("failed to create a tag and upload the tree");
        assert!(tag_created);
        assert_eq!(
            tree_created.clone().into_iter().collect::<Vec<_>>(),
            vec![
                (TreePath::ROOT, true),
                ("tEst-file..__.foo.42.".parse().unwrap(), true),
                ("test-dir-1".parse().unwrap(), true),
                ("test-dir-1/test-file".parse().unwrap(), true),
                ("test-dir-1/test-file.txt".parse().unwrap(), true),
                ("test-dir-1/test-subdir-1".parse().unwrap(), true),
                ("test-dir-1/test-subdir-2".parse().unwrap(), true),
                ("test-dir-1/test-subdir-2/test-file".parse().unwrap(), true),
                ("test-file".parse().unwrap(), true),
                ("test-file.json".parse().unwrap(), true),
                ("test-file.txt".parse().unwrap(), true),
            ]
        );

        assert_eq!(foo.tags().unwrap(), vec![v0_1_0.clone()]);
        assert_eq!(bar.tags().unwrap(), vec![v0_1_0]);
    });
    assert!(matches!(cl.await, Ok(())));

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
