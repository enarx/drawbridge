// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use drawbridge_client::mime::APPLICATION_OCTET_STREAM;
use drawbridge_client::types::{RepositoryConfig, TreePath, UserRecord};
use drawbridge_client::Client;
use drawbridge_server::{App, OidcConfig, TlsConfig};

use async_std::fs::{create_dir, write};
use async_std::net::{Ipv4Addr, TcpListener};
use async_std::task::{spawn, spawn_blocking};
use drawbridge_type::digest::Algorithms;
use drawbridge_type::Meta;
use futures::channel::oneshot::channel;
use futures::{join, try_join, StreamExt};
use http_types::convert::{json, Serialize};
use http_types::{Body, Response, StatusCode};
use jsonwebtoken::{encode, EncodingKey, Header};
use openidconnect::core::{
    CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType, CoreSubjectIdentifierType,
};
use openidconnect::{
    AuthUrl, EmptyAdditionalProviderMetadata, IssuerUrl, JsonWebKeySetUrl, ResponseTypes,
};
use rsa::{pkcs1::EncodeRsaPrivateKey, PublicKeyParts};
use rustls::{Certificate, PrivateKey, RootCertStore};
use rustls_pemfile::Item::*;
use tempfile::tempdir;

#[derive(Debug, Clone, serde::Serialize)]
struct TokenClaims {
    #[serde(rename = "iss")]
    issuer: String,
    #[serde(rename = "aud")]
    audience: Vec<String>,
    #[serde(rename = "sub")]
    subject: String,
    #[serde(rename = "iat")]
    issued_at: u64,
    #[serde(rename = "exp")]
    expires_at: u64,
    scope: String,
}

