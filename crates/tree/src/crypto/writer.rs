// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use std::pin::Pin;
use std::task::Context;

use async_std::io::{Result, Write};

use sha2::digest::Digest;
use sha2::{Sha224, Sha256, Sha384, Sha512};

pub(super) enum Inner {
    Sha224(Sha224),
    Sha256(Sha256),
    Sha384(Sha384),
    Sha512(Sha512),
}

pub struct Writer<T> {
    pub(super) writer: T,
    pub(super) inner: Inner,
}

impl<T: Write + Unpin> Write for Writer<T> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf).map_ok(|n| {
            match &mut self.inner {
                Inner::Sha224(h) => h.update(&buf[..n]),
                Inner::Sha256(h) => h.update(&buf[..n]),
                Inner::Sha384(h) => h.update(&buf[..n]),
                Inner::Sha512(h) => h.update(&buf[..n]),
            };
            n
        })
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Result<()>> {
        Pin::new(&mut self.writer).poll_close(cx)
    }
}
