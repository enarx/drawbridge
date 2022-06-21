// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{Client, Result};

use std::io::{copy, Read, Write};
use std::str::FromStr;

use drawbridge_type::digest::{Algorithms, ContentDigest};
use drawbridge_type::Meta;

use anyhow::{anyhow, bail, Context};
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
pub struct Entity<'a> {
    client: &'a Client,
    path: String,
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

impl<'a> Entity<'a> {
    pub fn new(client: &'a Client) -> Self {
        Self {
            client,
            path: Default::default(),
        }
    }

    /// Returns a child [Entity] rooted at `path`.
    pub fn child(&self, path: &str) -> Self {
        Self {
            client: self.client,
            path: format!("{}/{}", self.path, path),
        }
    }

    pub(super) fn create_request(&self, hash: &ContentDigest, mime: &Mime) -> Result<Request> {
        let url = self.client.url(&self.path)?;
        Ok(self
            .client
            .inner
            .put(url.as_str())
            .set("Content-Digest", &hash.to_string())
            .set(CONTENT_TYPE.as_str(), mime.as_ref()))
    }

    pub(super) fn create_bytes(&self, mime: &Mime, data: impl AsRef<[u8]>) -> Result<bool> {
        let data = data.as_ref();
        let (n, hash) = Algorithms::default()
            .read_sync(&data[..])
            .context("failed to compute content digest")?;
        if n != data.len() as u64 {
            bail!(
                "invalid amount of bytes read, expected: {}, got {n}",
                data.len(),
            )
        }
        let res = self
            .create_request(&hash, &mime)?
            .send_bytes(&data)
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

    pub fn get(&self) -> Result<(u64, Mime, impl Read)> {
        let url = self.client.url(&self.path)?;
        let res = self
            .client
            .inner
            .get(url.as_str())
            .call()
            .map_err(parse_ureq_error)
            .context("GET request failed")?;

        let hash: ContentDigest = parse_header(&res, "Content-Digest")?;
        let mime = parse_header(&res, CONTENT_TYPE.as_str())?;
        let size = parse_header(&res, CONTENT_LENGTH.as_str())?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::OK) => Ok((size, mime, hash.verifier(res.into_reader().take(size)))),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get_to(&self, dst: &mut impl Write) -> Result<(u64, Mime)> {
        let (size, mime, mut rdr) = self.get()?;
        let n = copy(&mut rdr, dst)?;
        if n != size {
            bail!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                n,
            )
        }
        Ok((size, mime))
    }

    pub fn get_json<T>(&self) -> Result<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        let (_, _, rdr) = self.get()?;
        serde_json::from_reader(rdr).context("failed to decode JSON")
    }

    pub fn get_bytes(&self) -> Result<(Mime, Vec<u8>)> {
        let (size, mime, mut rdr) = self.get()?;
        let mut buf =
            Vec::with_capacity(size.try_into().context("failed to convert u64 to usize")?);
        let n = copy(&mut rdr, &mut buf).context("I/O failure")?;
        if n != size {
            bail!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                n,
            )
        };
        Ok((mime, buf))
    }

    pub fn get_string(&self) -> Result<(Mime, String)> {
        let (size, mime, mut rdr) = self.get()?;
        let size = size.try_into().context("failed to convert u64 to usize")?;
        let mut s = String::with_capacity(size);
        let n = rdr.read_to_string(&mut s).context("I/O failure")?;
        if n != size {
            bail!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                n,
            )
        };
        Ok((mime, s))
    }
}
