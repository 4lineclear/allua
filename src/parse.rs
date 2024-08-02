use std::rc::Rc;

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    util::Symbol,
};

pub mod iter;
#[cfg(test)]
pub mod test;
pub mod token;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader {
    cursor: lex::Cursor,
    errors: ErrorMulti,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnParseMode {
    Module,
    Fn,
}

impl From<&str> for Reader {
    fn from(value: &str) -> Self {
        Self::new(value.into())
    }
}

impl From<Rc<str>> for Reader {
    fn from(value: Rc<str>) -> Self {
        Self::new(value)
    }
}

impl From<String> for Reader {
    fn from(value: String) -> Self {
        Self::new(value.into())
    }
}

impl Reader {
    #[must_use]
    pub fn new(src: Rc<str>) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
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
        let id = self.current_range(len);

        match id {
            "let" => self.parse_decl(token::DeclKind::Let).map(Into::into),
            "const" => self.parse_decl(token::DeclKind::Const).map(Into::into),
            s => self.parse_unknown_ident(s),
        }
    }

    fn parse_unknown_ident(&self, s: &str) -> Option<token::Token> {
        todo!("{s}")
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
        let Some(name) = self.parse_until_ident() else {
            self.push_err(LexicalError::NameNotFound(self.cursor.pos()));
            return None;
        };

        let name = name.to_owned();
        let is_semi = self.eq_or_semi()?;

        Some((name.into(), if is_semi { None } else { self.parse_expr() }))
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

    fn parse_until_ident(&mut self) -> Option<&str> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some(self.current_range(token.len)),
                Eof => return None,
                _ => self.filter_all(token),
            }
        }
    }

    fn parse_expr(&mut self) -> Option<token::Expr> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                _ if self.filter_block_comment(token) => (),
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

        let pos = self.cursor.pos();
        match token.kind {
            BlockComment { terminated, .. } if !terminated => (),
            _ => return false,
        }
        self.push_err(LexicalError::UnclosedBlockComment(pos));
        true
    }

    fn current_char(&self) -> char {
        let pos = self.cursor.token_pos() as usize;
        self.src()[pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    fn current_range(&self, len: u32) -> &str {
        let pos = self.cursor.token_pos() as usize;
        &self.src()[pos..pos + len as usize]
    }

    fn push_err(&mut self, err: impl Into<ErrorOnce>) {
        self.errors.push(err);
    }
}
