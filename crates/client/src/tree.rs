// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::Tag;

use std::error::Error;
use std::io::Write;

use drawbridge_type::{Meta, TreePath};

use bytes::Bytes;
use mime::Mime;
use reqwest::blocking::{Body, Response};
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

pub struct Node<'a> {
    pub(crate) tag: &'a Tag<'a>,
    pub(crate) path: &'a TreePath,
}

impl Node<'_> {
    pub fn create(&self, mime: Mime, body: impl Into<Body>) -> Result<bool, Box<dyn Error>> {
        let res = self
            .tag
            .repo
            .client
            .inner
            .put(self.tag.repo.client.url.join(&format!(
                "{}/_tag/{}/tree{}",
                self.tag.repo.name, self.tag.name, self.path,
            ))?)
            .header(CONTENT_TYPE, mime.to_string())
            // TODO: Calculate and set Content-Digest
            .body(body)
            .send()?
            .error_for_status()?;
        match res.status() {
            StatusCode::CREATED => Ok(true),
            StatusCode::OK => Ok(false),
            _ => Err("unexpected status code")?,
        }
    }

    fn get(&self) -> Result<(Meta, Response), Box<dyn Error>> {
        let res = self
            .tag
            .repo
            .client
            .inner
            .get(self.tag.repo.client.url.join(&format!(
                "{}/_tag/{}/tree{}",
                self.tag.repo.name, self.tag.name, self.path,
            ))?)
            .send()?
            .error_for_status()?;
        let (hash, mime) = {
            let hdr = res.headers();
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
                hdr.get(CONTENT_TYPE)
                    .ok_or("missing Content-Type header")?
                    .to_str()?
                    .parse()?,
            )
        };
        let size = res
            .content_length()
            .ok_or("missing Content-Length header")?;
        match res.status() {
            StatusCode::OK => Ok((Meta { hash, size, mime }, res)),
            _ => Err("unexpected status code")?,
        }
    }

    pub fn get_to(&self, dst: &mut impl Write) -> Result<(u64, Mime), Box<dyn Error>> {
        let (
            Meta {
                // TODO: Validate digest
                // https://github.com/profianinc/drawbridge/issues/103
                hash: _,
                size,
                mime,
            },
            mut res,
        ) = self.get()?;
        let n = res.copy_to(dst)?;
        if n != size {
            Err(format!(
                "invalid amount of bytes read, expected {}, read {}",
                size, n,
            ))?
        }
        Ok((size, mime))
    }

    pub fn get_bytes(&self) -> Result<(Bytes, Mime), Box<dyn Error>> {
        let (
            Meta {
                // TODO: Validate digest
                // https://github.com/profianinc/drawbridge/issues/103
                hash: _,
                size,
                mime,
            },
            res,
        ) = self.get()?;
        let b = res.bytes()?;
        if b.len() as u64 != size {
            Err(format!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                b.len(),
            ))?
        }
        Ok((b, mime))
    }

    pub fn get_text(&self) -> Result<(String, Mime), Box<dyn Error>> {
        let (
            Meta {
                // TODO: Validate digest
                // https://github.com/profianinc/drawbridge/issues/103
                hash: _,
                size,
                mime,
            },
            res,
        ) = self.get()?;
        let s = res.text()?;
        if s.len() as u64 != size {
            Err(format!(
                "invalid amount of bytes read, expected {}, read {}",
                size,
                s.len(),
            ))?
        }
        Ok((s, mime))
    }
}
