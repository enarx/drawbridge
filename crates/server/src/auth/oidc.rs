// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{store::GetError, OidcConfig, Store, User};

use drawbridge_type::{UserContext, UserRecord};

use anyhow::{anyhow, bail, Context};
use axum::extract::rejection::{TypedHeaderRejection, TypedHeaderRejectionReason};
use axum::extract::{Extension, FromRequest, RequestParts};
use axum::headers::authorization::Bearer;
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, TypedHeader};
use jsonwebtoken::{
    decode, decode_header,
    jwk::{AlgorithmParameters, JwkSet},
    Algorithm, DecodingKey, Validation,
};
use openidconnect::core::CoreProviderMetadata;
use openidconnect::ureq::http_client;
use openidconnect::IssuerUrl;
use serde::{Deserialize, Deserializer};
use tracing::{error, info, trace, warn};

pub struct Verifier {
    keyset: HashMap<String, DecodingKey>,
    validator: Validation,
}

impl std::fmt::Debug for Verifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Verifier")
            .field("validator", &self.validator)
            .finish()
    }
}

#[derive(Clone, Debug, Deserialize)]
struct VerifiedInfo {
    #[serde(rename = "sub")]
    subject: String,
    #[serde(rename = "scope", deserialize_with = "deserialize_scopes")]
    scopes: HashSet<String>,
}

#[allow(single_use_lifetimes)]
fn deserialize_scopes<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    Ok(HashSet::from_iter(s.split(' ').map(|s| s.to_owned())))
}

impl Verifier {
    pub fn new(config: OidcConfig) -> Result<Self, anyhow::Error> {
        let mut validator = Validation::new(Algorithm::RS256);
        validator.set_audience(&[config.audience]);
        validator.set_issuer(&[config.issuer.as_str()]);
        validator.set_required_spec_claims(&["exp", "iat", "scope", "aud"]);
        validator.validate_exp = true;

        let oidc_md =
            CoreProviderMetadata::discover(&IssuerUrl::from_url(config.issuer), http_client)
                .context("failed to discover provider metadata")?;
        let jwks = oidc_md.jwks();
        let jwks = serde_json::to_string(&jwks).context("failed to serialize jwks")?;
        let keyset: JwkSet = serde_json::from_str(&jwks).context("failed to parse jwks")?;
        let keyset = keyset
            .keys
            .into_iter()
            .map(|jwk| {
                let kid = jwk.common.key_id.ok_or_else(|| anyhow!("missing kid"))?;
                let key = match jwk.algorithm {
                    AlgorithmParameters::RSA(ref rsa) => {
                        DecodingKey::from_rsa_components(&rsa.n, &rsa.e)
                            .context("Error creating DecodingKey")
                    }
                    _ => bail!("Unsupported algorithm encountered: {:?}", jwk.algorithm),
                }?;
                Ok((kid, key))
            })
            .collect::<Result<HashMap<String, DecodingKey>, anyhow::Error>>()
            .context("failed to parse jwks")?;

        Ok(Self { keyset, validator })
    }

    fn verify_token(&self, token: &str) -> Result<VerifiedInfo, anyhow::Error> {
        let header = decode_header(token).context("Error decoding header")?;
        let kid = match header.kid {
            Some(k) => k,
            None => bail!("Token doesn't have a `kid` header field"),
        };
        let key = self
            .keyset
            .get(&kid)
            .ok_or_else(|| anyhow!("No key found for kid: {}", kid))?;
        let decoded_token =
            decode::<VerifiedInfo>(token, key, &self.validator).context("Error decoding token")?;
        Ok(decoded_token.claims)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScopeContext {
    User,
    Repository,
    Tag,
}

impl std::fmt::Display for ScopeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeContext::User => write!(f, "drawbridge_users"),
            ScopeContext::Repository => write!(f, "drawbridge_repositories"),
            ScopeContext::Tag => write!(f, "drawbridge_tags"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScopeLevel {
    Read,
    Write,
}

impl std::fmt::Display for ScopeLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScopeLevel::Read => write!(f, "read"),
            ScopeLevel::Write => write!(f, "write"),
        }
    }
}

