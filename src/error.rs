use std::error::Error as StdError;
use std::fmt::Display;

// NOTE: types of errors:
// - lexical    : encoding, definition, ident rules, token structure.
// - syntactical: contextual, set path, not one of.
// - semantic   : type errors, arg errors, nonexistent imports.

// TODO: consider having three vecs for each error kind
// TODO: move some lex error handling to crate::lex

/// A set of parsing errors
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorMulti {
    errors: Vec<ErrorOnce>,
    // TODO: replace the above with the below
    // lex: Vec<Lexical>
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
            Lexical(Unexpected(new_start, new_end)) => match self.errors.last_mut() {
                Some(Lexical(Unexpected(_, old_end))) if new_start == *old_end => {
                    *old_end = new_end;
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
// TODO: consider generalised errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexicalError {
    /// Some type of unclosed block
    Unclosed(u32),
    UnclosedBlockComment(u32),
    InvalidChar(u32),
    NameNotFound(u32),
    /// (start inclusive, end exclusive)
    Unexpected(u32, u32),
    /// Expected a token, eof found, should be extended in the future
    Eof(u32),
    MissingSemi(u32),
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
            Unexpected(_, _) => todo!(),
            Eof(_) => todo!(),
            MissingSemi(_) => todo!(),
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
