// SPDX-License-Identifier: Apache-2.0

use super::{ContentDigest, Reader};

use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::AsyncRead;

/// A verifying reader
///
/// This type is exactly the same as [`Reader`](super::Reader) except that it
/// additionally verifies the expected hashes. When the end-of-file condition
/// is reached, if the actual hashes do not match the expected hashes, an error
/// is produced.
#[allow(missing_debug_implementations)] // Reader does not implement Debug
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

impl<T: io::Read, H> io::Read for Verifier<T, H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.reader.read(buf)? {
            0 if self.reader.digests() != self.hashes => {
                Err(Error::new(ErrorKind::InvalidData, "hash mismatch"))
            }
            n => Ok(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::io::{copy, sink};

    use super::*;

    #[async_std::test]
    async fn read_success() {
        let rdr = &b"foo"[..];
        let content_digest = "sha-224=:CAj2TmDViXn8tnbJbsk4Jw3qQkRa7vzTpOb42w==:,sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:,sha-384=:mMEf/f3VQGdrGhN8saIrKnA1DJpEFx1rEYDGvly7LuP3nVMsih3Z7y6OCOdSo7q7:,sha-512=:9/u6bgY2+JDlb7vzKD5STG+jIErimDgtYkdB0NxmODJuKCxBvl5CVNiCB3LFUYosWowMf37aGVlKfrU5RT4e1w==:"
                .parse::<ContentDigest>()
                .unwrap();

        assert_eq!(
            copy(&mut content_digest.clone().verifier(rdr), &mut sink())
                .await
                .unwrap(),
            "foo".len() as u64,
        );
        assert_eq!(
            std::io::copy(&mut content_digest.verifier(rdr), &mut std::io::sink()).unwrap(),
            "foo".len() as u64,
        );
    }

    #[async_std::test]
    async fn read_failure() {
        let rdr = &b"bar"[..];
        let content_digest = "sha-224=:CAj2TmDViXn8tnbJbsk4Jw3qQkRa7vzTpOb42w==:,sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:,sha-384=:mMEf/f3VQGdrGhN8saIrKnA1DJpEFx1rEYDGvly7LuP3nVMsih3Z7y6OCOdSo7q7:,sha-512=:9/u6bgY2+JDlb7vzKD5STG+jIErimDgtYkdB0NxmODJuKCxBvl5CVNiCB3LFUYosWowMf37aGVlKfrU5RT4e1w==:"
                .parse::<ContentDigest>()
                .unwrap();

        assert_eq!(
            copy(&mut content_digest.clone().verifier(rdr), &mut sink())
                .await
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidData,
        );
        assert_eq!(
            std::io::copy(&mut content_digest.verifier(rdr), &mut std::io::sink())
                .unwrap_err()
                .kind(),
            ErrorKind::InvalidData,
        );
    }
}