impl ScopeLevel {
    fn sufficient_levels(&self) -> &[&str] {
        match self {
            ScopeLevel::Read => &["read", "manage"],
            ScopeLevel::Write => &["write", "manage"],
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Claims(VerifiedInfo);

impl Claims {
    pub fn subject(&self) -> &str {
        &self.0.subject
    }

    fn check_scope(
        &self,
        context: ScopeContext,
        level: ScopeLevel,
    ) -> Result<(), (StatusCode, String)> {
        for level in level.sufficient_levels() {
            let scope = format!("{}:{}", level, context);
            if self.0.scopes.contains(&scope) {
                return Ok(());
            }
        }
        Err((
            StatusCode::UNAUTHORIZED,
            format!(
                "Token is missing a scope for level {}, context {}",
                level, context
            ),
        ))
    }

    /// Asserts that the token has a scope that satisfies the given context and level.
    #[allow(clippy::result_large_err)]
    pub fn assert_scope(
        &self,
        context: ScopeContext,
        level: ScopeLevel,
    ) -> Result<(), impl IntoResponse> {
        self.check_scope(context, level)
            .map_err(|e| e.into_response())
    }

    /// Assert that the client is the user identified by `cx`, and that the token has a scope that
    /// satisfies the given context and level.
    pub async fn assert_user<'a>(
        &self,
        store: &'a Store,
        cx: &UserContext,
        scope_context: ScopeContext,
        scope_level: ScopeLevel,
    ) -> Result<User<'a>, impl IntoResponse> {
        let subj = self.subject();
        let oidc_record = UserRecord {
            subject: subj.to_string(),
        };

        let user = store.user(cx);
        let owner_record: UserRecord = user.get_content_json().await.map_err(|e|{
            match e {
                GetError::NotFound => (StatusCode::UNAUTHORIZED, format!("User `{cx}` not found")).into_response(),
                _ => {
            warn!(target: "app::auth::oidc", ?oidc_record, error = ?e, "failed to get user by OpenID Connect subject");
e.into_response()
                },
            }})?;

        if oidc_record != owner_record {
            warn!(target: "app::auth::oidc", ?oidc_record, user = ?cx, ?owner_record, "User access not authorized");
            return Err((
                StatusCode::UNAUTHORIZED,
                format!("You are logged in as `{subj}`, and not authorized for user `{cx}`"),
            )
                .into_response());
        }

        self.check_scope(scope_context, scope_level)
            .map_err(|e| e.into_response())?;

        Ok(user)
    }
}

#[async_trait]
impl<B: Send> FromRequest<B> for Claims {
    type Rejection = Response;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization::<Bearer>(token)) =
            req.extract()
                .await
                .map_err(|e: TypedHeaderRejection| match e.reason() {
                    TypedHeaderRejectionReason::Missing => {
                        (StatusCode::UNAUTHORIZED, "Bearer token header missing").into_response()
                    }
                    _ => e.into_response(),
                })?;
        warn!(target: "app::auth::oidc", ?token, "got token");

        let Extension(verifier) = req
            .extract::<Extension<Arc<Verifier>>>()
            .await
            .map_err(|e| {
                error!(target: "app::auth::oidc", "OpenID Connect verifier extension missing");
                e.into_response()
            })?;

        trace!(target: "app:auth::oidc", "verifying token");

        let claims = verifier
            .verify_token(token.token())
            .map_err(|e| {
                error!(target: "app::auth::oidc", error = ?e, "failed to verify token");
                (StatusCode::UNAUTHORIZED, "Invalid token provided").into_response()
            })
            .map(Self);
        info!(target: "app::auth::oidc", ?claims, "verified token");
        claims
    }
}
