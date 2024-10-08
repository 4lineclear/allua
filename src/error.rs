use std::error::Error as StdError;
use std::fmt::Display;

use crate::lex;
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
            Lexical(Expected(new, new_correct)) => match self.lex.last_mut() {
                Some(Expected(old, old_correct))
                    if new.from == old.to && &new_correct == old_correct =>
                {
                    old.to = new.to;
                }
                _ => self.lex.push(Expected(new, new_correct)),
            },
            Lexical(DupeComma(new)) => match self.lex.last_mut() {
                Some(DupeComma(old)) if new.from == old.to => {
                    old.to = new.to;
                }
                _ => self.lex.push(DupeComma(new)),
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

// TODO: have expected use a u64 instead of a vec
//
// Should be a big space & speed improvement

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalError {
    /// Duplicate commas
    DupeComma(BSpan),
    /// Some type of unclosed block
    Unclosed(BSpan),
    /// (start inclusive, end exclusive)
    Expected(BSpan, Vec<lex::LexKind>),
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
