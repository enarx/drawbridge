// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{ContentDigest, Reader};

use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::AsyncRead;

/// A verifying reader
///
/// This type is exactly the same as [`Reader`](crate::Reader) except that it
/// additionally verifies the expected hashes. When the end-of-file condition
/// is reached, if the actual hashes do not match the expected hashes, an error
/// is produced.
pub struct Verifier<T, H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    reader: Reader<T>,
    hashes: ContentDigest<H>,
}

#[allow(unsafe_code)]
unsafe impl<T, H> Sync for Verifier<T, H>
where
    T: Sync,
    H: Sync + AsRef<[u8]> + From<Vec<u8>>,
{
}

#[allow(unsafe_code)]
unsafe impl<T, H> Send for Verifier<T, H>
where
    T: Send,
    H: Send + AsRef<[u8]> + From<Vec<u8>>,
{
}

impl<T, H> Verifier<T, H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    pub(crate) fn new(reader: Reader<T>, hashes: ContentDigest<H>) -> Self {
        Self { reader, hashes }
    }

    pub fn digests(&self) -> ContentDigest<Box<[u8]>> {
        self.reader.digests()
    }
}

impl<T: Unpin, H> Unpin for Verifier<T, H> where H: AsRef<[u8]> + From<Vec<u8>> {}

impl<T: AsyncRead + Unpin, H> AsyncRead for Verifier<T, H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.reader)
            .poll_read(cx, buf)
            .map(|r| match r? {
                0 if self.reader.digests() != self.hashes => {
                    Err(Error::new(ErrorKind::InvalidData, "hash mismatch"))
                }
                n => Ok(n),
            })
    }
}

#[cfg(test)]
mod tests {
    use futures::io::{copy, sink};

    use super::*;

    #[tokio::test]
    async fn read_success() {
        const HASH: &str = "sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:";
        let mut reader = HASH.parse::<ContentDigest>().unwrap().verifier(&b"foo"[..]);
        copy(&mut reader, &mut sink()).await.unwrap();
    }

    #[tokio::test]
    async fn read_failure() {
        const HASH: &str = "sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:";
        let mut reader = HASH.parse::<ContentDigest>().unwrap().verifier(&b"bar"[..]);
        let err = copy(&mut reader, &mut sink()).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }
}
