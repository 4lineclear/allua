use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    util::Symbol,
};

// TODO: add raw idents back in eventually.
// TODO: simplify to_string

#[cfg(test)]
pub mod test;
pub mod token;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    buf: Vec<token::Token>,
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
            buf: Vec::new(),
        }
    }

    #[must_use]
    #[inline]
    pub fn src(&self) -> &str {
        self.cursor.src()
    }

    /// Parse a module
    pub fn module(&mut self, name: &str) -> token::Module {
        let mut md = token::Module::new(name);
        while let Some(token) = self.next_inner(FnParseMode::Module) {
            md.push(token);
        }
        md
    }

    pub fn next(&mut self, mode: FnParseMode) -> Option<token::Token> {
        let token = self.next_inner(mode);
        if let Some(token) = token {
            self.buf.push(token);
        }
        token
    }

    /// The top level parsing function; parses the next token from within a fn
    ///
    /// Parses both functions and modules, catching lexical errors
    #[inline]
    #[allow(unused)]
    pub fn next_inner(&mut self, mode: FnParseMode) -> Option<token::Token> {
        use lex::token::TokenKind::*;

        loop {
            let token = self.cursor.advance_token();
            let len = token.len;
            let kind = token.kind;
            match kind {
                _ if self.filter_block_comment(token) => (),
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
                CloseBrace if mode == FnParseMode::Fn => return None,
                Eof => return None,
                _ => self.filter_all(token),
            }
        }
    }

    fn parse_ident(&mut self, len: u32) -> Option<token::Token> {
        match self.current_range(len) {
            "let" => self.parse_decl(token::DeclKind::Let).map(Into::into),
            "const" => self.parse_decl(token::DeclKind::Const).map(Into::into),
            _ => self.parse_unknown_ident(len),
        }
    }

    // TODO: create proper bytespan/cursorspan type
    fn parse_unknown_ident(&mut self, len: u32) -> Option<token::Token> {
        let type_pos = self.token_pos();
        let type_len = len;

        // ident, should be a type decl
        if let Either::B(len) = self.paren_or_ident()? {
            let name_pos = self.token_pos();
            let name_len = len;
            let is_semi = self.eq_or_semi()?;

            Some(
                token::Decl::new(
                    token::DeclKind::Type(self.range(type_pos, type_len).into()),
                    self.range(name_pos, name_len).into(),
                    if is_semi {
                        None
                    } else {
                        let expr = self.parse_expr();
                        self.parse_semi();
                        expr
                    },
                )
                .into(),
            )
        } else {
            self.parse_fn();
            todo!()
        }
    }

    fn parse_fn(&mut self) -> Option<()> {
        // debug_assert_eq!(self.current_char(), '(');
        use lex::token::TokenKind::*;
        let close_found = loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace | Ident | Comma => (),
                CloseParen => break true,
                Eof => break false,
                _ => self.filter_all(token),
            }
        };

        if close_found {
        } else {
        }

        todo!()
    }

    fn paren_or_ident(&mut self) -> Option<Either<(), u32>> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => break Some(Either::B(token.len)),
                OpenParen => break Some(Either::A(())),
                Eof => break None,
                _ => self.filter_all(token),
            }
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
        let Some((pos, len)) = self.parse_until_ident() else {
            self.push_err(LexicalError::NameNotFound(self.cursor.pos()));
            return None;
        };

        let is_semi = self.eq_or_semi()?;

        Some((
            self.range(pos, len).into(),
            if is_semi {
                None
            } else {
                let expr = self.parse_expr();
                self.parse_semi();
                expr
            },
        ))
    }

    pub fn eq_or_semi(&mut self) -> Option<bool> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Semi => break Some(true),
                Eq => break Some(false),
                Eof => break None,
                _ => self.filter_all(token),
            }
        }
    }

    fn parse_until_ident(&mut self) -> Option<(u32, u32)> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some((self.token_pos(), token.len)),
                Eof => return None,
                _ => self.filter_all(token),
            }
        }
    }

    // TODO: add ident parsing
    fn parse_expr(&mut self) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => todo!("only literals allowed for now"),
                Literal { kind, suffix_start } => {
                    break Some(token::Expr::Value(token::Value::new(
                        self.current_range(token.len).into(),
                        kind,
                        suffix_start,
                    )))
                }
                Eof => break None,
                _ => self.filter_all(token),
            }
        }
    }

    fn parse_semi(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Eof => {
                    self.errors
                        .push(LexicalError::MissingSemi(self.cursor.pos(), token.len));
                    break false;
                }
                Semi => break true,
                _ => self.filter_all(token),
            }
        }
    }

    pub fn filter_all(&mut self, token: lex::Token) {
        use lex::TokenKind::*;
        use LexicalError::*;
        let pos = self.cursor.pos();
        self.push_err(match token.kind {
            LineComment { doc_style } | BlockComment { doc_style, .. } => {
                UnexpectedComment(doc_style, pos)
            }
            Whitespace => UnexpectedWhitespace(pos),
            Literal { kind, .. } => UnexpectedLit(kind, pos),
            Semi | Comma | Dot | OpenParen | CloseParen | OpenBrace | CloseBrace | OpenBracket
            | CloseBracket | At | Pound | Tilde | Question | Colon | Dollar | Eq | Bang | Lt
            | Gt | Minus | And | Or | Plus | Star | Slash | Caret | Percent => {
                UnexpectedPunct(self.current_char(), pos - 1)
            }
            Ident => UnexpectedIdent(self.current_range(token.len).into(), pos),
            InvalidIdent | InvalidPrefix | Unknown => InvalidChar(pos),
            Eof => UnexpectedEof(pos),
        });
    }

    fn filter_block_comment(&mut self, token: lex::Token) -> bool {
        use lex::TokenKind::*;
        match token.kind {
            BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::UnclosedBlockComment(self.token_pos()));
                true
            }
            _ => false,
        }
    }

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
