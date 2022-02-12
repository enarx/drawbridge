use drawbridge_common::endpoint::GITHUB;
use reqwest::Client;
use rpassword::prompt_password_stdout;

use crate::{cli::Cli, error::CliError, token::Token};

pub async fn login(cli: &Cli, _client: &Client) -> Result<(), CliError> {
    let url = format!("http://{}{}", cli.server, GITHUB);

    let _ = open::that_in_background(&url);
    println!("Login with {}", url);
    let token = prompt_password_stdout("Token: ").map_err(CliError::TokenPrompt)?;
    Token::set(&token)
}
