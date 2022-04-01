// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Algorithm, ContentDigest};

use std::task::Context;
use std::{pin::Pin, task::Poll};

use sha2::digest::DynDigest;
use tokio::io::{AsyncWrite, Result};

/// A hashing writer
///
/// This type wraps another writer and hashes the bytes as they are written.
pub struct Writer<T> {
    writer: T,
    digests: Vec<(Algorithm, Box<dyn DynDigest>)>,
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Writer<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf).map_ok(|n| {
            for digest in &mut self.digests {
                digest.1.update(&buf[..n]);
            }

            n
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.writer).poll_shutdown(cx)
    }
}

impl<T> Writer<T> {
    pub(crate) fn new(writer: T, digests: impl IntoIterator<Item = Algorithm>) -> Self {
        let digests = digests.into_iter().map(|a| (a, a.hasher())).collect();
        Writer { writer, digests }
    }

    /// Calculates the digests for all the bytes written so far.
    pub fn digests(&self) -> ContentDigest<Box<[u8]>> {
        let mut set = ContentDigest::default();

        for digest in &self.digests {
            set.insert(digest.0, digest.1.clone().finalize().into());
        }

        set
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{copy, sink};

    use super::*;

    #[tokio::test]
    async fn success() {
        const HASH: &str = "sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:";
        let set = HASH.parse::<ContentDigest>().unwrap();

        let mut writer = set.clone().writer(sink());
        copy(&mut &b"foo"[..], &mut writer).await.unwrap();
        assert_eq!(writer.digests(), set);
    }

    #[tokio::test]
    async fn failure() {
        const HASH: &str = "sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:";
        let set = HASH.parse::<ContentDigest>().unwrap();

        let mut writer = set.clone().writer(sink());
        copy(&mut &b"bar"[..], &mut writer).await.unwrap();
        assert_ne!(writer.digests(), set);
    }
}
