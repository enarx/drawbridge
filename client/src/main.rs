mod cli;
mod command;
mod error;
mod token;

#[tokio::main]
async fn main() -> Result<(), error::CliError> {
    use clap::Parser;
    use cli::{Cli, Command};
    use command::{login, logout, protected, status};
    use reqwest::Client;

    let cli = Cli::parse();
    let client = Client::builder()
        .build()
        .map_err(error::CliError::Reqwest)?;

    match &cli.command {
        Command::Status => status(&cli, &client).await,
        Command::Login => login(&cli, &client).await,
        Command::Logout => logout(&cli, &client).await,
        Command::Protected => protected(&cli, &client).await,
    }
}
