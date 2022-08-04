// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{scope, Client, Result, Scope};

use std::io::{copy, Read, Write};
use std::marker::PhantomData;
use std::str::FromStr;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::Meta;

use anyhow::{anyhow, bail, ensure, Context};
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::StatusCode;
use mime::Mime;
use ureq::serde::{Deserialize, Serialize};
use ureq::{Request, Response};

fn parse_header<T>(req: &Response, name: &str) -> Result<T>
where
    T: FromStr,
    T::Err: 'static + Sync + Send + std::error::Error,
{
    req.header(name)
        .ok_or_else(|| anyhow!("missing `{name}` header"))?
        .parse()
        .context(format!("failed to parse `{name}` header"))
}

#[derive(Clone, Debug)]
pub struct Entity<'a, C: Scope, E: Scope> {
    client: &'a Client<C>,
    path: String,
    phantom: PhantomData<E>,
}

fn parse_ureq_error(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(code, msg) => match msg.into_string() {
            Ok(msg) if !msg.is_empty() => {
                anyhow!(msg).context(format!("request failed with status code `{code}`"))
            }
            _ => anyhow!("request failed with status code `{code}`"),
        },

        ureq::Error::Transport(e) => anyhow::Error::new(e).context("transport layer failure"),
    }
}

impl<'a, C: Scope> Entity<'a, C, C> {
    pub fn new(client: &'a Client<C>) -> Self {
        Self {
            client,
            path: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'a> Entity<'a, scope::Unknown, scope::Unknown> {
    /// Changes the scope of the entity.
    pub fn scope<O: Scope>(self) -> Entity<'a, scope::Unknown, O> {
        Entity {
            client: self.client,
            path: self.path,
            phantom: PhantomData,
        }
    }
}

impl<'a, C: Scope, E: Scope> Entity<'a, C, E> {
    /// Returns a child [Entity] rooted at `path`.
    pub fn child<O: Scope>(&self, path: &str) -> Entity<'a, C, O> {
        Entity {
            client: self.client,
            path: format!("{}/{}", self.path, path),
            phantom: PhantomData,
        }
    }

    pub(super) fn create_request(&self, hash: &ContentDigest, mime: &Mime) -> Result<Request> {
        let token = self.client.token.as_ref().ok_or_else(|| {
            anyhow!("endpoint requires authorization, but no token was configured")
        })?;
        let url = self.client.url(&self.path)?;
        Ok(self
            .client
            .inner
            .put(url.as_str())
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Digest", &hash.to_string())
            .set(CONTENT_TYPE.as_str(), mime.as_ref()))
    }

    pub(super) fn create_bytes(&self, mime: &Mime, data: impl AsRef<[u8]>) -> Result<bool> {
        let data = data.as_ref();
        let (n, hash) = Algorithms::default()
            .read_sync(data)
            .context("failed to compute content digest")?;
        ensure!(
            n == data.len() as u64,
            "invalid amount of bytes read, expected {}, read {n}",
            data.len(),
        );
        let res = self
            .create_request(&hash, mime)?
            .send_bytes(data)
            .map_err(parse_ureq_error)?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::CREATED) => Ok(true),
            Ok(StatusCode::OK) => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub(super) fn create_json(&self, mime: &Mime, val: &impl Serialize) -> Result<bool> {
        let buf = serde_json::to_vec(val).context("failed to encode value to JSON")?;
        self.create_bytes(mime, buf)
    }

    pub(super) fn create_from(
        &self,
        Meta { hash, size, mime }: &Meta,
        rdr: impl Read,
    ) -> Result<bool> {
        let res = self
            .create_request(hash, mime)?
            .set(CONTENT_LENGTH.as_str(), &size.to_string())
            .send(rdr)
            .map_err(parse_ureq_error)?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::CREATED) => Ok(true),
            Ok(StatusCode::OK) => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get(&self, limit: u64) -> Result<(Meta, impl Read)> {
        let url = self.client.url(&self.path)?;
        let mut req = self.client.inner.get(url.as_str());
        if let Some(ref token) = self.client.token {
            req = req.set("Authorization", &format!("Bearer {token}"))
        }
        let res = req
            .call()
            .map_err(parse_ureq_error)
            .context("GET request failed")?;

        let hash: ContentDigest = parse_header(&res, "Content-Digest")?;
        let mime = parse_header(&res, CONTENT_TYPE.as_str())?;
        let size = parse_header(&res, CONTENT_LENGTH.as_str())?;
        ensure!(
            size <= limit,
            "response size of `{size}` exceeds the limit of `{limit}`"
        );
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::OK) => Ok((
                Meta {
                    hash: hash.clone(),
                    size,
                    mime,
                },
                hash.verifier(res.into_reader().take(size)),
            )),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get_to(&self, limit: u64, dst: &mut impl Write) -> Result<Meta> {
        let (meta @ Meta { size, .. }, mut rdr) = self.get(limit)?;
        let n = copy(&mut rdr, dst)?;
        ensure!(
            n == size,
            "invalid amount of bytes read, expected {size}, read {n}"
        );
        Ok(meta)
    }

    pub fn get_json<T>(&self, limit: u64) -> Result<(Meta, T)>
    where
        for<'de> T: Deserialize<'de>,
    {
        let (meta, rdr) = self.get(limit)?;
        let v = serde_json::from_reader(rdr).context("failed to decode JSON")?;
        Ok((meta, v))
    }

    pub fn get_bytes(&self, limit: u64) -> Result<(Meta, Vec<u8>)> {
        let (meta @ Meta { size, .. }, rdr) = self.get(limit)?;
        let mut rdr = rdr.take(limit);
        let mut buf =
            Vec::with_capacity(size.try_into().context("failed to convert u64 to usize")?);
        let n = copy(&mut rdr, &mut buf).context("I/O failure")?;
        ensure!(
            n == size,
            "invalid amount of bytes read, expected {size}, read {n}"
        );
        Ok((meta, buf))
    }

    pub fn get_string(&self, limit: u64) -> Result<(Meta, String)> {
        let (meta @ Meta { size, .. }, mut rdr) = self.get(limit)?;
        let size = size.try_into().context("failed to convert u64 to usize")?;
        let mut s = String::with_capacity(size);
        let n = rdr.read_to_string(&mut s).context("I/O failure")?;
        ensure!(
            n == size,
            "invalid amount of bytes read, expected {size}, read {n}"
        );
        Ok((meta, s))
    }
}
