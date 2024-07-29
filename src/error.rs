use std::error::Error as StdError;
use std::fmt::Display;

/// a parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    //
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("errors not implemented")
    }
}

impl StdError for Error {}

pub type StdResult<T, E> = std::result::Result<T, E>;

/// a parsing result
pub type Result<T> = std::result::Result<T, Error>;
