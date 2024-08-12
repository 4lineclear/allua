// TODO: consider adding tests to spans
use std::collections::HashMap;

use crate::{
    error::{ErrorMulti, ErrorOnce},
    lex,
    parse::token,
    span::{BSpan, TSpan},
    util::Symbol,
};

use super::AsBSpan;

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    pub cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    tokens: Vec<token::Token>,
    block_spans: HashMap<usize, BSpan>,
    /// a backlog of blocks
    blocks: Vec<usize>,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: lex::Cursor::new(src),
            errors: ErrorMulti::default(),
            tokens: Vec::new(),
            block_spans: HashMap::new(),
            blocks: Vec::new(),
        }
    }

    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        lex::Cursor<'a>,
        ErrorMulti,
        Vec<token::Token>,
        HashMap<usize, BSpan>,
        Vec<usize>,
    ) {
        let Reader {
            cursor,
            errors,
            tokens,
            block_spans,
            blocks,
        } = self;

        (cursor, errors, tokens, block_spans, blocks)
    }

    pub fn set_block(&mut self, token: lex::Lexeme) {
        let Some(pos) = self.blocks.pop() else {
            use lex::LexKind::*;
            self.err_expected(token, [Ident, RawIdent, OpenBrace, Eof]);
            // NOTE: should be the same as Self::next_or_close_brace
            return;
        };

        self.tokens[pos] = token::Token::Block(TSpan {
            from: pos,
            to: self.len(),
        });

        let Some(it) = self.block_spans.get_mut(&pos) else {
            self.push_err(ErrorOnce::Other(format!(
                "invalid pos recieved while setting pos: {pos}"
            )));
            return;
        };

        it.to = self.cursor.pos();
    }

    pub fn push_block(&mut self, pos: usize) {
        self.blocks.push(pos);
        self.block_spans
            .insert(pos, BSpan::new(self.token_pos(), self.cursor.pos()));
    }

    #[must_use]
    pub fn blocks_left(&self) -> bool {
        !self.blocks.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn truncate(&mut self, len: usize) {
        self.tokens.truncate(len);
    }

    /// Replace the given index with the given token
    ///
    /// # Panics
    ///
    /// Invalid index given or, when in debug, token at index was not dummy
    pub fn set_at(&mut self, set_idx: usize, token: impl Into<token::Token>) {
        debug_assert_eq!(self.tokens[set_idx], token::Token::Dummy);
        self.tokens[set_idx] = token.into();
    }

    pub fn push_err(&mut self, err: impl Into<ErrorOnce>) {
        self.errors.push(err);
    }

    pub fn push_token(&mut self, token: impl Into<token::Token>) {
        self.tokens.push(token.into());
    }

    pub fn pop_token(&mut self) -> Option<token::Token> {
        self.tokens.pop()
    }

    #[must_use]
    pub const fn src(&self) -> &str {
        self.cursor.src()
    }

    #[allow(dead_code)]
    #[must_use]
    fn current_char(&self) -> char {
        let pos = self.token_pos();
        self.src()[pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    #[must_use]
    pub fn current_range(&self, len: usize) -> &str {
        self.range(self.token_span(len))
    }

    pub fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Token(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }

    #[must_use]
    pub fn range(&self, span: impl Into<AsBSpan>) -> &str {
        let span = self.span(span);
        &self.src()[span.from..span.to]
    }

    #[must_use]
    pub fn symbol(&self, span: BSpan) -> Symbol {
        self.src()[span.from..span.to].into()
    }

    #[must_use]
    pub fn get_token(&self, index: usize) -> Option<token::Token> {
        self.tokens.get(index).copied()
    }

    #[must_use]
    pub fn last_token(&self) -> Option<token::Token> {
        self.tokens.last().copied()
    }

    #[must_use]
    pub const fn token_pos(&self) -> usize {
        self.cursor.token_pos()
    }

    #[must_use]
    const fn token_span(&self, len: usize) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
}
