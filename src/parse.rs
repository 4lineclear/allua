// TODO: create pattern-composer macro
// TODO: consider adding system where doc comments can be anywhere?
// maybe change how doc comments are considered compared to rust.
// TODO: consider using u64 or usize over u32
// TODO: consider rewriting everything
// TODO: add tuples
// TODO: allow for parsing code blocks in other areas.
// code block
// TODO: create a compiler error type.
// TODO: add visibility item to Fn
// TODO: consider unifying the "different kinds" of expr syntax into one
// TODO: add operators
#![allow(clippy::cast_possible_truncation)]

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    span::{BSpan, TSpan},
};

pub use secure::Reader;

/// a secure module for keeping certain fields safe.
mod secure;
#[cfg(test)]
pub mod test;
pub mod token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Module,
    Fn,
}

impl<'a> Reader<'a> {
    /// Parse a module
    #[must_use]
    pub fn module(mut self, name: &str) -> (token::Module, ErrorMulti) {
        while self.next() {}
        let (cursor, mut errors, mut tokens, spans, blocks) = self.into_parts();

        for pos in blocks {
            let Some(span) = spans.get(&pos) else {
                errors.push(ErrorOnce::Other(format!(
                    "found pos in block backlog that was out of bounds: {pos}"
                )));
                continue;
            };
            let span = BSpan::new(span.from, cursor.pos());
            errors.push(LexicalError::Unclosed(span));
            tokens.truncate(pos);
        }

        (token::Module::new(name, tokens), errors)
    }

    fn next(&mut self) -> bool {
        let token = self.cursor.advance_token();
        match self.next_or_close_brace(token) {
            Either3::A(()) => false,
            Either3::B(()) => true,
            Either3::C(()) => {
                self.set_block(token);
                true
            }
        }
    }

    /// `A` = `Eof` `B` = `Token` `C` = `CloseBrace`
    fn next_or_close_brace(&mut self, token: lex::Token) -> Either3<(), (), ()> {
        use lex::token::TokenKind::*;
        let span = self.token_span(token.len);
        let kind = token.kind;
        match kind {
            // (?doc)comments or whitespace. skip normal comments
            _ if self.filter_comment_or_whitespace(token) => (),
            // ignore semicolons
            Semi => (),
            Ident | RawIdent => match self.parse_ident(span) {
                Some(token) => self.push_token(token),
                None => (),
            },
            OpenBrace => {
                self.push_block(self.len());
                self.push_token(token::Token::Dummy);
            }
            // code block end
            CloseBrace => return Either3::C(()),
            Eof => return Either3::A(()),
            _ => self.err_unexpected(token),
        };

        Either3::B(())
    }

    fn parse_ident(&mut self, span: BSpan) -> Option<token::Token> {
        match self.range(span) {
            "let" => self.parse_decl(token::DeclKind::Let).map(Into::into),
            "const" => self.parse_decl(token::DeclKind::Const).map(Into::into),
            "fn" => {
                self.parse_fn_def();
                None
            }
            // NOTE: parse_expr adds fn_call in place, breaking this.
            "return" => {
                let set_idx = self.len();
                self.push_token(token::Token::Dummy);
                let Either::B(expr) = self.parse_return() else {
                    self.truncate(set_idx);
                    return None;
                };
                self.set_return(set_idx, set_idx + 1);
                expr.map(Into::into)
            }
            _ => self.parse_fn_call(span, true).map(Into::into),
        }
    }

    fn parse_decl(&mut self, kind: token::DeclKind) -> Option<token::Decl> {
        // get either var-name or type-name
        let Some(first_span) = self.until_ident() else {
            self.push_err(LexicalError::Eof(self.cursor.pos()));
            return None;
        };

        // if this is C, first_span is for type, not var
        let next_token = self.eq_or_ident();

        let name;
        let value;
        let type_name;
        match next_token {
            Either3::A(()) => {
                self.push_err(LexicalError::Eof(self.cursor.pos()));
                return None;
            }
            Either3::B(()) => {
                value = self.parse_expr();
                name = self.range(first_span);
                type_name = None;
            }
            Either3::C(var_span) => {
                value = if self.until_eq() {
                    self.parse_expr()
                } else {
                    self.push_err(LexicalError::Eof(self.cursor.pos()));
                    return None;
                };
                name = self.range(var_span);
                type_name = Some(self.symbol(first_span));
            }
        };

        Some(token::Decl::new(kind, type_name, name.into(), value))
    }

