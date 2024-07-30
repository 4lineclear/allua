use std::error::Error as StdError;
use std::fmt::Display;

/// A set of parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorMulti {
    errors: Vec<ErrorOnce>,
}

/// a single parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorOnce {
    //
}

impl Display for ErrorOnce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("errors not implemented")
    }
}

impl StdError for ErrorOnce {}

pub type StdResult<T, E> = std::result::Result<T, E>;

/// a parsing result
pub type Result<T> = std::result::Result<T, ErrorOnce>;
