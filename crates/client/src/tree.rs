// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Result, Tag};

use std::io::{copy, Read, Write};

use drawbridge_type::digest::ContentDigest;
use drawbridge_type::{Meta, TreePath};

use anyhow::{anyhow, bail};
use http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use http::StatusCode;
use mime::Mime;
use ureq::{Request, Response};

pub struct Node<'a> {
    pub(crate) tag: &'a Tag<'a>,
    pub(crate) path: &'a TreePath,
}

impl Node<'_> {
    fn create_request(&self, _hash: ContentDigest, mime: Mime) -> Result<Request> {
        let req = self
            .tag
            .repo
            .client
            .inner
            .put(
                self.tag
                    .repo
                    .client
                    .url
                    .join(&format!(
                        "{}/_tag/{}/tree{}",
                        self.tag.repo.name, self.tag.name, self.path,
                    ))?
                    .as_str(),
            )
            // TODO: Set Content-Digest
            // https://github.com/profianinc/drawbridge/issues/102
            .set(CONTENT_TYPE.as_str(), &mime.to_string());
        Ok(req)
    }

    pub fn create_from(&self, Meta { hash, size, mime }: Meta, rdr: impl Read) -> Result<bool> {
        let res = self
            .create_request(hash, mime)?
            .set(CONTENT_LENGTH.as_str(), &size.to_string())
            .send(rdr)?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::CREATED) => Ok(true),
            Ok(StatusCode::OK) => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn create_bytes(&self, mime: Mime, data: impl AsRef<[u8]>) -> Result<bool> {
        let res = self
            // TODO: Calculate and set Content-Digest
            .create_request(Default::default(), mime)?
            .send_bytes(data.as_ref())?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::CREATED) => Ok(true),
            Ok(StatusCode::OK) => Ok(false),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    fn get_response(&self) -> Result<(Meta, Response)> {
        let res = self
            .tag
            .repo
            .client
            .inner
            .get(
                self.tag
                    .repo
                    .client
                    .url
                    .join(&format!(
                        "{}/_tag/{}/tree{}",
                        self.tag.repo.name, self.tag.name, self.path,
                    ))?
                    .as_str(),
            )
            .call()?;
        let (hash, mime) = {
            (
                // TODO: figure out why CONTENT_DIGEST does not work
                // TODO: fix parsing of Content-Digest header
                //hdr.get("content-digest")
                //    .ok_or("missing Content-Digest header")?
                //    .to_str()?
                //    .parse()
                //    .map_err(|e| format!("failed to parse Content-Digest value: {}", e))?,
                // https://github.com/profianinc/drawbridge/issues/103
                Default::default(),
                res.header(CONTENT_TYPE.as_str())
                    .ok_or_else(|| anyhow!("missing Content-Type header"))?
                    .parse()?,
            )
        };
        let size = res
            .header(CONTENT_LENGTH.as_str())
            .ok_or_else(|| anyhow!("missing Content-Length header"))?
            .parse()?;
        match StatusCode::from_u16(res.status()) {
            Ok(StatusCode::OK) => Ok((Meta { hash, size, mime }, res)),
            _ => bail!("unexpected status code: {}", res.status()),
        }
    }

    pub fn get(&self) -> Result<(impl Read + Send, u64, Mime)> {
        let (
            Meta {
                // TODO: Validate digest
                // https://github.com/profianinc/drawbridge/issues/103
                hash: _,
                size,
                mime,
            },
            res,
        ) = self.get_response()?;
        // TODO: Wrap the reader and verify size
        // https://github.com/profianinc/drawbridge/issues/103
        Ok((res.into_reader().take(size), size, mime))
    }

    pub fn get_to(&self, dst: &mut impl Write) -> Result<(u64, Mime)> {
        let (mut r, size, mime) = self.get()?;
        let n = copy(&mut r, dst)?;
        if n != size {
            bail!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                n,
            )
        }
        Ok((size, mime))
    }

    pub fn get_bytes(&self) -> Result<(Vec<u8>, Mime)> {
        let mut v = vec![];
        self.get_to(&mut v).map(|(_, mime)| (v, mime))
    }

    pub fn get_string(&self) -> Result<(String, Mime)> {
        let (
            Meta {
                // TODO: Validate digest
                // https://github.com/profianinc/drawbridge/issues/103
                hash: _,
                size,
                mime,
            },
            res,
        ) = self.get_response()?;
        let s = res.into_string()?;
        if s.len() as u64 != size {
            bail!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                s.len(),
            )
        }
        Ok((s, mime))
    }
}
