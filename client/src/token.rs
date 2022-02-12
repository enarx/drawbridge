use directories::ProjectDirs;
use std::{
    fs::{create_dir, read_to_string, remove_file, write},
    path::PathBuf,
};

use crate::error::CliError;

const TOKEN_FILE: &str = "token.txt";

fn token_path() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "Profian", "Drawbridge Client")
        .expect("ProjectDirs from directories crate");
    let dir = project_dirs.config_dir();

    if !dir.exists() {
        create_dir(dir).expect("create config directory");
    }

    let mut dir = dir.to_owned();
    dir.push(TOKEN_FILE);
    dir
}

pub struct Token;

impl Token {
    pub fn get() -> Result<String, CliError> {
        let token_path = token_path();

        if !token_path.exists() {
            Err(CliError::NotLoggedIn)
        } else {
            Ok(read_to_string(token_path).map_err(CliError::TokenIo)?)
        }
    }

    pub fn set(token: &str) -> Result<(), CliError> {
        write(token_path(), token).map_err(CliError::TokenIo)?;
        Ok(())
    }

    pub fn delete() -> Result<(), CliError> {
        let token_path = token_path();

        if token_path.exists() {
            remove_file(token_path).map_err(CliError::TokenIo)?;
        }

        Ok(())
    }
}
