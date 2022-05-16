// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use super::{Algorithm, Error, Reader, Verifier, Writer};

use std::collections::btree_map::IntoIter;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_byte::Bytes;
use serde::{Deserialize, Serialize};

#[cfg(all(feature = "headers", feature = "http"))]
use headers::{Error as HeadErr, Header, HeaderName, HeaderValue};

/// A set of hashes for the same contents
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct ContentDigest<H = Box<[u8]>>(BTreeMap<Algorithm, Bytes<H>>)
where
    H: AsRef<[u8]> + From<Vec<u8>>;

impl<H> ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    /// Creates a reader instance
    pub fn reader<T>(&self, reader: T) -> Reader<T> {
        Reader::new(reader, self.iter().map(|x| *x.0))
    }

    /// Creates a writer instance
    pub fn writer<T>(&self, writer: T) -> Writer<T> {
        Writer::new(writer, self.iter().map(|x| *x.0))
    }

    /// Creates a verifier instance
    pub fn verifier<T>(self, reader: T) -> Verifier<T, H> {
        Verifier::new(self.reader(reader), self)
    }
}

impl<H> From<BTreeMap<Algorithm, Bytes<H>>> for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    fn from(value: BTreeMap<Algorithm, Bytes<H>>) -> Self {
        Self(value)
    }
}

impl<H> Eq for ContentDigest<H> where H: AsRef<[u8]> + From<Vec<u8>> {}
impl<T, U> PartialEq<ContentDigest<U>> for ContentDigest<T>
where
    T: AsRef<[u8]> + From<Vec<u8>>,
    U: AsRef<[u8]> + From<Vec<u8>>,
{
    fn eq(&self, other: &ContentDigest<U>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        for (lhs, rhs) in self.0.iter().zip(other.0.iter()) {
            if lhs.0 != rhs.0 {
                return false;
            }

            if lhs.1.as_ref() != rhs.1.as_ref() {
                return false;
            }
        }

        true
    }
}

impl<H> Deref for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    type Target = BTreeMap<Algorithm, Bytes<H>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<H> DerefMut for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<H> std::fmt::Display for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut comma = "";

        for (algo, hash) in self.iter() {
            write!(f, "{}{}=:{}:", comma, algo, hash)?;
            comma = ",";
        }

        Ok(())
    }
}

impl<H> FromStr for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(',')
            .map(|s| {
                let (key, val) = s.split_once('=').ok_or(Error::MissingEq)?;
                let alg = key.parse()?;
                let b64 = val
                    .strip_prefix(':')
                    .and_then(|val| val.strip_suffix(':'))
                    .ok_or(Error::MissingColons)?
                    .parse()?;
                Ok((alg, b64))
            })
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

impl<H> IntoIterator for ContentDigest<H>
where
    H: AsRef<[u8]> + From<Vec<u8>>,
{
    type Item = (Algorithm, Bytes<H>);
    type IntoIter = IntoIter<Algorithm, Bytes<H>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(all(feature = "headers", feature = "http"))]
static CONTENT_DIGEST: HeaderName = HeaderName::from_static("content-digest");

#[cfg(all(feature = "headers", feature = "http"))]
impl<H> Header for ContentDigest<H>
where
    H: Default + AsRef<[u8]> + From<Vec<u8>>,
{
    fn name() -> &'static HeaderName {
        &CONTENT_DIGEST
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, HeadErr>
    where
        Self: Sized,
        I: Iterator<Item = &'i HeaderValue>,
    {
        let mut all = Self::default();

        for value in values {
            let digests: ContentDigest<H> = std::str::from_utf8(value.as_bytes())
                .map_err(|_| HeadErr::invalid())?
                .parse()
                .map_err(|_| HeadErr::invalid())?;

            for (algo, hash) in digests {
                all.insert(algo, hash);
            }
        }

        if all.is_empty() {
            return Err(HeadErr::invalid());
        }

        Ok(all)
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        let value = HeaderValue::from_str(&self.to_string()).unwrap();
        values.extend([value].into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn isomorphism() {
        const STR: &str = "sha-224=:CAj2TmDViXn8tnbJbsk4Jw3qQkRa7vzTpOb42w==:,sha-256=:LCa0a2j/xo/5m0U8HTBBNBNCLXBkg7+g+YpeiGJm564=:,sha-384=:mMEf/f3VQGdrGhN8saIrKnA1DJpEFx1rEYDGvly7LuP3nVMsih3Z7y6OCOdSo7q7:,sha-512=:9/u6bgY2+JDlb7vzKD5STG+jIErimDgtYkdB0NxmODJuKCxBvl5CVNiCB3LFUYosWowMf37aGVlKfrU5RT4e1w==:";
        assert_eq!(STR.parse::<ContentDigest>().unwrap().to_string(), STR);
    }
}
