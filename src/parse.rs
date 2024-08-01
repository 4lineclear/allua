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
                Ident => self.parse_ident(len),
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
                | And | Or | Plus | Star | Slash | Caret | Percent => self.unexpected_punct(len),
                // encoding err, add error and continue
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => return None,
            }
        }
    }

    fn current_char(&self, len: u32) -> char {
        let pos = self.cursor.token_pos() as usize;
        self.src[pos..pos + len as usize]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    fn parse_ident(&mut self, len: u32) {
        let from = self.cursor.token_pos() as usize;
        let to = from + len as usize;
        let id = &self.src[from..to];

        match id {
            "let" => self.parse_decl(token::DeclType::Let),
            "const" => self.parse_decl(token::DeclType::ConstType(Symbol::from(""))),
            s => todo!("unexpected ident \"{s}\""),
        }
    }

    fn parse_decl(&mut self, kind: token::DeclType) {
        match kind {
            token::DeclType::Let => {
                let name_found = self.parse_var_name();
                if !name_found {
                    self.errors
                        .push(LexicalError::NameNotFound(self.cursor.pos()));
                    return;
                }
            }
            token::DeclType::Type(_) => todo!("not implemented yet"),
            token::DeclType::ConstType(_) => todo!("not implemented yet"),
        }
    }
    fn parse_var_name(&mut self) -> bool {
        use lex::token::TokenKind::*;
        loop {
            let token = self.cursor.advance_token();
            let kind = token.kind;
            let len = token.len;

            match kind {
                // TODO: handle wrongly placed doc comments
                LineComment { .. } | BlockComment { .. } | Whitespace => (),
                Ident => return true,
                OpenBrace => {
                    todo!("code blocks not implemented yet!")
                }
                // out of place symbols
                Literal { kind, .. } => self
                    .errors
                    .push(LexicalError::UnexpectedLit(kind, self.cursor.pos())),
                Semi | CloseBrace | OpenParen | CloseParen | OpenBracket | CloseBracket | Comma
                | Dot | At | Pound | Tilde | Question | Colon | Dollar | Eq | Bang | Lt | Gt
                | Minus | And | Or | Plus | Star | Slash | Caret | Percent => {
                    self.unexpected_punct(len)
                }
                Unknown | InvalidIdent | InvalidPrefix => self
                    .errors
                    .push(LexicalError::InvalidChar(self.cursor.pos())),
                Eof => return false,
            };
        }
    }

    fn unexpected_punct(&mut self, len: u32) {
        self.errors.push(LexicalError::UnexpectedPunct(
            self.current_char(len),
            self.cursor.pos() - 1,
        ))
    }
}
