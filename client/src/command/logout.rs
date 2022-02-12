use reqwest::Client;

use crate::{cli::Cli, error::CliError, token::Token};

pub async fn logout(_cli: &Cli, _client: &Client) -> Result<(), CliError> {
    match Token::get() {
        Err(CliError::NotLoggedIn) => println!("Already logged out."),
        Err(e) => return Err(e),
        Ok(_) => {
            Token::delete()?;
            println!("Logged out.");
        }
    }

    Ok(())
}
