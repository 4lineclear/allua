use token::Token;

use crate::{
    error::{ErrorMulti, LexicalError},
    lex,
};

#[cfg(test)]
pub mod test;
pub mod token;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    src: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
            src,
            pos: 0,
        }
    }
    /// The top level parsing function; parses the next token from within a fn
    ///
    /// Parses both functions and modules, catching lexical errors
    pub fn fn_next(&mut self) -> Option<Token> {
        use lex::token::TokenKind::*;

        loop {
            let token = self.cursor.advance_token();
            let len = token.len;
            let kind = token.kind;
            match kind {
                LineComment { doc_style } => {}
                BlockComment {
                    doc_style,
                    terminated,
                } => {}
                // empty
                Semi | Whitespace => continue,
                Ident => {}
                Literal { kind, suffix_start } => {}
                OpenParen => {}
                CloseParen => {}
                OpenBrace => {}
                CloseBrace => {}
                // NOTE: maybe add to unexpected tokens
                OpenBracket => {}
                CloseBracket => {}
                Comma | Dot | At | Pound | Tilde | Question | Colon | Dollar | Eq | Bang | Lt
                | Gt | Minus | And | Or | Plus | Star | Slash | Caret | Percent => self
                    .errors
                    .push(LexicalError::UnexpectedPunct(self.current_char())),
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::Invalid(len, self.cursor.pos_within_token())),
                Eof => return None,
            }
            self.pos += len as usize;
        }
    }
    fn current_char(&self) -> char {
        self.src[self.pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }
}

//
// pub fn ws_start<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     preceded(multispace0, inner)
// }
//
// pub fn ws_end<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     terminated(inner, multispace0)
// }
//
// /// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
// /// trailing whitespace, returning the output of `inner`.
// pub fn ws<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     delimited(multispace0, inner, multispace0)
// }
//
// pub fn identifier(input: &str) -> IResult<&str, &str, Error<&str>> {
//     recognize(all_consuming(pair(
//         verify(anychar, |&ch| is_xid_start(ch) || ch == '_'),
//         many0_count(verify(anychar, |&ch| is_xid_continue(ch))),
//     )))
//     .parse(input)
// }
//
// /// parse top level fn
// ///
// /// eq a rust mod
// pub fn parse<'a>(name: &'a str, s: &'a str) -> IResult<&'a str, Fn<'a>, Error<&'a str>> {
//     let params = CommaList { tokens: Vec::new() };
//     let (s, tokens) = parse_tokens().parse(s)?;
//     Ok((
//         s,
//         Fn {
//             name,
//             params,
//             tokens,
//         },
//     ))
// }
//
// fn parse_tokens<'a>() -> impl nom::Parser<&'a str, TokenSet<'a>, Error<&'a str>> {
//     many0(parse_token())
// }
//
// fn parse_token<'a>() -> impl nom::Parser<&'a str, Token<'a>, Error<&'a str>> {
//     alt((parse_decl.map(Token::from), parse_expr.map(Token::from)))
// }
//
// fn parse_decl(s: &str) -> IResult<&str, Decl, Error<&str>> {
//     let (s, ty) = parse_decl_type(s)?;
//     let decl = Decl {
//         ty,
//         name: todo!(),
//         value: todo!(),
//     };
//     Ok((s, decl))
// }
//
// fn parse_decl_type(s: &str) -> IResult<&str, DeclType, Error<&str>> {
//     let (s, ty) = ws_start(identifier)(s)?;
//     match ty {
//         "let" => Ok((s, DeclType::Let)),
//         "const" => {
//             let (s, ty_name) = ws_start(identifier)(s)?;
//             Ok((s, DeclType::ConstType(ty_name)))
//         }
//         ty => Ok((s, DeclType::Type(ty))),
//     }
// }
//
// fn parse_expr<I>(s: &str) -> IResult<&str, Expr, Error<I>> {
//     let expr = Expr::FnCall {
//         name: todo!(),
//         params: todo!(),
//     };
//     Ok((s, expr))
// }
//
