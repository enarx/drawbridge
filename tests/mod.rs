use std::net::{Ipv4Addr, TcpListener};

use drawbridge_app::Builder;
use drawbridge_client::types::{RepositoryConfig, TagEntry, TreeEntry};
use drawbridge_client::{mime, Client, Url};

use futures::channel::oneshot::channel;
use hyper::Server;

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
    let cl = tokio::task::spawn_blocking(move || {
        let cl = Client::builder(addr.parse::<Url>().unwrap())
            .build()
            .unwrap();

        let foo_repo = "user/foo".parse().unwrap();
        let foo = cl.repository(&foo_repo);

        let bar_repo = "user/test/bar".parse().unwrap();
        let bar = cl.repository(&bar_repo);

        assert!(matches!(foo.get(), Err(_)));
        assert!(matches!(bar.get(), Err(_)));

        assert!(matches!(foo.create(&RepositoryConfig {}), Ok(true)));
        assert!(matches!(bar.create(&RepositoryConfig {}), Ok(true)));

        assert_eq!(foo.tags().unwrap(), vec![]);
        assert_eq!(bar.tags().unwrap(), vec![]);

        let v0_1_0 = "0.1.0".parse().unwrap();
        let foo_v0_1_0 = foo.tag(&v0_1_0);

        let entry = TagEntry::Unsigned(TreeEntry {
            digest: Default::default(),
            custom: Default::default(),
        });

        assert!(matches!(foo_v0_1_0.get(), Err(_)));
        assert!(matches!(foo_v0_1_0.create(&entry), Ok(true)));
        assert_eq!(foo_v0_1_0.get().unwrap(), entry);

        let root_path = "/".parse().unwrap();
        let root = foo_v0_1_0.path(&root_path);
        assert!(matches!(root.get_text(), Err(_)));
        assert!(matches!(root.create(mime::TEXT_PLAIN, "test"), Ok(true)));
        assert_eq!(root.get_text().unwrap(), ("test".into(), mime::TEXT_PLAIN));
    });
    assert!(matches!(cl.await, Ok(())));

    // Stop server
    assert_eq!(tx.send(()), Ok(()));
    assert!(matches!(srv.await, Ok(Ok(()))));
}
