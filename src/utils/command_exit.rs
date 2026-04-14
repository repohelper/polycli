use std::error::Error;
use std::fmt;

use anyhow::Result;

#[derive(Debug)]
pub struct CommandExitError {
    code: u8,
    message: String,
}

impl CommandExitError {
    #[must_use]
    pub fn new(code: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn code(&self) -> u8 {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CommandExitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl Error for CommandExitError {}

pub fn fail<T>(code: u8, message: impl Into<String>) -> Result<T> {
    Err(CommandExitError::new(code, message).into())
}
