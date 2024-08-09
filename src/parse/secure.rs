// TODO: consider adding tests to spans
use std::collections::HashMap;

use crate::{
    error::{ErrorMulti, ErrorOnce},
    lex::{self},
    parse::token,
    span::{BSpan, TSpan},
    util::Symbol,
};

/// Reads tokens into a tokenstream
#[derive(Debug)]
pub struct Reader<'a> {
    pub cursor: lex::Cursor<'a>,
    errors: ErrorMulti,
    tokens: Vec<token::Token>,
    block_spans: HashMap<u32, BSpan>,
    /// a backlog of blocks
    blocks: Vec<u32>,
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
        HashMap<u32, BSpan>,
        Vec<u32>,
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

    pub fn set_block(&mut self, token: lex::Token) {
        let Some(pos) = self.blocks.pop() else {
            self.err_unexpected(token);
            return;
        };

        self.tokens[pos as usize] = token::Token::Block(TSpan {
            from: pos,
            to: self.len() as u32,
        });

        let Some(it) = self.block_spans.get_mut(&pos) else {
            self.push_err(ErrorOnce::Other(format!(
                "invalid pos recieved while setting pos: {pos}"
            )));
            return;
        };

        it.to = self.cursor.pos();
    }

    pub fn push_block(&mut self, pos: u32) {
        self.blocks.push(pos);
        self.block_spans
            .insert(pos, BSpan::new(self.token_pos(), self.cursor.pos()));
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn truncate(&mut self, len: u32) {
        self.tokens.truncate(len as usize);
    }

    pub fn set_fn_call(&mut self, set_idx: usize, symbol: Symbol, span: TSpan) {
        self.tokens[set_idx] = token::Expr::FnCall(symbol, span).into();
    }

    pub fn set_fn_def(
        &mut self,
        set_idx: usize,
        name: BSpan,
        type_name: Option<BSpan>,
        param_span: TSpan,
        token_span: TSpan,
    ) {
        self.tokens[set_idx] = token::Fn::new(
            self.range(name).into(),
            type_name.map(|span| self.range(span).into()),
            param_span,
            token_span,
        )
        .into();
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
        let pos = self.token_pos() as usize;
        self.src()[pos..]
            .chars()
            .next()
            .expect("couldn't get current char")
    }

    #[must_use]
    pub fn current_range(&self, len: u32) -> &str {
        self.range(self.token_span(len))
    }

    #[must_use]
    pub fn range(&self, span: BSpan) -> &str {
        &self.src()[span.from as usize..span.to as usize]
    }

    #[must_use]
    pub fn symbol(&self, span: BSpan) -> Symbol {
        self.src()[span.from as usize..span.to as usize].into()
    }

    #[must_use]
    pub const fn token_pos(&self) -> u32 {
        self.cursor.token_pos()
    }

    #[must_use]
    pub const fn token_span(&self, len: u32) -> BSpan {
        BSpan::new(self.token_pos(), self.token_pos() + len)
    }
}
