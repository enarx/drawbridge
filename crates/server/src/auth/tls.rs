// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::io::BufRead;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context};
use rustls::server::AllowAnyAnonymousOrAuthenticatedClient;
use rustls::{Certificate, PrivateKey, RootCertStore, ServerConfig};
use rustls_pemfile::Item::{ECKey, PKCS8Key, RSAKey, X509Certificate};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct TrustedCertificate;

#[repr(transparent)]
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct Config(ServerConfig);

impl Deref for Config {
    type Target = ServerConfig;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Config> for ServerConfig {
    fn from(conf: Config) -> Self {
        conf.0
    }
}

fn read_certificates(mut rd: impl BufRead) -> anyhow::Result<Vec<Certificate>> {
    rustls_pemfile::read_all(&mut rd)?
        .into_iter()
        .map(|item| match item {
            X509Certificate(buf) => Ok(Certificate(buf)),
            _ => bail!("unsupported certificate type"),
        })
        .collect()
}

impl Config {
    pub fn read(
        mut certs: impl BufRead,
        mut key: impl BufRead,
        mut cas: impl BufRead,
    ) -> anyhow::Result<Self> {
        let certs =
            read_certificates(&mut certs).context("failed to read server certificate chain")?;
        let key = {
            let mut items = rustls_pemfile::read_all(&mut key)
                .context("failed to read server certificate key")?;
            let key = items
                .pop()
                .ok_or_else(|| anyhow!("server certificate key missing"))
                .and_then(|item| match item {
                    RSAKey(buf) | PKCS8Key(buf) | ECKey(buf) => Ok(PrivateKey(buf)),
                    _ => bail!("unsupported key type"),
                })?;
            if !items.is_empty() {
                bail!("more than one server certificate key specified")
            }
            key
        };

        let client_verifier = {
            let mut roots = RootCertStore::empty();
            read_certificates(&mut cas)
                .context("failed to read CA certificates")?
                .into_iter()
                .try_for_each(|ref cert| roots.add(cert))
                .context("failed to construct root certificate store")?;
            // TODO: Allow client certificates signed by unknown CAs.
            AllowAnyAnonymousOrAuthenticatedClient::new(roots)
        };

        ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(client_verifier))
            .with_single_cert(certs, key)
            .context("invalid server certificate key")
            .map(Self)
    }
}
