use std::{error, fmt, io};

use reqwest::StatusCode;

#[derive(Debug)]
pub enum CliError {
    NotLoggedIn,
    UnexpectedStatus(StatusCode, String),
    Reqwest(reqwest::Error),
    TokenPrompt(io::Error),
    TokenIo(io::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CliError::NotLoggedIn => "Not logged in.".to_owned(),
                CliError::UnexpectedStatus(code, body) =>
                    format!("Unexpected status code: {}: {:#?}", code, body),
                CliError::Reqwest(e) => format!("{}", e),
                CliError::TokenPrompt(e) => format!("Failed to prompt for token: {}", e),
                CliError::TokenIo(e) => format!("Token IO failed: {}", e),
            }
        )
    }
}

impl error::Error for CliError {}
