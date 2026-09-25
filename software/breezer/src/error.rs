//! Central error type for Breezer.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Network / HTTP failure.
    Http(reqwest::Error),
    /// Serialization failure.
    Json(serde_json::Error),
    /// I/O failure.
    Io(std::io::Error),
    /// Slint platform error.
    Slint(slint::PlatformError),
    /// Deezer authentication problem (bad/expired ARL, etc.).
    Auth(String),
    /// Something is not implemented yet.
    Unimplemented(String),
    /// Generic, message-based error.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(e) => write!(f, "network error: {e}"),
            Error::Json(e) => write!(f, "serialization error: {e}"),
            Error::Io(e) => write!(f, "I/O error: {e}"),
            Error::Slint(e) => write!(f, "UI error: {e}"),
            Error::Auth(m) => write!(f, "authentication error: {m}"),
            Error::Unimplemented(m) => write!(f, "not implemented yet: {m}"),
            Error::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http(e) => Some(e),
            Error::Json(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Slint(_) | Error::Auth(_) | Error::Unimplemented(_) | Error::Other(_) => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<slint::PlatformError> for Error {
    fn from(e: slint::PlatformError) -> Self {
        Error::Slint(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;