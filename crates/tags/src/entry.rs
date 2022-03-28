// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: Apache-2.0

use io::Error;
use std::io;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use drawbridge_http::http::{Request, StatusCode};
use drawbridge_http::{async_trait, FromRequest};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct Entry(Vec<u8>);

impl Deref for Entry {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Entry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromStr for Entry {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Entry(s.as_bytes().to_vec()))
    }
}

#[async_trait]
impl FromRequest for Entry {
    type Error = StatusCode;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.body_string()
            .await
            .or(Err(StatusCode::BadRequest))?
            .parse()
            .or(Err(StatusCode::BadRequest))
    }
}

#[cfg(test)]
mod test {
    use crate::Entry;
    use std::ops::Deref;
    use std::str::FromStr;

    #[test]
    fn entry_test() {
        let test_string = "foo_bar_baz";
        let test_entry = Entry::from_str(test_string).unwrap();
        assert_eq!(test_string, test_entry);
        assert_eq!(test_entry.to_vec(), test_entry.deref().to_vec());
    }
}
