use clap::{AppSettings, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
/// The drawbridge CLI.
pub struct Cli {
    #[clap(short, long, default_value = "0.0.0.0:3000")]
    /// The drawbridge server to interface with.
    pub server: String,
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Check if you are logged in and other stuff.
    Status,
    /// Login using OAuth.
    Login,
    /// Logout using OAuth.
    Logout,
    /// Access a protected endpoint.
    Protected,
}
