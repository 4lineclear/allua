//! a basic version of a lua-like langauge
#![forbid(unsafe_code)]
#![deny(
    clippy::all,
    clippy::pedantic,
    // clippy::cargo,
    clippy::nursery,
    missing_docs,
    rustdoc::all,
    future_incompatible
)]
#![warn(missing_debug_implementations)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::wildcard_dependencies)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::unused_io_amount)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::new_without_default)]
#![allow(missing_docs)]
#![allow(dead_code)]

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{anychar, multispace0},
    combinator::{all_consuming, opt, recognize, verify},
    error::ParseError,
    multi::{many0, many0_count},
    sequence::{delimited, pair, preceded, terminated},
    IResult, Parser,
};
use unicode_ident::{is_xid_continue, is_xid_start};

type StdResult<T, E> = std::result::Result<T, E>;
type Result<I, T> = StdResult<T, Error<I>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error<I> {
    Nom(I, nom::error::ErrorKind),
    Multi(Vec<Self>),
}

impl<I> ParseError<I> for Error<I> {
    fn from_error_kind(input: I, kind: nom::error::ErrorKind) -> Self {
        Self::Nom(input, kind)
    }

    fn append(input: I, kind: nom::error::ErrorKind, other: Self) -> Self {
        Self::Multi(vec![Self::Nom(input, kind), other])
    }
}

// TODO: better error handling

// impl<I, E> FromExternalError<I, E> for Error<I> {
//     fn from_external_error(_input: I, _kind: nom::error::ErrorKind, _e: E) -> Self {
//         todo!()
//     }
// }
//
// impl<I> ContextError<I> for Error<I> {
//     fn add_context(_input: I, _ctx: &'static str, _other: Self) -> Self {
//         todo!()
//     }
// }
//

pub fn ws_start<'a, O, E: ParseError<&'a str>, F>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    preceded(multispace0, inner)
}

pub fn ws_end<'a, O, E: ParseError<&'a str>, F>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    terminated(inner, multispace0)
}

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
pub fn ws<'a, O, E: ParseError<&'a str>, F>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Parser<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn identifier(input: &str) -> IResult<&str, &str, Error<&str>> {
    recognize(all_consuming(pair(
        verify(anychar, |&ch| is_xid_start(ch) || ch == '_'),
        many0_count(verify(anychar, |&ch| is_xid_continue(ch))),
    )))
    .parse(input)
}

/// parse top level fn
///
/// eq a rust mod
pub fn parse<'a>(name: &'a str, s: &'a str) -> IResult<&'a str, Fn<'a>, Error<&'a str>> {
    let params = CommaList { tokens: Vec::new() };
    let (s, tokens) = parse_tokens().parse(s)?;
    Ok((
        s,
        Fn {
            name,
            params,
            tokens,
        },
    ))
}

// NOTE:
// is run automatically when imported, unless `defer` is used

/// A user defined function
///
/// Acts as both as a module, datatype and function
///
///
/// ```ignore
/// fn ::name(::params) then
///   ::tokens
/// end
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fn<'a> {
    name: &'a str,
    params: CommaList<Decl<'a>>,
    tokens: TokenSet<'a>,
}

type TokenSet<'a> = Vec<Token<'a>>;

fn parse_tokens<'a>() -> impl nom::Parser<&'a str, TokenSet<'a>, Error<&'a str>> {
    many0(parse_token())
}

fn parse_token<'a>() -> impl nom::Parser<&'a str, Token<'a>, Error<&'a str>> {
    alt((parse_decl.map(Token::from), parse_expr.map(Token::from)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token<'a> {
    Fn(Fn<'a>),
    Decl(Decl<'a>),
    Expr(Expr<'a>),
    Import(Import<'a>),
}

macro_rules! token_from {
    ($($name:ident),*) => {$(
        impl<'a> From<$name<'a>> for Token<'a> {
            fn from(value: $name<'a>) -> Self {
                Self::$name(value)
            }
        }
    )*};
}

token_from!(Fn, Decl, Expr, Import);

/// [`DeclType`] <name> ?(= <value>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decl<'a> {
    /// if none try implicit
    ty: DeclType<'a>,
    name: &'a str,
    value: Option<Value<'a>>,
}

fn parse_decl(s: &str) -> IResult<&str, Decl, Error<&str>> {
    let (s, ty) = parse_decl_type(s)?;
    let decl = Decl {
        ty,
        name: todo!(),
        value: todo!(),
    };
    Ok((s, decl))
}

// TODO: add let style const decl

/// <type> | `let` | `const` | `const` <type>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclType<'a> {
    /// let <name>
    Let,
    /// <type> <name>
    Type(&'a str),
    /// const <type> <name>
    ConstType(&'a str),
}

fn parse_decl_type(s: &str) -> IResult<&str, DeclType, Error<&str>> {
    let (s, ty) = ws_start(identifier)(s)?;
    match ty {
        "let" => Ok((s, DeclType::Let)),
        "const" => {
            let (s, ty_name) = ws_start(identifier)(s)?;
            Ok((s, DeclType::ConstType(ty_name)))
        }
        ty => Ok((s, DeclType::Type(ty))),
    }
}

/// <item>, <item>, ..
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommaList<T> {
    tokens: Vec<T>,
}

/// ?(<defer>) `use` <name>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import<'a> {
    name: &'a str,
    defer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'a> {
    // "..."
    String(&'a str),
}

/// <expr>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr<'a> {
    /// <name>(<params>...)
    FnCall {
        name: &'a str,
        params: CommaList<Decl<'a>>,
    },
}

fn parse_expr<I>(s: &str) -> IResult<&str, Expr, Error<I>> {
    let expr = Expr::FnCall {
        name: todo!(),
        params: todo!(),
    };
    Ok((s, expr))
}

mod test {
    use crate::DeclType;

    #[test]
    fn identifier() {
        let tests = ["foo", "_identifier", "Москва", "東京"];
        for input in tests {
            let ident = super::identifier(input);
            assert_eq!(ident, Ok(("", input)));
        }
    }
    #[test]
    fn decl_type() {
        // NOTE: when parsing identifier, expected whitespace start.
        // that is wrong.
        let tests = [
            ("let foo", DeclType::Let),
            // ("const foo", DeclType::Const),
            ("string foo", DeclType::Type("string")),
            ("const string foo", DeclType::ConstType("string")),
        ];

        for (input, expected) in tests {
            let ident = super::parse_decl_type(input);
            assert_eq!(ident, Ok(("foo", expected)));
        }
    }
}
