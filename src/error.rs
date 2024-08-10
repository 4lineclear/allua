use std::error::Error as StdError;
use std::fmt::Display;

use crate::span::BSpan;

// NOTE: types of errors:
// - lexical    : encoding, definition, ident rules, token structure.
// - syntactical: contextual, set path, not one of.
// - semantic   : type errors, arg errors, nonexistent imports.

// TODO: move some lex error handling to crate::lex

/// A set of parsing errors
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorMulti {
    pub lex: Vec<LexicalError>,
    pub other: Vec<String>,
}

impl ErrorMulti {
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn push(&mut self, err: impl Into<ErrorOnce>) {
        use ErrorOnce::*;
        use LexicalError::*;
        let err = err.into();
        match err {
            Lexical(Unexpected(new)) => match self.lex.last_mut() {
                Some(Unexpected(old)) if new.from == old.to => {
                    old.to = new.to;
                }
                _ => self.lex.push(Unexpected(new)),
            },
            Lexical(lex) => self.lex.push(lex),
            Other(err) => self.other.push(err),
        }
    }
}

/// a single parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorOnce {
    Lexical(LexicalError),
    Other(String),
}

impl From<LexicalError> for ErrorOnce {
    fn from(value: LexicalError) -> Self {
        Self::Lexical(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalError {
    /// Some type of unclosed block
    Unclosed(BSpan),
    /// (start inclusive, end exclusive)
    Unexpected(BSpan),
    /// Expected a token, eof found, should be extended in the future
    Eof(usize),
}

impl Display for ErrorOnce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("errors display not implemented")
    }
}

impl StdError for ErrorOnce {}

pub type StdResult<T, E> = std::result::Result<T, E>;

/// a parsing result
pub type Result<T> = std::result::Result<T, ErrorOnce>;
