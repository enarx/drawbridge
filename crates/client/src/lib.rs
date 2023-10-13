// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(
    clippy::all,
    absolute_paths_not_starting_with_crate,
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    noop_method_call,
    rust_2018_compatibility,
    rust_2018_idioms,
    rust_2021_compatibility,
    single_use_lifetimes,
    trivial_bounds,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_code,
    unreachable_patterns,
    unreachable_pub,
    unstable_features,
    unused,
    unused_crate_dependencies,
    unused_import_braces,
    unused_lifetimes,
    unused_results,
    variant_size_differences
)]

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

pub use drawbridge_jose as jose;
pub use drawbridge_type as types;

pub use anyhow::{Context, Result};
pub use mime;
pub use url::Url;

use std::marker::PhantomData;
use std::sync::Arc;

use drawbridge_type::{RepositoryContext, TagContext, TreeContext, UserContext};

use rustls::{Certificate, OwnedTrustAnchor, PrivateKey, RootCertStore};

/// API version used by this crate
pub const API_VERSION: &str = "0.1.0";

mod private {
    pub trait Scope: Copy + Clone {}
}

pub trait Scope: private::Scope {}

impl<T> Scope for T where T: private::Scope {}

pub mod scope {
    use super::private::Scope;

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Root;
    impl Scope for Root {}

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct User;
    impl Scope for User {}

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Repository;
    impl Scope for Repository {}

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Tag;
    impl Scope for Tag {}

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Node;
    impl Scope for Node {}

    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    pub struct Unknown;
    impl Scope for Unknown {}
}

#[derive(Clone, Debug)]
pub struct Client<S = scope::Root> {
    inner: ureq::Agent,
    root: Url,
    token: Option<String>,
    scope: PhantomData<S>,
}

impl<S: Scope> Client<S> {
    pub fn builder(url: Url) -> ClientBuilder<S> {
        ClientBuilder::new(url)
    }

    pub fn new_scoped(url: Url) -> Result<Self> {
        Self::builder(url).build_scoped()
    }

    fn url(&self, path: &str) -> Result<Url> {
        format!("{}{path}", self.root)
            .parse()
            .context("failed to construct URL")
    }
}

impl Client<scope::Root> {
    pub fn new(url: Url) -> Result<Self> {
        Self::builder(url).build()
    }

    pub fn user(&self, UserContext { name }: &UserContext) -> User<'_, scope::Root> {
        User::new(Entity::new(self), name)
    }

    pub fn repository<'a>(
        &'a self,
        RepositoryContext { owner, name }: &'a RepositoryContext,
    ) -> Repository<'_, scope::Root> {
        self.user(owner).repository(name)
    }

    pub fn tag<'a>(
        &'a self,
        TagContext { repository, name }: &'a TagContext,
    ) -> Tag<'_, scope::Root> {
        self.repository(repository).tag(name)
    }

    pub fn tree<'a>(&'a self, TreeContext { tag, path }: &'a TreeContext) -> Node<'_, scope::Root> {
        self.tag(tag).path(path)
    }
}

#[derive(Clone, Debug)]
pub struct ClientBuilder<S: Scope = scope::Root> {
    url: Url,
    credentials: Option<(Vec<Certificate>, PrivateKey)>,
    roots: Option<RootCertStore>,
    token: Option<String>,
    user_agent: Option<String>,
    scope: PhantomData<S>,
}

impl<S: Scope> ClientBuilder<S> {
    pub fn new(url: Url) -> Self {
        Self {
            url,
            credentials: None,
            roots: None,
            token: None,
            user_agent: None,
            scope: PhantomData,
        }
    }

    pub fn user_agent(self, user_agent: impl Into<String>) -> Self {
        Self {
            user_agent: Some(user_agent.into()),
            ..self
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

    pub fn token(self, token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..self
        }
    }

    pub fn build_scoped(self) -> Result<Client<S>> {
        let tls = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(if let Some(roots) = self.roots {
                roots
            } else {
                let mut root_store = RootCertStore::empty();
                root_store.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                    OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                }));
                root_store
            });
        let tls = if let Some((cert, key)) = self.credentials {
            tls.with_client_auth_cert(cert, key)?
        } else {
            tls.with_no_client_auth()
        };

        let user_agent = self.user_agent.unwrap_or_else(|| {
            format!("{}/{}", env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"))
        });

        Ok(Client {
            inner: ureq::AgentBuilder::new()
                .tls_config(Arc::new(tls))
                .user_agent(&user_agent)
                .build(),
            root: self.url,
            token: self.token,
            scope: self.scope,
        })
    }
}

impl ClientBuilder<scope::Root> {
    pub fn build(self) -> Result<Client<scope::Root>> {
        let url = self
            .url
            .join(&format!("api/v{API_VERSION}"))
            .context("failed to construct URL")?;
        Self { url, ..self }.build_scoped()
    }
}
