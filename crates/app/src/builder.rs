// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{handle, Store};

use std::error::Error;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use drawbridge_auth::CertificateSession;

use anyhow::{anyhow, Context};
use axum::handler::Handler;
use axum::routing::IntoMakeService;
use axum::{Extension, Router};
use futures_util::future::poll_fn;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, Http};
use rustls::server::AllowAnyAuthenticatedClient;
use rustls::{Certificate, RootCertStore};
use rustls_pemfile::Item;
use tokio::net::TcpListener;
use tokio_rustls::rustls::{self, PrivateKey, ServerConfig};
use tokio_rustls::TlsAcceptor;
use tower::MakeService;

pub struct Builder<S> {
    store: S,
}

impl<S: AsRef<Path>> Builder<S> {
    /// Constructs a new [Builder].
    pub fn new(store: S) -> Self {
        Self { store }
    }

    fn build_router(self) -> Result<Router, Box<dyn Error>> {
        let path = self.store.as_ref();
        let store = File::open(path).map(Store::from).context(format!(
            "failed to open store at `{}`",
            path.to_string_lossy()
        ))?;
        Ok(Router::new()
            .fallback(handle.into_service())
            .layer(Extension(Arc::new(store))))
    }

    /// Builds the application and returns Drawbridge instance as a [tower::MakeService].
    pub fn build_service(self) -> Result<IntoMakeService<Router>, Box<dyn Error>> {
        self.build_router().map(|router| router.into_make_service())
    }

    /// Serves the app with an HTTPS server with optional client authentication enabled.
    pub async fn serve(
        self,
        listen_addr: &SocketAddr,
        pem_certificates: &[u8],
        pem_private_key: &[u8],
        pem_ca: Option<Vec<u8>>,
        graceful_shutdown: Arc<AtomicBool>,
    ) -> Result<(), Box<dyn Error>> {
        let rustls_config =
            build_rustls_server_config(pem_certificates, pem_private_key, pem_ca).await?;
        let acceptor = TlsAcceptor::from(rustls_config);
        let listener = TcpListener::bind(listen_addr).await?;
        let mut listener = AddrIncoming::from_listener(listener)?;
        let app = self.build_router()?;

        loop {
            if graceful_shutdown.load(Ordering::Relaxed) {
                break;
            }

            let accept = tokio::time::timeout(
                Duration::from_secs(1),
                poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx)),
            )
            .await;

            let accept = match accept {
                Err(_) => continue,
                Ok(accept) => accept,
            };

            let stream = match accept {
                Some(Ok(result)) => result,
                _ => continue,
            };

            let acceptor = acceptor.clone();
            let mut app = app.clone();

            tokio::spawn(async move {
                let accept_result = acceptor.accept(stream).await;

                if let Ok(stream) = accept_result {
                    let (addr_stream, server_connection) = stream.get_ref();

                    if let Some(_certs) = server_connection.peer_certificates() {
                        app = app.layer(Extension(CertificateSession));
                    }

                    let app = app
                        .into_make_service_with_connect_info::<SocketAddr>()
                        .make_service(addr_stream)
                        .await
                        .unwrap();

                    let _ = Http::new().serve_connection(stream, app).await;
                }
            });
        }

        Ok(())
    }
}

async fn build_rustls_server_config(
    mut pem_certificates: &[u8],
    mut pem_private_key: &[u8],
    pem_ca: Option<Vec<u8>>,
) -> Result<Arc<ServerConfig>, anyhow::Error> {
    let certificates = rustls_pemfile::certs(&mut pem_certificates)
        .context("failed to extract certificates from buffer")?
        .into_iter()
        .map(Certificate)
        .collect();
    let key = match rustls_pemfile::read_one(&mut pem_private_key)? {
        Some(Item::RSAKey(key)) | Some(Item::PKCS8Key(key)) => PrivateKey(key),
        t => panic!("private key invalid or not supported {:?}", t),
    };

    let config_builder = ServerConfig::builder().with_safe_defaults();
    let config_builder = match pem_ca {
        None => config_builder.with_no_client_auth(),
        Some(ca) => match rustls_pemfile::read_one(&mut ca.as_ref())? {
            Some(Item::X509Certificate(ca)) => {
                let mut root_cert_store = RootCertStore::empty();
                root_cert_store
                    .add(&Certificate(ca))
                    .context("bad ca cert")?;
                config_builder
                    .with_client_cert_verifier(AllowAnyAuthenticatedClient::new(root_cert_store))
            }
            _ => {
                return Err(anyhow!("invalid root ca cert"));
            }
        },
    };

    let mut server_config = config_builder
        .with_single_cert(certificates, key)
        .context("bad certificate/key")?;

    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(server_config))
}