#[async_std::test]
async fn app() {
    tracing_subscriber::fmt::init();

    let oidc_lis = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("failed to bind to address");
    let oidc_addr = oidc_lis.local_addr().unwrap();
    let oidc_issuer = format!("http://{}/", oidc_addr);
    let oidc_audience = "testserver";

    let mut rng = rand::thread_rng();
    let oidc_key_raw = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
    let oidc_key_der = oidc_key_raw.to_pkcs1_der().expect("failed to encode key");
    let oidc_key = EncodingKey::from_rsa_der(&oidc_key_der.as_bytes());
    let oidc_key_pub = oidc_key_raw.to_public_key();
    let oidc_key_jwk = drawbridge_jose::jwk::Jwk {
        key: drawbridge_jose::jwk::Key::Rsa {
            prv: None,
            n: oidc_key_pub.n().to_bytes_be().into(),
            e: oidc_key_pub.e().to_bytes_be().into(),
        },
        prm: drawbridge_jose::jwk::Parameters {
            kid: Some("signkey".into()),
            ..Default::default()
        },
    };
    let oidc_pubkeys = drawbridge_jose::jwk::JwkSet {
        keys: vec![oidc_key_jwk.clone()],
    };

    const SUBJECT: &str = "test|subject";

    let mut jwt_header = Header::new(jsonwebtoken::Algorithm::RS256);
    jwt_header.kid = Some("signkey".to_owned());
    let jwt_payload = TokenClaims {
        issuer: oidc_issuer.clone(),
        audience: vec![oidc_audience.to_owned()],
        subject: SUBJECT.to_owned(),
        issued_at: SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        expires_at: (SystemTime::now() + Duration::from_secs(3600))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        scope:
            "openid manage:drawbridge_users manage:drawbridge_repositories manage:drawbridge_tags"
                .into(),
    };

    let mut oidc_tokens = HashMap::from([
        ("valid", {
            let payload = jwt_payload.clone();
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("expired", {
            let mut payload = jwt_payload.clone();
            payload.issued_at = SystemTime::now()
                .checked_sub(Duration::from_secs(3600))
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            payload.expires_at = SystemTime::now()
                .checked_sub(Duration::from_secs(3600))
                .unwrap()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("no_scopes", {
            let mut payload = jwt_payload.clone();
            payload.scope = "openid".into();
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("missing_manage_user_scope", {
            let mut payload = jwt_payload.clone();
            payload.scope = "openid manage:drawbridge_repositories manage:drawbridge_tags read:drawbridge_users".into();
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("invalid_aud", {
            let mut payload = jwt_payload.clone();
            payload.audience = vec!["invalid".into()];
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("invalid_iss", {
            let mut payload = jwt_payload.clone();
            payload.issuer = "invalid".into();
            encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token")
        }),
        ("invalid_sig", {
            let payload = jwt_payload.clone();
            let mut token = encode(&jwt_header, &payload, &oidc_key).expect("failed to sign token");
            token.pop();
            token
        }),
    ]);
    let oidc_token_valid = oidc_tokens.remove("valid").unwrap();

    let (oidc_tx, oidc_rx) = channel::<()>();
    let oidc = spawn(async move {
        oidc_lis
            .incoming()
            .take_until(oidc_rx)
            .for_each_concurrent(None, |stream| async {
                let oidc_pubkeys = &oidc_pubkeys;

                async_h1::accept(
                    stream.expect("failed to initialize stream"),
                    |req| async move {
                        fn json_response(
                            body: &impl Serialize,
                        ) -> Result<Response, http_types::Error> {
                            let mut res = Response::new(StatusCode::Ok);
                            res.insert_header("Content-Type", "application/json");
                            let body = Body::from_json(&json!(body))?;
                            res.set_body(body);
                            Ok(res)
                        }

                        let oidc_url = format!("http://{oidc_addr}/");
                        match req.url().path() {
                            "/.well-known/openid-configuration" => {
                                json_response(&CoreProviderMetadata::new(
                                    // Parameters required by the OpenID Connect Discovery spec.
                                    IssuerUrl::new(oidc_url.to_string()).unwrap(),
                                    AuthUrl::new(format!("{oidc_url}authorize")).unwrap(),
                                    // Use the JsonWebKeySet struct to serve the JWK Set at this URL.
                                    JsonWebKeySetUrl::new(format!("{oidc_url}jwks")).unwrap(),
                                    vec![ResponseTypes::new(vec![CoreResponseType::Code])],
                                    vec![CoreSubjectIdentifierType::Pairwise],
                                    vec![CoreJwsSigningAlgorithm::RsaSsaPssSha256],
                                    EmptyAdditionalProviderMetadata {},
                                ))
                            }
                            "/jwks" => json_response(&oidc_pubkeys),
                            p => panic!("Unsupported path requested: `{p}`"),
                        }
                    },
                )
                .await
                .expect("failed to handle OIDC connection");
            })
            .await
    });

    let srv_lis = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .expect("failed to bind to address");
    let srv_port = srv_lis.local_addr().unwrap().port();

    let store = tempdir().expect("failed to create temporary store directory");

    let (srv_tx, srv_rx) = channel::<()>();
    let srv = spawn(async move {
        let tls = TlsConfig::read(
            include_bytes!("../testdata/server.crt").as_slice(),
            include_bytes!("../testdata/server.key").as_slice(),
            include_bytes!("../testdata/ca.crt").as_slice(),
        )
        .unwrap();
        let app = App::new(
            store.path(),
            tls,
            OidcConfig {
                audience: oidc_audience.to_string(),
                issuer: oidc_issuer.parse().unwrap(),
            },
        )
        .await
        .unwrap();
        srv_lis
            .incoming()
            .take_until(srv_rx)
            .for_each_concurrent(None, |stream| async {
                app.handle(stream.expect("failed to initialize stream"))
                    .await
                    .expect("failed to handle stream")
            })
            .await
    });

    let cl = spawn_blocking(move || async move {
        let (anon_cl, cert_cl, oidc_valid_cl, blank_cl) = {
            let cl = Client::builder(format!("https://localhost:{srv_port}").parse().unwrap())
                .roots({
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

            let cert = rustls_pemfile::certs(&mut std::io::BufReader::new(
                include_bytes!("../testdata/client.crt").as_slice(),
            ))
            .unwrap()
            .into_iter()
            .map(Certificate)
            .collect::<Vec<Certificate>>();

            let key = rustls_pemfile::read_one(&mut std::io::BufReader::new(
                include_bytes!("../testdata/client.key").as_slice(),
            ))
            .unwrap()
            .map(|item| match item {
                RSAKey(buf) | PKCS8Key(buf) | ECKey(buf) => PrivateKey(buf),
                _ => panic!("unsupported key type `{:?}`", item),
            })
            .unwrap();

            (
                cl.clone().build().unwrap(),
                cl.clone().credentials(cert, key).build().unwrap(),
                cl.clone().token(oidc_token_valid).build().unwrap(),
                cl,
            )
        };

        let user_name = "testuser".parse().unwrap();
        let user_record = UserRecord {
            subject: SUBJECT.into(),
        };

        let anon_user = anon_cl.user(&user_name);
        let cert_user = cert_cl.user(&user_name);
        let oidc_user = oidc_valid_cl.user(&user_name);

        assert!(anon_user.get().is_err());
        assert!(cert_user.get().is_err());
        assert!(oidc_user.get().is_err());

        assert!(anon_user.create(&user_record).is_err());
        assert!(cert_user.create(&user_record).is_err());
        for (token_type, token) in oidc_tokens {
            let client = blank_cl.clone().token(token).build().unwrap();
            assert!(
                client.user(&user_name).create(&user_record).is_err(),
                "OIDC token type {} accepted incorrectly",
                token_type
            );
        }
        assert!(oidc_user
            .create(&UserRecord {
                subject: format!("{}other", user_record.subject),
            })
            .is_err());
        assert!(oidc_user
            .create(&user_record)
            .expect("failed to create user"));
        assert!(oidc_valid_cl
            .user(&format!("{user_name}other").parse().unwrap())
            .create(&user_record)
            .expect("failed to create other user"));

        assert!(anon_user.get().is_err());
        assert!(cert_user.get().is_err());
        assert_eq!(oidc_user.get().expect("failed to get user"), user_record);

        let prv_repo_name = "test-repo-private".parse().unwrap();
        let prv_repo_conf = RepositoryConfig { public: false };

        let pub_repo_name = "test-repo-public".parse().unwrap();
        let pub_repo_conf = RepositoryConfig { public: true };

        let anon_prv_repo = anon_user.repository(&prv_repo_name);
        let cert_prv_repo = cert_user.repository(&prv_repo_name);
        let oidc_prv_repo = oidc_user.repository(&prv_repo_name);

        let anon_pub_repo = anon_user.repository(&pub_repo_name);
        let cert_pub_repo = cert_user.repository(&pub_repo_name);
        let oidc_pub_repo = oidc_user.repository(&pub_repo_name);

        assert!(anon_prv_repo.get().is_err());
        assert!(cert_prv_repo.get().is_err());
        assert!(oidc_prv_repo.get().is_err());

        assert!(anon_pub_repo.get().is_err());
        assert!(cert_pub_repo.get().is_err());
        assert!(oidc_pub_repo.get().is_err());

        assert!(anon_prv_repo.tags().is_err());
        assert!(cert_prv_repo.tags().is_err());
        assert!(oidc_prv_repo.tags().is_err());

        assert!(anon_pub_repo.tags().is_err());
        assert!(cert_pub_repo.tags().is_err());
        assert!(oidc_pub_repo.tags().is_err());

        assert!(anon_prv_repo.create(&prv_repo_conf).is_err());
        assert!(cert_prv_repo.create(&prv_repo_conf).is_err());
        assert_eq!(
            oidc_prv_repo
                .create(&prv_repo_conf)
                .expect("failed to create repository"),
            true
        );

        assert!(anon_pub_repo.create(&pub_repo_conf).is_err());
        assert!(cert_pub_repo.create(&pub_repo_conf).is_err());
        assert_eq!(
            oidc_pub_repo
                .create(&pub_repo_conf)
                .expect("failed to create repository"),
            true
        );

        assert!(anon_prv_repo.get().is_err());
        assert!(cert_prv_repo.get().is_err());
        assert_eq!(
            oidc_prv_repo.get().expect("failed to get repository"),
            prv_repo_conf
        );

        assert!(anon_pub_repo.get().is_err());
        assert!(cert_pub_repo.get().is_err());
        assert_eq!(
            oidc_pub_repo.get().expect("failed to get repository"),
            pub_repo_conf
        );

        assert!(anon_prv_repo.tags().is_err());
        assert!(cert_prv_repo.tags().is_err());
        assert_eq!(oidc_prv_repo.tags().expect("failed to get tags"), vec![]);

        assert_eq!(anon_pub_repo.tags().expect("failed to get tags"), vec![]);
        assert_eq!(cert_pub_repo.tags().expect("failed to get tags"), vec![]);
        assert_eq!(oidc_pub_repo.tags().expect("failed to get tags"), vec![]);

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

        let tag_name = "0.1.0".parse().unwrap();

        let anon_prv_tag = anon_prv_repo.tag(&tag_name);
        let cert_prv_tag = cert_prv_repo.tag(&tag_name);
        let oidc_prv_tag = oidc_prv_repo.tag(&tag_name);

        let anon_pub_tag = anon_pub_repo.tag(&tag_name);
        let cert_pub_tag = cert_pub_repo.tag(&tag_name);
        let oidc_pub_tag = oidc_pub_repo.tag(&tag_name);

        assert!(anon_prv_tag.get().is_err());
        assert!(cert_prv_tag.get().is_err());
        assert!(oidc_prv_tag.get().is_err());

        assert!(anon_pub_tag.get().is_err());
        assert!(cert_pub_tag.get().is_err());
        assert!(oidc_pub_tag.get().is_err());

        assert!(anon_prv_tag.create_from_path_unsigned(pkg.path()).is_err());
        assert!(cert_prv_tag.create_from_path_unsigned(pkg.path()).is_err());
        let (prv_tag_created, prv_tree_created) = oidc_prv_tag
            .create_from_path_unsigned(pkg.path())
            .expect("failed to create a tag and upload the tree");
        assert!(prv_tag_created);
        assert_eq!(
            prv_tree_created.clone().into_iter().collect::<Vec<_>>(),
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

        assert!(anon_pub_tag.create_from_path_unsigned(pkg.path()).is_err());
        assert!(cert_pub_tag.create_from_path_unsigned(pkg.path()).is_err());
        assert_eq!(
            oidc_pub_tag
                .create_from_path_unsigned(pkg.path())
                .expect("failed to create a tag and upload the tree"),
            (prv_tag_created, prv_tree_created)
        );

        assert!(anon_prv_repo.tags().is_err());
        assert!(cert_prv_repo.tags().is_err());
        assert_eq!(
            oidc_prv_repo.tags().expect("failed to get tags"),
            vec![tag_name.clone()]
        );

        assert_eq!(
            anon_pub_repo.tags().expect("failed to get tags"),
            vec![tag_name.clone()]
        );
        assert_eq!(
            cert_pub_repo.tags().expect("failed to get tags"),
            vec![tag_name.clone()]
        );
        assert_eq!(
            oidc_pub_repo.tags().expect("failed to get tags"),
            vec![tag_name.clone()]
        );

        let file_name = "test-file.txt".parse().unwrap();
        let file_meta = Algorithms::default()
            .read_sync("text".as_bytes())
            .map(|(size, hash)| Meta {
                hash,
                size,
                mime: APPLICATION_OCTET_STREAM,
            })
            .unwrap();
        let file_expected = (file_meta, "text".into());

        let anon_prv_file = anon_prv_tag.path(&file_name);
        let cert_prv_file = cert_prv_tag.path(&file_name);
        let oidc_prv_file = oidc_prv_tag.path(&file_name);

        let anon_pub_file = anon_pub_tag.path(&file_name);
        let cert_pub_file = cert_pub_tag.path(&file_name);
        let oidc_pub_file = oidc_pub_tag.path(&file_name);

        assert!(anon_prv_file.get_string(5).is_err());
        assert_eq!(
            cert_prv_file.get_string(5).expect("failed to get file"),
            file_expected,
        );
        assert_eq!(
            oidc_prv_file.get_string(5).expect("failed to get file"),
            file_expected,
        );

        assert_eq!(
            anon_pub_file.get_string(5).expect("failed to get file"),
            file_expected,
        );
        assert_eq!(
            cert_pub_file.get_string(5).expect("failed to get file"),
            file_expected,
        );
        assert_eq!(
            oidc_pub_file.get_string(5).expect("failed to get file"),
            file_expected,
        );
    });
    assert!(matches!(cl.await.await, ()));

    // Stop OpenID Connect provider
    assert_eq!(oidc_tx.send(()), Ok(()));

    // Stop server
    assert_eq!(srv_tx.send(()), Ok(()));
    assert!(matches!(join!(oidc, srv), ((), ())));
}
