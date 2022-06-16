// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod entity;
mod repo;
mod tag;
mod tree;
mod user;

pub use entity::*;
pub use repo::*;
pub use tag::*;
pub use tree::*;
pub use user::*;

pub use drawbridge_type as types;

pub use anyhow::{Context, Result};
pub use mime;
pub use url::Url;

use std::sync::Arc;

use drawbridge_type::{RepositoryContext, TagContext, TreeContext, UserContext};

use rustls::cipher_suite::{
    TLS13_AES_128_GCM_SHA256, TLS13_AES_256_GCM_SHA384, TLS13_CHACHA20_POLY1305_SHA256,
};
use rustls::kx_group::{SECP256R1, SECP384R1, X25519};
use rustls::version::TLS13;
use rustls::{Certificate, OwnedTrustAnchor, PrivateKey, RootCertStore};

#[derive(Clone, Debug)]
pub struct Client {
    inner: ureq::Agent,
    root: Url,
}

impl Client {
    pub fn builder(url: Url) -> ClientBuilder {
        ClientBuilder::new(url)
    }

    pub fn new(url: Url) -> Result<Self> {
        Self::builder(url).build()
    }

    fn url(&self, path: &str) -> Result<Url> {
        self.root.join(path).context("failed to construct URL")
    }

    pub fn user(&self, UserContext { name }: &UserContext) -> User<'_> {
        User::new(Entity::new(self), name)
    }

    pub fn repository<'a>(
        &'a self,
        RepositoryContext { owner, name }: &'a RepositoryContext,
    ) -> Repository<'_> {
        self.user(owner).repository(name)
    }

    pub fn tag<'a>(&'a self, TagContext { repository, name }: &'a TagContext) -> Tag<'_> {
        self.repository(repository).tag(name)
    }

    pub fn tree<'a>(&'a self, TreeContext { tag, path }: &'a TreeContext) -> Node<'_> {
        self.tag(tag).path(path)
    }
}

#[derive(Clone, Debug)]
pub struct ClientBuilder {
    url: Url,
    credentials: Option<(Vec<Certificate>, PrivateKey)>,
    roots: Option<RootCertStore>,
}

impl ClientBuilder {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            credentials: None,
            roots: None,
        }
    }

    pub fn credentials(self, cert: Vec<Certificate>, key: PrivateKey) -> Self {
        Self {
            credentials: Some((cert, key)),
            ..self
        }
    }

    pub fn roots(self, roots: RootCertStore) -> Self {
        Self {
            roots: Some(roots),
            ..self
        }
    }

    pub fn build(self) -> Result<Client> {
        let tls = rustls::ClientConfig::builder()
            .with_cipher_suites(&[
                TLS13_AES_256_GCM_SHA384,
                TLS13_AES_128_GCM_SHA256,
                TLS13_CHACHA20_POLY1305_SHA256,
            ])
            .with_kx_groups(&[&X25519, &SECP384R1, &SECP256R1])
            .with_protocol_versions(&[&TLS13])?
            .with_root_certificates(if let Some(roots) = self.roots {
                roots
            } else {
                let mut root_store = RootCertStore::empty();
                root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
                    |ta| {
                        OwnedTrustAnchor::from_subject_spki_name_constraints(
                            ta.subject,
                            ta.spki,
                            ta.name_constraints,
                        )
                    },
                ));
                root_store
            });
        let tls = if let Some((cert, key)) = self.credentials {
            tls.with_single_cert(cert, key)?
        } else {
            tls.with_no_client_auth()
        };

        Ok(Client {
            inner: ureq::AgentBuilder::new().tls_config(Arc::new(tls)).build(),
            root: self.url,
        })
    }
}
