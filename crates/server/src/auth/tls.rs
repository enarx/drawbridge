// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::io::BufRead;
use std::ops::Deref;

use anyhow::{bail, Context};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use rustls_pemfile::Item::{Pkcs1Key, Pkcs8Key, Sec1Key, X509Certificate};
use rustls_pki_types::CertificateDer;
use rustls_pki_types::PrivateKeyDer;

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

fn read_certificates(mut rd: impl BufRead) -> anyhow::Result<Vec<CertificateDer<'static>>> {
    rustls_pemfile::read_all(&mut rd)
        .map(|item| match item? {
            X509Certificate(buf) => Ok(buf),
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
            if let Some(key) = rustls_pemfile::read_all(&mut key).next() {
                match key? {
                    Pkcs1Key(inner) => PrivateKeyDer::from(inner),
                    Pkcs8Key(inner) => PrivateKeyDer::from(inner),
                    Sec1Key(inner) => PrivateKeyDer::from(inner),
                    _ => {
                        bail!("Unexpected key type found");
                    }
                }
            } else {
                bail!("No key found")
            }
        };

        let client_verifier = {
            let mut roots = RootCertStore::empty();
            read_certificates(&mut cas)
                .context("failed to read CA certificates")?
                .into_iter()
                .try_for_each(|cert| roots.add(cert))
                .context("failed to construct root certificate store")?;
            // TODO: Allow client certificates signed by unknown CAs.
            WebPkiClientVerifier::builder(roots.into())
                .allow_unauthenticated()
                .build()?
        };

        ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, key)
            .context("invalid server certificate key")
            .map(Self)
    }
}
