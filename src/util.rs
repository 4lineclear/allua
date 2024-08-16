use unicode_normalization::{is_nfc_quick, IsNormalized, UnicodeNormalization};

pub type Symbol = ustr::Ustr;

use Either::*;
use Filtered::*;

use crate::lex::Lexeme;
use crate::span::BSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B> Either<A, B> {
    pub fn map_a<C>(self, map: impl Fn(A) -> C) -> Either<C, B> {
        match self {
            A(a) => Either::A(map(a)),
            B(b) => Either::B(b),
        }
    }

    pub fn map_b<C>(self, map: impl Fn(B) -> C) -> Either<A, C> {
        match self {
            A(a) => Either::A(a),
            B(b) => Either::B(map(b)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either3<X, Y, Z> {
    X(X),
    Y(Y),
    Z(Z),
}

/// A filtered [`lex::Lexeme`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filtered<T> {
    InputEnd,
    Correct(T),
    /// !(`Whitespace` | `Eof` | `Correct(T)`)
    Other(Lexeme),
}

impl<T> Filtered<T> {
    pub fn map<T2>(self, map: impl Fn(T) -> T2) -> Filtered<T2> {
        match self {
            InputEnd => InputEnd,
            Correct(t) => map(t).into(),
            Other(t) => Other(t),
        }
    }

    pub const fn is_correct(&self) -> bool {
        matches!(self, Correct(_))
    }
}

impl<T> From<T> for Filtered<T> {
    fn from(value: T) -> Self {
        Correct(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsStr<'a> {
    Span(AsBSpan),
    Symbol(Symbol),
    Str(&'a str),
}

impl From<Symbol> for AsStr<'_> {
    fn from(value: Symbol) -> Self {
        Self::Symbol(value)
    }
}

impl<'a> From<&'a str> for AsStr<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsBSpan {
    // Current span used as start
    Len(usize),
    Lex(Lexeme),
    // Uses given
    Span(BSpan),
}

macro_rules! impl_from {
    ($($ty:ty, $name:ident),*) => { $(
        impl From<$ty> for AsBSpan {
            fn from(value: $ty) -> Self {
                Self::$name(value)
            }
        }
        impl<'a> From<$ty> for AsStr<'a> {
            fn from(value: $ty) -> Self {
                Self::Span(AsBSpan::$name(value))
            }
        }
    )*};
}

impl_from!(usize, Len, Lexeme, Lex, BSpan, Span);

#[must_use]
pub const fn is_whitespace(c: char) -> bool {
    matches!(
        c,
        // Usual ASCII suspects
        '\u{0009}'   // \t
        | '\u{000A}' // \n
        | '\u{000B}' // vertical tab
        | '\u{000C}' // form feed
        | '\u{000D}' // \r
        | '\u{0020}' // space

        // NEXT LINE from latin1
        | '\u{0085}'

        // Bidi markers
        | '\u{200E}' // LEFT-TO-RIGHT MARK
        | '\u{200F}' // RIGHT-TO-LEFT MARK

        // Dedicated whitespace characters from Unicode
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

#[must_use]
pub fn is_id_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

#[must_use]
pub fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

#[must_use]
pub fn nfc_normalize(string: &str) -> Symbol {
    match is_nfc_quick(string.chars()) {
        IsNormalized::Yes => Symbol::from(string),
        _ => Symbol::from(&string.chars().nfc().collect::<String>()),
    }
}
