// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use drawbridge_app::{App, TlsConfig};
use drawbridge_client::mime::APPLICATION_OCTET_STREAM;
use drawbridge_client::types::{RepositoryConfig, TreeContext, TreePath, UserConfig};
use drawbridge_client::Client;

use async_std::fs::{create_dir, write};
use async_std::net::{Ipv4Addr, TcpListener};
use async_std::task::{spawn, spawn_blocking};
use futures::channel::oneshot::channel;
use futures::{try_join, StreamExt};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::Item::*;
use tempfile::tempdir;

#[async_std::test]
async fn app() {
    env_logger::builder().is_test(true).init();

    let lis = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("failed to bind to address");
    let port = lis.local_addr().unwrap().port();
    let store = tempdir().expect("failed to create temporary store directory");

    let (tx, rx) = channel::<()>();
    let srv = spawn(async move {
        let tls = TlsConfig::read(
            include_bytes!("../testdata/server.crt").as_slice(),
            include_bytes!("../testdata/server.key").as_slice(),
            include_bytes!("../testdata/ca.crt").as_slice(),
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
    let cl = spawn_blocking(move || async move {
        let (user_cl, cert_cl, anon_cl) = {
            let cl = Client::builder(format!("https://localhost:{port}").parse().unwrap()).roots({
                let mut roots = RootCertStore::empty();
                rustls_pemfile::certs(&mut std::io::BufReader::new(
                    include_bytes!("../testdata/ca.crt").as_slice(),
                ))
                .unwrap()
                .into_iter()
                .map(Certificate)
                .try_for_each(|ref cert| roots.add(cert))
                .unwrap();
                roots
            });
            let user_cl = cl.clone().build().unwrap();
            let cert_cl = cl
                .clone()
                .credentials(
                    rustls_pemfile::certs(&mut std::io::BufReader::new(
                        include_bytes!("../testdata/client.crt").as_slice(),
                    ))
                    .unwrap()
                    .into_iter()
                    .map(Certificate)
                    .collect::<Vec<Certificate>>(),
                    rustls_pemfile::read_one(&mut std::io::BufReader::new(
                        include_bytes!("../testdata/client.key").as_slice(),
                    ))
                    .unwrap()
                    .map(|item| match item {
                        RSAKey(buf) | PKCS8Key(buf) | ECKey(buf) => PrivateKey(buf),
                        _ => panic!("unsupported key type `{:?}`", item),
                    })
                    .unwrap(),
                )
                .build()
                .unwrap();
            let anon_cl = cl.build().unwrap();
            (user_cl, cert_cl, anon_cl)
        };

        let user = user_cl.user(&"user".parse().unwrap());
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

        try_join!(
            write(pkg.path().join("test-file"), "no extension"),
            write(pkg.path().join("test-file.txt"), "text"),
            write(pkg.path().join("test-file.json"), "not valid json"),
            write(pkg.path().join("tEst-file..__.foo.42."), "invalidext"),
            create_dir(pkg.path().join("test-dir-1")),
        )
        .unwrap();

        try_join!(
            write(pkg.path().join("test-dir-1").join("test-file.txt"), "text"),
            write(pkg.path().join("test-dir-1").join("test-file"), "test"),
            create_dir(pkg.path().join("test-dir-1").join("test-subdir-1")),
            create_dir(pkg.path().join("test-dir-1").join("test-subdir-2")),
        )
        .unwrap();

        write(
            pkg.path()
                .join("test-dir-1")
                .join("test-subdir-2")
                .join("test-file"),
            "test",
        )
        .await
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
        assert_eq!(bar.tags().unwrap(), vec![v0_1_0.clone()]);

        let test_file = "test-file.txt".parse().unwrap();
        assert_eq!(
            foo.tag(&v0_1_0).path(&test_file).get_string().unwrap(),
            (APPLICATION_OCTET_STREAM, "text".into())
        );

        let test_file_cx = TreeContext {
            tag: "user/foo:0.1.0".parse().unwrap(),
            path: test_file,
        };
        assert_eq!(
            cert_cl.tree(&test_file_cx).get_string().unwrap(),
            (APPLICATION_OCTET_STREAM, "text".into())
        );
        assert_eq!(
            anon_cl.tree(&test_file_cx).get_string().unwrap(),
            (APPLICATION_OCTET_STREAM, "text".into())
        );
    });
    assert!(matches!(cl.await.await, ()));

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, ()));
}
