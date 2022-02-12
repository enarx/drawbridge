use drawbridge_common::{endpoint::PROTECTED, COOKIE_NAME};
use reqwest::Client;

use crate::{cli::Cli, error::CliError, token::Token};

pub async fn protected(cli: &Cli, client: &Client) -> Result<(), CliError> {
    let token = Token::get()?;
    let message = client
        .get(format!("http://{}{}", cli.server, PROTECTED))
        .header("Cookie", format!("{}={}", COOKIE_NAME, token))
        .send()
        .await
        .map_err(CliError::Reqwest)?
        .text()
        .await
        .map_err(CliError::Reqwest)?;
    println!("{}", message);
    Ok(())
}
