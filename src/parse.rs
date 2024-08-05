use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    span::TSpan,
    util::Symbol,
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

    /// The top level parsing function; parses the next token from within a fn
    ///
    /// Parses both functions and modules, catching lexical errors
    #[inline]
    pub fn next(&mut self, mode: ParseMode) -> Option<token::Token> {
        match mode {
            ParseMode::Module => self.fn_next(false),
            ParseMode::Fn => self.fn_next(true),
        }
    }

    #[allow(unused)]
    fn fn_next(&mut self, is_fn: bool) -> Option<token::Token> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            let len = token.len;
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
                Ident => match self.parse_ident(len) {
                    Some(token) => return Some(token),
                    None => (),
                },
                // code block
                OpenBrace => {
                    todo!("code blocks not implemented yet!")
                }
                // fn end
                CloseBrace if is_fn => return None,
                Eof => return None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn parse_ident(&mut self, len: u32) -> Option<token::Token> {
        match self.current_range(len) {
            "let" => self.parse_decl(token::DeclKind::Let).map(Into::into),
            "const" => self.parse_decl(token::DeclKind::Const).map(Into::into),
            _ => self.parse_unnested_ident(len),
        }
    }

    // NOTE: unnested ident can only be one of:
    // - Decl
    // - Expr
    fn parse_unnested_ident(&mut self, len: u32) -> Option<token::Token> {
        let type_pos = self.token_pos();
        let type_len = len;

        if let Either::B(len) = self.open_paren_or_ident()? {
            let name_len = len;
            let name_pos = self.token_pos();

            let value = if self.expect_eq_or_semi()? {
                None
            } else {
                let expr = self.parse_expr();
                self.expect_semi();
                expr
            };

            Some(
                token::Decl::new(
                    token::DeclKind::Type(self.range(type_pos, type_len).into()),
                    self.range(name_pos, name_len).into(),
                    value,
                )
                .into(),
            )
        } else {
            let from = self.tokens.len() as u32;
            self.tokens.push(token::Token::Dummy);
            let last = self.parse_fn_call();
            let to = self.tokens.len() as u32 - 1;
            self.tokens[from as usize] =
                token::Expr::FnCall(self.range(type_pos, type_len).into(), TSpan { from, to })
                    .into();
            self.expect_semi();
            last.map(Into::into)
        }
    }

    /// `A` = `Paren`, `B` = `Ident`
    fn open_paren_or_ident(&mut self) -> Option<Either<(), u32>> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                OpenParen => break Some(Either::A(())),
                Ident => break Some(Either::B(token.len)),
                Eof => break None,
                _ => self.err_unexpected(token),
            }
        }
    }

    // NOTE: unnested ident can only be Expr
    fn parse_nested_ident(&mut self, len: u32) -> Option<token::Token> {
        let type_pos = self.token_pos();
        let type_len = len;

        match self.open_or_close_paren_or_comma()? {
            Either3::A(()) => {
                let from = self.tokens.len() as u32;
                self.tokens.push(token::Token::Dummy);
                let last = self.parse_fn_call();
                let to = self.tokens.len() as u32 - 1;
                self.tokens[from as usize] =
                    token::Expr::FnCall(self.range(type_pos, type_len).into(), TSpan { from, to })
                        .into();
                self.expect_semi();
                last.map(Into::into)
            }
            Either3::B(()) => Some(token::Expr::Var(self.range(type_pos, type_len).into()).into()),
            Either3::C(()) => Some(token::Expr::Var(self.range(type_pos, type_len).into()).into()),
        }
    }

    /// `A` = `Open`, `B` = `Close`, `C` = `Comma`
    fn open_or_close_paren_or_comma(&mut self) -> Option<Either3<(), (), ()>> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                OpenParen => break Some(Either3::A(())),
                CloseParen => break Some(Either3::B(())),
                Comma => break Some(Either3::C(())),
                Eof => break None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn parse_fn_call(&mut self) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        let mut last = None;

        loop {
            let token = self.cursor.advance_token();
            if let CloseParen | Eof = token.kind {
                break last;
            }
            let expr = self.parse_expr_with(token);
            if let Some(expr) = expr {
                self.tokens.push(expr.into());
                last = Some(expr);
            }
        }
    }

    fn parse_decl(&mut self, kind: token::DeclKind) -> Option<token::Decl> {
        match kind {
            token::DeclKind::Let => self.parse_let(),
            token::DeclKind::Const => self.parse_let(),
            n => unreachable!("\"{n:?}\" should never appear here"),
        }
        .map(|(name, value)| (token::Decl::new(kind, name, value)))
    }

    /// parses the tokens follow `let` | `const`
    fn parse_let(&mut self) -> Option<(Symbol, Option<token::Expr>)> {
        let Some((pos, len)) = self.until_ident() else {
            self.push_err(LexicalError::NameNotFound(self.cursor.pos()));
            return None;
        };

        let is_semi = self.expect_eq_or_semi()?;

        Some((
            self.range(pos, len).into(),
            if is_semi {
                None
            } else {
                let expr = self.parse_expr();
                self.expect_semi();
                expr
            },
        ))
    }

    /// Parses until an ident, returns the byte position
    fn until_ident(&mut self) -> Option<(u32, u32)> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some((self.token_pos(), token.len)),
                Eof => return None,
                _ => self.err_unexpected(token),
            }
        }
    }

    fn parse_expr(&mut self) -> Option<token::Expr> {
        let token = self.cursor.advance_token();
        self.parse_expr_with(token)
    }

    fn parse_expr_with(&mut self, mut token: lex::Token) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        loop {
            match token.kind {
                _ if self.err_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => match self.parse_nested_ident(token.len) {
                    Some(token) => self.tokens.push(token),
                    None => (),
                },
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

    fn expect_eq_or_semi(&mut self) -> Option<bool> {
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

    fn expect_semi(&mut self) -> bool {
        if let Some(true) = self.expect_eq_or_semi() {
            self.errors
                .push(LexicalError::MissingSemi(self.cursor.pos()));
            true
        } else {
            false
        }
    }

    fn err_unexpected(&mut self, token: lex::Token) {
        let start = self.token_pos();
        let end = start + token.len;
        self.push_err(LexicalError::Unexpected(start, end));
    }

    fn err_block_comment(&mut self, token: lex::Token) -> bool {
        match token.kind {
            lex::TokenKind::BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::UnclosedBlockComment(self.token_pos()));
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
        let pos = self.token_pos() as usize;
        &self.src()[pos..pos + len as usize]
    }

    fn range(&self, pos: u32, len: u32) -> &str {
        let pos = pos as usize;
        &self.src()[pos..pos + len as usize]
    }

    fn token_pos(&self) -> u32 {
        self.cursor.token_pos()
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
