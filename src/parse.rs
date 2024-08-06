use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    span::{BSpan, TSpan},
};

// TODO: add raw idents back in eventually.
// TODO: simplify to_string
// TODO: create pattern-composer macro
// TODO: consider adding system where doc comments can be anywhere?
// maybe change how doc comments are considered compared to rust.
// TODO: consider using u64 or usize over u32
// TODO: consider rewriting the below.
// TODO: move to use BSpan
// TODO: consider removing normal var bindings, replacing them with <let> <type> <name> (? = <value>);

#[cfg(test)]
pub mod test;
pub mod token;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    tokens: Vec<token::Token>,
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
        }
    }

    #[must_use]
    #[inline]
    pub fn src(&self) -> &str {
        self.cursor.src()
    }

    /// Parse a module
    pub fn module(mut self, name: &str) -> (token::Module, ErrorMulti) {
        while let Some(token) = self.next(ParseMode::Module) {
            self.tokens.push(token);
        }
        (token::Module::new(name, self.tokens), self.errors)
    }

    #[allow(unused)]
    fn next(&mut self, mode: ParseMode) -> Option<token::Token> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            let span = self.token_span(token.len);
            let kind = token.kind;
            match kind {
                // (?doc)comments. skip normal comments
                _ if self.err_block_comment(token) => (),
                LineComment { doc_style } => {
                    if let Some(style) = doc_style {
                        todo!("doc comments not added yet");
                    }
                }
                BlockComment {
                    doc_style,
                    terminated,
                } => {
                    if let Some(style) = doc_style {
                        todo!("doc comments not added yet");
                    }
                }
                // empty
                Semi | Whitespace => (),
                Ident => match self.parse_ident(span) {
                    Some(token) => return Some(token),
                    None => (),
                },
                // NOTE: may require "backlog" stack
                // code block
                OpenBrace => {
                    todo!("code blocks not implemented yet!")
                }
                // fn end
                CloseBrace if mode == ParseMode::Fn => return None,
                Eof => return None,
                _ => self.err_unexpected(token),
            }
        }
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
                type_name = Some(self.range(first_span).into());
                value = expr;
            }
        };

        Some(token::Decl::new(kind, type_name, name.into(), value).into())
    }

    /// (..)
    fn parse_fn_call(&mut self, span: BSpan, check_paren: bool) -> Option<token::Expr> {
        if check_paren && !self.until_open_paren() {
            return None;
        }
        let from = self.tokens.len() as u32;

        self.tokens.push(token::Token::Dummy);

        let mut expr = None;

        loop {
            let next = match dbg!(self.parse_params()) {
                // close paren
                (true, None) => break,
                (true, Some(last)) => {
                    expr = Some(last);
                    break;
                }
                // expr
                (false, Some(expr)) => expr,
                // eof
                (false, None) => {
                    self.tokens.truncate(from as usize);
                    self.push_err(LexicalError::Eof(self.token_pos()));
                    return None;
                }
            };
            self.tokens.push(next.into());
            expr = Some(next);
        }

        let to = self.tokens.len() as u32;

        self.tokens[from as usize] =
            token::Expr::FnCall(self.range(span).into(), TSpan { from, to }).into();

        expr
    }

    /// ..)
    ///
    /// Returns true when close paren found
    fn parse_params(&mut self) -> (bool, Option<token::Expr>) {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            let span = self.token_span(token.len);
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace | Comma => (),
                CloseParen => break (true, None),
                Ident => match self.next_param_kind() {
                    Either4::A(()) => break (false, self.parse_fn_call(span, false)),
                    Either4::B(()) => {
                        break (true, Some(token::Expr::Var(self.range(span).into())))
                    }
                    Either4::C(()) => {
                        break (false, Some(token::Expr::Var(self.range(span).into())))
                    }
                    Either4::D(()) => break (false, None),
                },
                Literal { kind, suffix_start } => {
                    let expr =
                        Some(token::Value::new(self.range(span).into(), kind, suffix_start).into());
                    break (false, expr);
                }
                Eof => break (false, None),
                _ => self.err_unexpected(token),
            }
        }
    }

    /// `A` = `Open`, `B` = `Close`,
    /// `C` = `Comma`, `D` = `Eof`
    fn next_param_kind(&mut self) -> Either4<(), (), (), ()> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                OpenParen => break Either4::A(()),
                CloseParen => break Either4::B(()),
                Comma => break Either4::C(()),
                Eof => break Either4::D(()),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some(self.token_span(token.len)),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => break self.parse_fn_call(span, true),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Semi => break Some(Either3::A(())),
                Eq => break Some(Either3::B(())),
                Ident => break Some(Either3::C(self.token_span(token.len))),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
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
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Semi => break true,
                Eof => {
                    self.push_err(LexicalError::MissingSemi(self.token_pos()));
                    break false;
                }
                _ => self.err_unexpected(token),
            }
        }
    }

    fn err_unexpected(&mut self, token: lex::Token) {
        self.push_err(LexicalError::Unexpected(self.token_span(token.len)));
    }

    fn err_block_comment(&mut self, token: lex::Token) -> bool {
        match token.kind {
            lex::TokenKind::BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::Unclosed(self.token_span(token.len)));
                true
            }
            _ => false,
        }
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

    #[inline]
    fn token_pos(&self) -> u32 {
        self.cursor.token_pos()
    }

    fn token_span(&self, len: u32) -> BSpan {
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
