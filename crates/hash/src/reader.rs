// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::Hash;

use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::Context;

use futures::AsyncRead;
use sha2::digest::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};

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

impl<T: AsyncRead + Unpin> AsyncRead for Reader<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
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
    use super::*;

    use futures::io::{copy, sink};

    #[tokio::test]
    async fn read_success() {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";
        let hash: Hash = HASH.parse().unwrap();
        let mut read = hash.reader(&b"foo"[..]);
        copy(&mut read, &mut sink()).await.unwrap();
    }

    #[tokio::test]
    async fn read_failure() {
        const HASH: &str = "sha256:LCa0a2j_xo_5m0U8HTBBNBNCLXBkg7-g-YpeiGJm564";
        let hash: Hash = HASH.parse().unwrap();
        let mut read = hash.reader(&b"bar"[..]);
        match copy(&mut read, &mut sink()).await {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidData),
            Ok(..) => panic!("unexpected success"),
        }
    }

    #[tokio::test]
    async fn meta_hash() {
        // printf "sha384:%s" $(printf '%s' '{"contentLength":42,"contentType":"text/plain","eTag":"sha384:mqVuAfXRKap7bdgcCY5uykM6-R9GqQ8K_uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC"}' | openssl dgst -sha384 -binary | openssl base64 -A | tr '/' '_' | tr '+' '-')
        const HASH: &str =
            "sha384:hF8t6NZNTsnhhFcVjYeIc1kkavoZ3HIaWI_a7Z-l1odHq32xX3YaeFPyo4Jjf6Be";
        let hash: Hash = HASH.parse().unwrap();
        let meta = r#"{"contentLength":42,"contentType":"text/plain","eTag":"sha384:mqVuAfXRKap7bdgcCY5uykM6-R9GqQ8K_uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC"}"#;
        let mut read = hash.reader(meta.as_bytes());
        copy(&mut read, &mut sink()).await.unwrap();
    }
}
