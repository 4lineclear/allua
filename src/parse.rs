use crate::{
    error::{ErrorMulti, LexicalError},
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
    // buf: Vec<token::Token>,
}

#[derive(PartialEq)]
pub enum Mode {
    Module,
    Fn,
}

impl<'a> Reader<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
            src,
            // buf: Vec::new(),
        }
    }

    /// Parse a module
    pub fn module(&mut self, name: &str) -> token::Module {
        let mut md = token::Module::new(name);

        loop {
            match self.next(Mode::Module) {
                Some(token) => md.push(token),
                None => break,
            };
        }

        md
    }

    /// The top level parsing function; parses the next token from within a fn
    ///
    /// Parses both functions and modules, catching lexical errors
    #[allow(dead_code)]
    #[allow(unused)]
    pub fn next(&mut self, mode: Mode) -> Option<token::Token> {
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
                        self.errors
                            .push(LexicalError::UnclosedBlockComment(self.cursor.token_pos()));
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
                CloseBrace if mode == Mode::Fn => return None,
                // out of place symbol, add error and continue
                CloseBrace | OpenParen | CloseParen | OpenBracket | CloseBracket | Comma | Dot
                | At | Pound | Tilde | Question | Colon | Dollar | Eq | Bang | Lt | Gt | Minus
                | And | Or | Plus | Star | Slash | Caret | Percent => self.unexpected_punct(),
                // encoding err, add error and continue
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => return None,
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
            "const" => self
                .parse_decl(token::DeclKind::ConstType(Symbol::from("")))
                .map(Into::into),
            s => todo!("unexpected ident \"{s}\""),
        }
    }

    fn parse_decl(&mut self, kind: token::DeclKind) -> Option<token::Decl> {
        match kind {
            token::DeclKind::Let => self.parse_let(),
            token::DeclKind::Type(_) => todo!("not implemented yet"),
            token::DeclKind::Const(_) => todo!("not implemented yet"),
            token::DeclKind::ConstType(_) => todo!("not implemented yet"),
        }
        .map(|(name, value)| (token::Decl::new(kind, name, value)))
    }

    fn parse_let(&mut self) -> Option<(Symbol, Option<token::Expr>)> {
        use lex::token::TokenKind::*;
        let name = self.parse_until_ident();
        let Some(name) = name else {
            self.errors
                .push(LexicalError::NameNotFound(self.cursor.pos()));
            return None;
        };

        // should be '=' or ';'
        loop {
            let next = self.cursor.advance_token();
            match next.kind {
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                // uninit var
                Semi => return Some((name, None)),
                // init var
                Eq => break,
                // out of place symbols
                Ident => self.errors.push(LexicalError::UnexpectedIdent(
                    self.current_range(next.len).into(),
                    self.cursor.pos(),
                )),
                Literal { kind, .. } => self
                    .errors
                    .push(LexicalError::UnexpectedLit(kind, self.cursor.pos())),
                OpenBrace | CloseBrace | OpenParen | CloseParen | OpenBracket | CloseBracket
                | Comma | Dot | At | Pound | Tilde | Question | Colon | Dollar | Bang | Lt | Gt
                | Minus | And | Or | Plus | Star | Slash | Caret | Percent => {
                    self.unexpected_punct()
                }
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => (),
            }
        }
        let expr = self.parse_expr();
        if expr.is_none() {
            return None;
        }
        Some((name, expr))
    }

    fn parse_until_ident(&mut self) -> Option<Symbol> {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            match token.kind {
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return Some(self.current_range(token.len).into()),
                // out of place symbols
                Literal { kind, .. } => self
                    .errors
                    .push(LexicalError::UnexpectedLit(kind, self.cursor.pos())),
                Semi | OpenBrace | CloseBrace | OpenParen | CloseParen | OpenBracket
                | CloseBracket | Comma | Dot | At | Pound | Tilde | Question | Colon | Dollar
                | Eq | Bang | Lt | Gt | Minus | And | Or | Plus | Star | Slash | Caret
                | Percent => self.unexpected_punct(),
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => return None,
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
                // out of place symbols
                Semi | OpenBrace | CloseBrace | OpenParen | CloseParen | OpenBracket
                | CloseBracket | Comma | Dot | At | Pound | Tilde | Question | Colon | Dollar
                | Eq | Bang | Lt | Gt | Minus | And | Or | Plus | Star | Slash | Caret
                | Percent => self.unexpected_punct(),
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => return None,
            }
        }
    }

    fn unexpected_punct(&mut self) {
        self.errors.push(LexicalError::UnexpectedPunct(
            self.current_char(),
            self.cursor.pos() - 1,
        ))
    }
}
