// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

#![warn(rust_2018_idioms, unused_lifetimes, unused_qualifications, clippy::all)]
#![forbid(unsafe_code)]

mod hash;
mod reader;
mod writer;

pub use hash::{Error, Hash};
pub use reader::Reader;
pub use writer::Writer;

impl Hash {
    pub fn reader<T>(self, reader: T) -> Reader<T> {
        let inner = match self.0 {
            hash::Inner::Sha224(..) => reader::Inner::Sha224(Default::default()),
            hash::Inner::Sha256(..) => reader::Inner::Sha256(Default::default()),
            hash::Inner::Sha384(..) => reader::Inner::Sha384(Default::default()),
            hash::Inner::Sha512(..) => reader::Inner::Sha512(Default::default()),
        };

        Reader {
            reader,
            inner,
            hash: self,
        }
    }

    pub fn writer<T>(&self, writer: T) -> Writer<T> {
        let inner = match self.0 {
            hash::Inner::Sha224(..) => writer::Inner::Sha224(Default::default()),
            hash::Inner::Sha256(..) => writer::Inner::Sha256(Default::default()),
            hash::Inner::Sha384(..) => writer::Inner::Sha384(Default::default()),
            hash::Inner::Sha512(..) => writer::Inner::Sha512(Default::default()),
        };

        Writer { writer, inner }
    }
}

impl<T> Reader<T> {
    fn hash(&self) -> Hash {
        use hash::Buffer;
        use sha2::Digest;

        Hash(match &self.inner {
            reader::Inner::Sha224(h) => hash::Inner::Sha224(Buffer(h.clone().finalize())),
            reader::Inner::Sha256(h) => hash::Inner::Sha256(Buffer(h.clone().finalize())),
            reader::Inner::Sha384(h) => hash::Inner::Sha384(Buffer(h.clone().finalize())),
            reader::Inner::Sha512(h) => hash::Inner::Sha512(Buffer(h.clone().finalize())),
        })
    }
}

impl<T> Writer<T> {
    pub fn finish(self) -> Hash {
        use hash::Buffer;
        use sha2::Digest;

        Hash(match self.inner {
            writer::Inner::Sha224(h) => hash::Inner::Sha224(Buffer(h.finalize())),
            writer::Inner::Sha256(h) => hash::Inner::Sha256(Buffer(h.finalize())),
            writer::Inner::Sha384(h) => hash::Inner::Sha384(Buffer(h.finalize())),
            writer::Inner::Sha512(h) => hash::Inner::Sha512(Buffer(h.finalize())),
        })
    }
}
