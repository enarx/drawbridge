// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::io::{Error, ErrorKind};
use std::pin::Pin;
use std::task::Context;

use async_std::io::{Read, Result};

use sha2::digest::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};

use super::Hash;

pub(super) enum Inner {
    Sha224(Sha224),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

pub struct Reader<T> {
    pub(super) reader: T,
    pub(super) inner: Inner,
    pub(super) hash: Hash,
}

impl<T: Read + Unpin> Read for Reader<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<Result<usize>> {
        Pin::new(&mut self.reader).poll_read(cx, buf).map(|r| {
            let n = r?;

            match &mut self.inner {
                Inner::Sha224(h) => h.update(&buf[..n]),
                Inner::Sha256(h) => h.update(&buf[..n]),
                Inner::Sha384(h) => h.update(&buf[..n]),
                Inner::Sha512(h) => h.update(&buf[..n]),
            };

            // On EOF, validate the hash.
            if !buf.is_empty() && n == 0 && self.hash() != self.hash {
                Err(Error::new(ErrorKind::InvalidData, "hash mismatch"))
            } else {
                Ok(n)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use async_std::io::{copy, sink};

    use super::*;

    #[async_std::test]
    async fn read_success() {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";
        let hash: Hash = HASH.parse().unwrap();
        let mut read = hash.reader(&b"foo"[..]);
        copy(&mut read, &mut sink()).await.unwrap();
    }

    #[async_std::test]
    async fn read_failure() {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";
        let hash: Hash = HASH.parse().unwrap();
        let mut read = hash.reader(&b"bar"[..]);
        match copy(&mut read, &mut sink()).await {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidData),
            Ok(..) => panic!("unexpected success"),
        }
    }
}
