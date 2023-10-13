// SPDX-License-Identifier: Apache-2.0

mod algorithm;
mod algorithms;
mod digests;
mod reader;
mod verifier;
mod writer;

pub use algorithm::Algorithm;
pub use algorithms::Algorithms;
pub use digests::ContentDigest;
pub use reader::Reader;
pub use verifier::Verifier;
pub use writer::Writer;

/// Parsing error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    MissingEq,
    MissingColons,
    UnknownAlgorithm,
    Decode(base64::DecodeError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(e) => e.fmt(f),
            Self::MissingEq => f.write_str("missing equals"),
            Self::MissingColons => f.write_str("missing colons"),
            Self::UnknownAlgorithm => f.write_str("unknown algorithm"),
        }
    }
}

impl std::error::Error for Error {}

impl From<base64::DecodeError> for Error {
    fn from(value: base64::DecodeError) -> Self {
        Self::Decode(value)
    }
}