    /// (..)
    fn parse_fn_call(&mut self, span: BSpan, check_paren: bool) -> Option<token::Expr> {
        if check_paren && !self.until_open_paren() {
            self.err_unexpected(span);
            return None;
        }
        let from = self.len();
        self.push_token(token::Token::Dummy);

        let expr = loop {
            let next = match self.parse_call_params() {
                // Eof
                Either3::A(()) => {
                    self.truncate(from);
                    self.push_err(LexicalError::Eof(self.token_pos()));
                    return None;
                }
                // close paren
                Either3::B(expr) => break expr,
                // params
                Either3::C(Some(expr)) => expr,
                Either3::C(None) => continue,
            };
            self.push_token(next);
        };

        let set_idx = from;
        let to = self.len() + usize::from(expr.is_some());
        let from = from + 1;
        self.set_fn_call(set_idx, self.symbol(span), TSpan { from, to });

        expr
    }

    pub fn parse_fn_def(&mut self) {
        let Some(first) = self.until_ident() else {
            return;
        };

        let (name, type_name) = match self.until_open_paren_or_ident() {
            Either3::A(()) => {
                self.push_err(LexicalError::Eof(self.token_pos()));
                return;
            }
            Either3::B(()) => (first, None),
            Either3::C(_) if !self.until_open_paren() => {
                self.push_err(LexicalError::Eof(self.token_pos()));
                return;
            }
            Either3::C(span) => (span, Some(first)),
        };

        let dummy_pos = self.len();
        self.push_token(token::Token::Dummy);

        loop {
            match self.parse_def_params() {
                Either::A(()) => {
                    self.truncate(dummy_pos);
                    self.push_err(LexicalError::Eof(self.token_pos()));
                    return;
                }
                Either::B((true, Some(param))) => break self.push_token(param),
                Either::B((true, None)) => break,
                Either::B((false, Some(param))) => self.push_token(param),
                Either::B((false, None)) => (),
            };
        }

        let param_start = dummy_pos + 1;
        let param_end = self.len();
        let param_span = TSpan {
            from: param_start,
            to: param_end,
        };

        if !self.until_open_brace() {
            self.truncate(dummy_pos);
            self.push_err(LexicalError::Eof(self.token_pos()));
            return;
        }

        loop {
            let token = self.cursor.advance_token();
            match self.next_or_close_brace(token) {
                Either3::A(()) => {
                    self.truncate(dummy_pos);
                    self.push_err(LexicalError::Eof(self.token_pos()));
                    return;
                }
                Either3::B(()) => (),
                Either3::C(()) => break,
            };
        }

        let token_span = TSpan {
            from: param_end,
            to: self.len(),
        };

        self.set_fn_def(dummy_pos, name, type_name, param_span, token_span);
    }

