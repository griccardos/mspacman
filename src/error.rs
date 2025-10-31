use std::{error::Error, fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub enum AppError {
    Command(std::io::Error),
    String(std::string::FromUtf8Error),
}
impl Error for AppError {}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
