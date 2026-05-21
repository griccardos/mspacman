use std::{error::Error, fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub enum AppError {
    Command(std::io::Error),     //error running command
    CommandNonZero(Option<i32>), //exit code is not zero
    String(std::string::FromUtf8Error),
    DateError(jiff::Error),
    Other(String),
}
impl Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Command(e) => write!(f, "Command Error: {}", e),
            AppError::CommandNonZero(e) => write!(f, "Command Error: {:?}", e),
            AppError::String(e) => write!(f, "String Conversion Error: {}", e),
            AppError::Other(e) => write!(f, "Error: {}", e),
            AppError::DateError(e) => write!(f, "DateError: {}", e),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Command(err)
    }
}

impl From<FromUtf8Error> for AppError {
    fn from(err: FromUtf8Error) -> Self {
        AppError::String(err)
    }
}
impl From<jiff::Error> for AppError {
    fn from(err: jiff::Error) -> Self {
        AppError::DateError(err)
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::Other(err)
    }
}