    /// ..)
    ///
    /// `A` = `Eof` `B` = `CloseParen` `C` = `Param`
    fn parse_call_params(&mut self) -> Either3<(), Option<token::Expr>, Option<token::Expr>> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.token_span(token.len);
            match token.kind {
                Comma => (),
                CloseParen => break Either3::B(None),
                Ident | RawIdent => {
                    break loop {
                        let Some(after_ident) = self.lex_non_wc() else {
                            continue;
                        };
                        match after_ident.kind {
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

    /// return ..
    ///
    /// `A` = `Eof`  `C` = `Param`
    fn parse_return(&mut self) -> Either<(), Option<token::Expr>> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.token_span(token.len);
            match token.kind {
                Ident | RawIdent => {
                    let Some(after_ident) = self.lex_non_wc() else {
                        break Either::B(None);
                    };
                    match after_ident.kind {
                        OpenParen => break Either::B(self.parse_fn_call(span, false)),
                        Eof => break Either::A(()),
                        _ => self.err_unexpected(after_ident),
                    }
                }
                Literal { kind, suffix_start } => {
                    let expr =
                        Some(token::Value::new(self.symbol(span), kind, suffix_start).into());
                    break Either::B(expr);
                }
                Eof => break Either::A(()),
                _ => self.err_unexpected(token),
            }
        }
    }

    /// ..)
    ///
    /// `A` = `Eof` `B` & `true` = `CloseParen` `B` & `false` = `Param`
    fn parse_def_params(&mut self) -> Either<(), (bool, Option<token::FnDefParam>)> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.token_span(token.len);
            match token.kind {
                Comma => (),
                CloseParen => break Either::B((true, None)),
                Ident | RawIdent => match self.parse_def_decl(span) {
                    Either::A(()) => break Either::A(()),
                    Either::B(val) => break Either::B(val),
                },
                Eof => break Either::A(()),
                _ => self.err_unexpected(token),
            }
        }
    }

    /// similar to [`Self::parse_decl`], but detecting a closing paren
    ///
    /// `A` = `Eof` `B` & `true` = `CloseParen` `B` & `false` = `Param`
    fn parse_def_decl(&mut self, first: BSpan) -> Either<(), (bool, Option<token::FnDefParam>)> {
        use lex::token::TokenKind::*;
        let Some(second) = self.until_ident() else {
            self.push_err(LexicalError::Eof(self.token_pos()));
            return Either::A(());
        };
        let (close, value) = loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                // parse param with default val
                Eq => break (false, self.parse_expr()),
                // simple param found, cont parse
                Comma => break (false, None),
                // all params found, stop parse
                CloseParen => break (true, None),
                // eof with no close, err
                Eof => return Either::A(()),
                _ => self.err_unexpected(token),
            }
        };
        let fn_def_param = token::FnDefParam {
            type_name: self.range(first).into(),
            name: self.range(second).into(),
            value,
        };
        Either::B((close, Some(fn_def_param)))
    }

    /// Parses until an ident, returns the byte position
    fn until_ident(&mut self) -> Option<BSpan> {
        use lex::token::TokenKind::*;

        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                Ident | RawIdent => return Some(self.token_span(token.len)),
                Eof => return None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn until_open_paren(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                OpenParen => break true,
                Eof => break false,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn until_open_brace(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                OpenBrace => break true,
                Eof => break false,
                _ => self.err_unexpected(token),
            }
        }
    }

    /// `A` = `Eof`, `B`, = `OpenParen`, `C` = `Ident`
    fn until_open_paren_or_ident(&mut self) -> Either3<(), (), BSpan> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                OpenParen => return Either3::B(()),
                Ident | RawIdent => return Either3::C(self.token_span(token.len)),
                Eof => return Either3::A(()),
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

    /// A Eof, B Eq, C(len) Ident
    fn eq_or_ident(&mut self) -> Either3<(), (), BSpan> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                Eq => break Either3::B(()),
                Ident | RawIdent => break Either3::C(self.token_span(token.len)),
                Eof => break Either3::A(()),
                _ => self.err_unexpected(token),
            }
        }
    }

    /// Returns true if a eq was found
    fn until_eq(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                Eq => break true,
                Eof => break false,
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
        self.push_err(LexicalError::Unexpected(self.span(span)));
    }

    fn lex_non_wc(&mut self) -> Option<lex::Token> {
        let token = self.cursor.advance_token();
        (!self.filter_comment_or_whitespace(token)).then_some(token)
    }

    // TODO: consider also having a flag for parsing when there is a doc comment
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
    Len(usize),
    Token(lex::Token),
    // Uses given
    Span(BSpan),
}

impl From<usize> for AsBSpan {
    fn from(value: usize) -> Self {
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
