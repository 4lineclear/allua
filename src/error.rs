use std::error::Error as StdError;
use std::fmt::Display;

use crate::lex;
use crate::util::Symbol;

// NOTE: types of errors:
// - lexical    : encoding, definition, ident rules, token structure.
// - syntactical: contextual, set path, not one of.
// - semantic   : type errors, arg errors, nonexistent imports.

// TODO: consider having three vecs for each error kind

/// A set of parsing errors
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorMulti {
    errors: Vec<ErrorOnce>,
}

impl From<Vec<ErrorOnce>> for ErrorMulti {
    fn from(value: Vec<ErrorOnce>) -> Self {
        Self { errors: value }
    }
}

impl ErrorMulti {
    #[allow(clippy::match_wildcard_for_single_variants)]
    pub fn push(&mut self, err: impl Into<ErrorOnce>) {
        use ErrorOnce::*;
        use LexicalError::*;
        let err = err.into();
        match err {
            Lexical(UnexpectedPunct(_, t)) => match self.errors.last_mut() {
                Some(Lexical(UnexpectedPunct(_, f))) if t == *f + 1 => {
                    let f = *f;
                    self.errors.pop();
                    self.errors.push(UnexpectedRange(f, t).into());
                }
                Some(Lexical(UnexpectedRange(_, t1))) if t == *t1 + 1 => {
                    *t1 += 1;
                }
                _ => self.errors.push(err),
            },
            _ => self.errors.push(err),
        }
    }

    // TODO: test string fns
    #[must_use]
    #[cfg(any(test, debug_assertions))]
    pub fn to_test_string(&self) -> String {
        let mut out = String::new();
        for err in &self.errors {
            out.push_str(&err.to_test_string());
        }
        out
    }
}

/// a single parsing error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorOnce {
    Lexical(LexicalError),
}

impl ErrorOnce {
    #[must_use]
    #[cfg(any(test, debug_assertions))]
    pub fn to_test_string(&self) -> String {
        match self {
            Self::Lexical(err) => err.to_test_string(),
        }
    }
}

impl From<LexicalError> for ErrorOnce {
    fn from(value: LexicalError) -> Self {
        Self::Lexical(value)
    }
}

// TODO: create extendable errors
// ie invalid punct could be a range instead of a char
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalError {
    /// Some type of unclosed block
    Unclosed(u32),
    UnclosedBlockComment(u32),
    InvalidChar(u32),
    NameNotFound(u32),
    UnexpectedEof(u32),
    UnexpectedIdent(Symbol, u32),
    UnexpectedPunct(char, u32),
    /// should (?maybe) be constructed directly
    UnexpectedRange(u32, u32),
    UnexpectedLit(lex::LiteralKind, u32),
    UnexpectedComment(Option<lex::DocStyle>, u32),
    UnexpectedWhitespace(u32),
    MissingSemi(u32, u32),
}

impl LexicalError {
    #[cfg(any(test, debug_assertions))]
    fn to_test_string(&self) -> String {
        use LexicalError::*;
        match self {
            Unclosed(_) => todo!(),
            UnclosedBlockComment(_) => todo!(),
            InvalidChar(_) => todo!(),
            NameNotFound(_) => todo!(),
            UnexpectedEof(_) => todo!(),
            UnexpectedIdent(_, _) => todo!(),
            UnexpectedPunct(_, _) => todo!(),
            UnexpectedRange(_, _) => todo!(),
            UnexpectedLit(_, _) => todo!(),
            UnexpectedComment(_, _) => todo!(),
            UnexpectedWhitespace(_) => todo!(),
            MissingSemi(_, _) => todo!(),
        }
    }
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
