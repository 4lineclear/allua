// TODO: add raw idents back in eventually.
// TODO: create pattern-composer macro
// TODO: consider adding system where doc comments can be anywhere?
// maybe change how doc comments are considered compared to rust.
// TODO: consider using u64 or usize over u32
// TODO: consider rewriting the below.
// TODO: add tuples
// TODO: consider removing semicolons, replacing them with nl
#![allow(clippy::cast_possible_truncation)]

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    span::{BSpan, TSpan},
    util::Symbol,
};

#[cfg(test)]
pub mod test;
pub mod token;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    tokens: Vec<token::Token>,
    /// a backlog of blocks
    blocks: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Module,
    Fn,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
            tokens: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Parse a module
    #[must_use]
    pub fn module(mut self, name: &str) -> (token::Module, ErrorMulti) {
        while self.next() {}
        (token::Module::new(name, self.tokens), self.errors)
    }

    fn next(&mut self) -> bool {
        use lex::token::TokenKind::*;
        let token = self.cursor.advance_token();
        let span = self.token_span(token.len);
        let kind = token.kind;
        match kind {
            // (?doc)comments or whitespace. skip normal comments
            _ if self.filter_comment_or_whitespace(token) => (),
            Semi => (),
            Ident | RawIdent => match self.parse_ident(span) {
                Some(token) => self.tokens.push(token),
                None => (),
            },
            // code block
            OpenBrace => {
                self.tokens.push(token::Token::Dummy);
                self.blocks.push(self.tokens.len() as u32);
            }
            // code block end
            CloseBrace => {
                let Some(from) = self.blocks.pop() else {
                    self.err_unexpected(token);
                    return true;
                };
                self.tokens[from as usize - 1] = token::Token::Block(TSpan {
                    from,
                    to: self.tokens.len() as u32,
                });
            }
            Eof => return false,
            _ => self.err_unexpected(token),
        };
        true
    }

    // TODO: add handling for unset vars, when set is expected
    fn parse_ident(&mut self, span: BSpan) -> Option<token::Token> {
        let kind = match self.range(span) {
            "let" => token::DeclKind::Let,
            "const" => token::DeclKind::Const,
            _ => return self.parse_fn_call(span, true).map(Into::into),
        };
        self.parse_decl(kind).map(token::Token::from)
    }

    fn parse_decl(&mut self, kind: token::DeclKind) -> Option<token::Decl> {
        // get either var-name or type-name
        let Some(first_span) = self.until_ident() else {
            self.push_err(LexicalError::Eof(self.cursor.pos()));
            return None;
        };

        // if this is C, first_span is for type, not var
        let Some(next_token) = self.semi_or_eq_or_ident() else {
            self.push_err(LexicalError::Eof(self.cursor.pos()));
            return None;
        };

        let name;
        let mut type_name = None;
        let mut value = None;
        match next_token {
            Either3::A(()) => name = self.range(first_span),
            Either3::B(()) => {
                let expr = self.parse_expr();
                self.semi();
                name = self.range(first_span);
                value = expr;
            }
            Either3::C(var_span) => {
                let Some(is_semi) = self.semi_or_eq() else {
                    self.push_err(LexicalError::Eof(self.cursor.pos()));
                    return None;
                };
                let expr = if is_semi {
                    None
                } else {
                    let expr = self.parse_expr();
                    self.semi();
                    expr
                };
                name = self.range(var_span);
                type_name = Some(self.symbol(first_span));
                value = expr;
            }
        };

        Some(token::Decl::new(kind, type_name, name.into(), value))
    }

    /// (..)
    fn parse_fn_call(&mut self, span: BSpan, check_paren: bool) -> Option<token::Expr> {
        // TODO: test incorrect/correct function calls
        if check_paren && !self.until_open_paren() {
            self.err_unexpected(span);
            return None;
        }
        let from = self.tokens.len();
        self.tokens.push(token::Token::Dummy);

        let expr = loop {
            let next = match self.parse_params() {
                // Eof
                Either3::A(()) => {
                    self.tokens.truncate(from);
                    self.push_err(LexicalError::Eof(self.token_pos()));
                    return None;
                }
                // close paren
                Either3::B(expr) => break expr,
                // params
                Either3::C(Some(expr)) => expr,
                Either3::C(None) => continue,
            };
            self.tokens.push(next.into());
        };

        let set_idx = from;
        let to = self.tokens.len() as u32 + expr.is_some() as u32;
        let from = from as u32 + 1;
        self.tokens[set_idx] = token::Expr::FnCall(self.symbol(span), TSpan { from, to }).into();

        expr
    }

    /// ..)
    ///
    /// `A` = Eof `B` = CloseParen `C` = Param
    fn parse_params(&mut self) -> Either3<(), Option<token::Expr>, Option<token::Expr>> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            let span = self.token_span(token.len);
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Comma => (),
                CloseParen => break Either3::B(None),
                Ident | RawIdent => {
                    break loop {
                        let after_ident = self.cursor.advance_token();
                        match after_ident.kind {
                            _ if self.filter_comment_or_whitespace(after_ident) => (),
                            OpenParen => break Either3::C(self.parse_fn_call(span, false)),
                            CloseParen => {
                                break Either3::B(Some(token::Expr::Var(self.symbol(span))))
                            }
                            Comma => break Either3::C(Some(token::Expr::Var(self.symbol(span)))),
                            Eof => break Either3::A(()),
                            _ => self.err_unexpected(after_ident),
                        }
                    }
                }
                Literal { kind, suffix_start } => {
                    let expr =
                        Some(token::Value::new(self.symbol(span), kind, suffix_start).into());
                    break Either3::C(expr);
                }
                Eof => break Either3::A(()),
                _ => self.err_unexpected(token),
            }
        }
    }

    /// Parses until an ident, returns the byte position
    fn until_ident(&mut self) -> Option<BSpan> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Ident | RawIdent => return Some(self.token_span(token.len)),
                Eof => return None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn until_open_paren(&mut self) -> bool {
        use lex::token::TokenKind::*;

        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                OpenParen => break true,
                Eof => break false,
                _ => self.err_unexpected(token),
            }
        }
    }

    /// parse a top level expr
    fn parse_expr(&mut self) -> Option<token::Expr> {
        let token = self.cursor.advance_token();
        self.parse_expr_with(token)
    }

    /// parse a top level expr
    fn parse_expr_with(&mut self, mut token: lex::Token) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        loop {
            let span = self.token_span(token.len);
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Ident | RawIdent => break self.parse_fn_call(span, true),
                Literal { kind, suffix_start } => {
                    break Some(token::Expr::Value(token::Value::new(
                        self.current_range(token.len).into(),
                        kind,
                        suffix_start,
                    )))
                }
                Eof => break None,
                _ => self.err_unexpected(token),
            }
            token = self.cursor.advance_token();
        }
    }

    /// A Semi, B Eq, C(len) Ident
    fn semi_or_eq_or_ident(&mut self) -> Option<Either3<(), (), BSpan>> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Semi => break Some(Either3::A(())),
                Eq => break Some(Either3::B(())),
                Ident | RawIdent => break Some(Either3::C(self.token_span(token.len))),
                Eof => break None,
                _ => self.err_unexpected(token),
            }
        }
    }

    /// Returns true if a semicolon was found
    fn semi_or_eq(&mut self) -> Option<bool> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Semi => break Some(true),
                Eq => break Some(false),
                Eof => break None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn semi(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_comment_or_whitespace(token) => (),
                Semi => break true,
                Eof => {
                    self.push_err(LexicalError::MissingSemi(self.token_pos()));
                    break false;
                }
                _ => self.err_unexpected(token),
            }
        }
    }

    fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Token(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }

    fn err_unexpected(&mut self, span: impl Into<AsBSpan>) {
        self.push_err(LexicalError::Unexpected(self.span(span)))
    }

    fn filter_comment_or_whitespace(&mut self, token: lex::Token) -> bool {
        use lex::TokenKind::*;
        match token.kind {
            BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::Unclosed(self.token_span(token.len)));
                true
            }
            LineComment { .. } | BlockComment { .. } | Whitespace => true,
            _ => false,
        }
    }

    #[must_use]
    #[inline]
    pub const fn src(&self) -> &str {
        self.cursor.src()
    }

    #[allow(dead_code)]
    fn current_char(&self) -> char {
        let pos = self.token_pos() as usize;
        self.src()[pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    fn current_range(&self, len: u32) -> &str {
        self.range(self.token_span(len))
    }

    fn range(&self, span: BSpan) -> &str {
        &self.src()[span.from as usize..span.to as usize]
    }

    fn symbol(&self, span: BSpan) -> Symbol {
        self.src()[span.from as usize..span.to as usize].into()
    }

    const fn token_pos(&self) -> u32 {
        self.cursor.token_pos()
    }

    const fn token_span(&self, len: u32) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }

    fn push_err(&mut self, err: impl Into<ErrorOnce>) {
        self.errors.push(err);
    }
}

#[derive(Debug)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

#[derive(Debug)]
pub enum Either3<A, B, C> {
    A(A),
    B(B),
    C(C),
}

#[derive(Debug)]
pub enum Either4<A, B, C, D> {
    A(A),
    B(B),
    C(C),
    D(D),
}

enum AsBSpan {
    // Current span used as start
    Len(u32),
    Token(lex::Token),
    // Uses given
    Span(BSpan),
}

impl From<u32> for AsBSpan {
    fn from(value: u32) -> Self {
        Self::Len(value)
    }
}
impl From<lex::Token> for AsBSpan {
    fn from(value: lex::Token) -> Self {
        Self::Token(value)
    }
}
impl From<BSpan> for AsBSpan {
    fn from(value: BSpan) -> Self {
        Self::Span(value)
    }
}
