// SPDX-FileCopyrightText: 2022 Profian Inc. <opensource@profian.com>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::GetError;

use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotFound,
    Internal(anyhow::Error),
}

impl Error {
    pub fn into_get_error(self) -> GetError<Self> {
        match self {
            Error::NotFound => GetError::NotFound,
            e => GetError::Internal(e),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
