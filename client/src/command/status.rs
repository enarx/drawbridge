use drawbridge_common::{endpoint::STATUS, COOKIE_NAME};
use reqwest::{Client, StatusCode};

use crate::{cli::Cli, error::CliError, token::Token};

pub async fn status(cli: &Cli, client: &Client) -> Result<(), CliError> {
    let mut request = client.get(format!("http://{}{}", cli.server, STATUS));

    if let Ok(token) = Token::get() {
        request = request.header("Cookie", format!("{}={}", COOKIE_NAME, token))
    }

    let response = request.send().await.map_err(CliError::Reqwest)?;
    let status = response.status();
    let text = response.text().await.map_err(CliError::Reqwest)?;

    match status {
        StatusCode::OK => {
            println!("{}", text);
        }
        StatusCode::FORBIDDEN => {
            eprintln!("{}", text);
        }
        code => {
            return Err(CliError::UnexpectedStatus(code, text));
        }
    }

    Ok(())
}
