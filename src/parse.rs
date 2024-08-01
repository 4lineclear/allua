use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
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
    src: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnParseMode {
    Module,
    Fn,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
            src,
        }
    }

    /// Parse a module
    pub fn module(&mut self, name: &str) -> token::Module {
        let mut md = token::Module::new(name);
        while let Some(token) = self.next(FnParseMode::Module) {
            md.push(token);
        }
        md
    }

    /// The top level parsing function; parses the next token from within a fn
    ///
    /// Parses both functions and modules, catching lexical errors
    #[allow(unused)]
    pub fn next(&mut self, mode: FnParseMode) -> Option<token::Token> {
        use lex::token::TokenKind::*;

        loop {
            let token = self.cursor.advance_token();
            let len = token.len;
            let kind = token.kind;
            match kind {
                // (?doc)comments. skip normal comments
                LineComment { doc_style } => {
                    if let Some(style) = doc_style {
                        todo!("doc comments not added yet");
                    }
                }
                BlockComment {
                    doc_style,
                    terminated,
                } => {
                    if !terminated {
                        self.push_err(LexicalError::UnclosedBlockComment(self.cursor.token_pos()));
                    }
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
                Literal { kind, suffix_start } => {}
                // code block
                OpenBrace => {
                    todo!("code blocks not implemented yet!")
                }
                // fn end
                CloseBrace if mode == FnParseMode::Fn => return None,
                // encoding err, add error and continue
                Unknown | InvalidIdent | InvalidPrefix => {
                    self.push_err(LexicalError::InvalidChar(self.cursor.pos()));
                }
                Eof => return None,
                // out of place symbol, add error and continue
                _ => {
                    self.filter_punct(token);
                }
            }
        }
    }

    fn current_char(&self) -> char {
        let pos = self.cursor.token_pos() as usize;
        self.src[pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    fn current_range(&self, len: u32) -> &str {
        let pos = self.cursor.token_pos() as usize;
        &self.src[pos..pos + len as usize]
    }

    fn parse_ident(&mut self, len: u32) -> Option<token::Token> {
        let from = self.cursor.token_pos() as usize;
        let to = from + len as usize;
        let id = &self.src[from..to];

        match id {
            "let" => self.parse_decl(token::DeclKind::Let).map(Into::into),
            "const" => self.parse_decl(token::DeclKind::Const).map(Into::into),
            s => todo!("unexpected ident \"{s}\""),
        }
    }

    fn parse_decl(&mut self, kind: token::DeclKind) -> Option<token::Decl> {
        match kind {
            token::DeclKind::Let => self.parse_let(),
            token::DeclKind::Type(_) => todo!("not implemented yet"),
            token::DeclKind::Const | token::DeclKind::ConstType(_) => todo!("not implemented yet"),
        }
        .map(|(name, value)| (token::Decl::new(kind, name, value)))
    }

    fn parse_let(&mut self) -> Option<(Symbol, Option<token::Expr>)> {
        let name = self.parse_until_ident();
        let Some(name) = name else {
            self.push_err(LexicalError::NameNotFound(self.cursor.pos()));
            return None;
        };

        let semi = self.eq_or_semi(name);
        if semi.is_some() {
            semi
        } else {
            let expr = self.parse_expr();
            expr?;
            Some((name, expr))
        }
    }

    pub fn eq_or_semi(&mut self, name: ustr::Ustr) -> Option<(ustr::Ustr, Option<token::Expr>)> {
        use lex::token::TokenKind::*;
        loop {
            let next = self.cursor.advance_token();
            match next.kind {
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                // uninit var
                Semi => break Some((name, None)),
                // init var
                Eq => break None,
                _ if self.filter_ident(next) => (),
                _ if self.filter_lit(next) => (),
                _ if self.filter_invalid(next) => (),
                _ if self.filter_eof(next) => (),
                _ if self.filter_punct(next) => (),
                _ => unreachable!(),
            }
        }
    }

    fn parse_until_ident(&mut self) -> Option<Symbol> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some(self.current_range(token.len).into()),
                Eof => return None,
                _ if self.filter_lit(token) => (),
                _ if self.filter_invalid(token) => (),
                _ if self.filter_punct(token) => (),
                _ => unreachable!(),
            }
        }
    }

    fn parse_expr(&mut self) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => todo!("only literals allowed for now"),
                Literal { kind, suffix_start } => {
                    return Some(token::Expr::Value(token::Value::new(
                        self.current_range(token.len).into(),
                        kind,
                        suffix_start,
                    )))
                }
                Eof => return None,
                _ if self.filter_invalid(token) => (),
                _ if self.filter_punct(token) => (),
                _ => unreachable!(),
            }
        }
    }

    #[inline]
    fn filter_punct(&mut self, token: lex::Token) -> bool {
        use lex::token::TokenKind::*;
        if let Semi | OpenBrace | CloseBrace | OpenParen | CloseParen | OpenBracket | CloseBracket
        | Comma | Dot | At | Pound | Tilde | Question | Colon | Dollar | Eq | Bang | Lt
        | Gt | Minus | And | Or | Plus | Star | Slash | Caret | Percent = token.kind
        {
            self.push_err(LexicalError::UnexpectedPunct(
                self.current_char(),
                self.cursor.pos() - 1,
            ));
            true
        } else {
            false
        }
    }

    #[inline]
    fn filter_eof(&mut self, token: lex::Token) -> bool {
        use lex::token::TokenKind::*;
        if let Eof = token.kind {
            self.push_err(LexicalError::UnexpectedEof(self.cursor.pos()));
            true
        } else {
            false
        }
    }

    #[inline]
    fn filter_ident(&mut self, token: lex::Token) -> bool {
        use lex::token::TokenKind::*;
        if let Ident = token.kind {
            self.push_err(LexicalError::UnexpectedIdent(
                self.current_range(token.len).into(),
                self.cursor.pos(),
            ));
            true
        } else {
            false
        }
    }

    #[inline]
    fn filter_lit(&mut self, token: lex::Token) -> bool {
        use lex::token::TokenKind::*;
        if let Literal { kind, .. } = token.kind {
            self.push_err(LexicalError::UnexpectedLit(kind, self.cursor.pos()));
            true
        } else {
            false
        }
    }

    #[inline]
    fn filter_invalid(&mut self, token: lex::Token) -> bool {
        use lex::token::TokenKind::*;
        if let Unknown | InvalidIdent | InvalidPrefix = token.kind {
            self.push_err(LexicalError::InvalidChar(self.cursor.pos()));
            true
        } else {
            false
        }
    }

    fn push_err(&mut self, err: impl Into<ErrorOnce>) {
        self.errors.push(err);
    }
}
