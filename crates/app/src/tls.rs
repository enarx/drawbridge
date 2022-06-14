// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::io::BufRead;
use std::ops::Deref;

use anyhow::{anyhow, bail, Context};
use rustls::cipher_suite::{
    TLS13_AES_128_GCM_SHA256, TLS13_AES_256_GCM_SHA384, TLS13_CHACHA20_POLY1305_SHA256,
};
use rustls::kx_group::{SECP256R1, SECP384R1, X25519};
use rustls::server::AllowAnyAuthenticatedClient;
use rustls::version::TLS13;
use rustls::{Certificate, PrivateKey, RootCertStore, ServerConfig};
use rustls_pemfile::Item::{ECKey, PKCS8Key, RSAKey, X509Certificate};

#[repr(transparent)]
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
                .ok_or(anyhow!("server certificate key missing"))
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
            AllowAnyAuthenticatedClient::new(roots)
        };

        // TODO: load policy from config.
        ServerConfig::builder()
            .with_cipher_suites(&[
                TLS13_AES_256_GCM_SHA384,
                TLS13_AES_128_GCM_SHA256,
                TLS13_CHACHA20_POLY1305_SHA256,
            ])
            .with_kx_groups(&[&X25519, &SECP384R1, &SECP256R1])
            .with_protocol_versions(&[&TLS13])?
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, key)
            .context("invalid server certificate key")
            .map(Self)
    }
}
