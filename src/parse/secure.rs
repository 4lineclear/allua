// TODO: consider adding tests to spans
use std::collections::VecDeque;

use crate::{
    error::{ErrorMulti, ErrorOnce},
    lex::{Cursor, Lexeme},
    parse::{self, ExprKind, LITERAL},
    span::{BSpan, TSpan},
    util::Symbol,
};

use super::{token::Token, AsBSpan, AsStr, Expr, LexKind};

/// Reads tokens into a tokenstream
#[derive(Debug, Default)]
pub struct Reader<'a> {
    pub cursor: Cursor<'a>,
    errors: ErrorMulti,
    tokens: Vec<Token>,
    /// a backlog of blocks
    blocks: Vec<(usize, BSpan)>,
    /// a backlog of control flows
    flows: VecDeque<usize>,
    /// a backlog of idents
    exprs: VecDeque<usize>,
}

impl<'a> Reader<'a> {
    #[must_use]
    pub fn new(src: &'a str) -> Self {
        Self {
            cursor: Cursor::new(src),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn into_parts(self) -> (Cursor<'a>, ErrorMulti, Vec<Token>, Vec<(usize, BSpan)>) {
        let Reader {
            cursor,
            errors,
            tokens,
            blocks,
            flows: _,
            exprs,
        } = self;
        // println!("{exprs:#?}");

        (cursor, errors, tokens, blocks)
    }

    pub fn dummy(&mut self) -> usize {
        let idx = self.len();
        self.tokens.push(Token::Dummy);
        idx
    }

    pub fn set_block(&mut self, token: Lexeme) {
        let Some((pos, _)) = self.blocks.pop() else {
            self.err_expected(token, parse::EXPECTED);
            return;
        };

        self.tokens[pos] = Token::Block(TSpan {
            from: pos,
            to: self.len(),
        });
    }

    pub fn push_block(&mut self, pos: usize) {
        self.blocks
            .push((pos, BSpan::new(self.lex_pos(), self.cursor.pos())));
    }

    /// returns `Err(false)` if compiler error
    pub fn get_expr(&mut self) -> Result<(usize, Expr), bool> {
        let pos = self.first_ident().ok_or(true)?;
        // println!("checking pos: {pos}");
        match self.get_token(pos) {
            // Some(Token::Expr(expr)) if expr.end != pos => match self.get_token(expr.end - 1) {
            //     Some(Token::Expr(expr)) => Ok((pos, expr)),
            //     Some(other) => {
            //         self.compiler_error(format!(
            //             "non expr token found at end {}: {other:#?}",
            //             expr.end
            //         ));
            //         Err(false)
            //     }
            //     None => {
            //         self.compiler_error(format!("no token found at end {}", expr.end));
            //         Err(false)
            //     }
            // },
            Some(Token::Expr(expr)) => Ok((pos, expr)),
            Some(other) => {
                self.compiler_error(format!("non expr token found at pos {pos}: {other:#?}"));
                Err(false)
            }
            None => {
                self.compiler_error(format!("no token found at pos {pos}"));
                Err(false)
            }
        }
    }

    pub fn push_ident(&mut self, symbol: impl Into<AsStr<'a>>) {
        // use LexKind::*;
        let pos = self.len();
        self.push_expr(ExprKind::Var(self.symbol(symbol)));
        // println!("{:#?}", self.tokens);
        if let Some((pos, mut expr)) = self.get_expr().ok() {
            // println!("another one: {pos}");
            let ExprKind::FnCall(call) = &mut expr.kind else {
                return;
            };
            if expr.end != pos {
                // TODO: error handling here
                if expr.end + 1 != self.len() {
                    // println!("DAMN 1");
                    return;
                }
                if !call.comma {
                    // println!("DAMN 2");
                    return;
                }
            }
            call.comma = false;
            expr.end = self.len();
            self.set_at(pos, expr);
            // println!("{self:#?}");
        }
        self.exprs.push_back(pos);
    }

    pub fn pop_ident(&mut self) -> Option<usize> {
        self.exprs.pop_front()
    }

    pub fn first_ident(&mut self) -> Option<usize> {
        self.exprs.front().copied()
    }

    pub fn push_flow(&mut self, pos: usize) {
        self.flows.push_back(pos);
    }

    // TODO: rename this
    pub fn last_flow(&mut self, run: impl Fn(&mut Self, usize) -> bool) -> bool {
        let Some(&flow) = self.flows.front() else {
            return false;
        };
        let used = run(self, flow);
        if used {
            self.flows.pop_front();
        }
        true
    }

    #[must_use]
    pub fn blocks_left(&self) -> bool {
        !self.blocks.is_empty()
    }

    #[inline]
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
    pub fn set_at(&mut self, set_idx: usize, token: impl Into<Token>) {
        self.tokens[set_idx] = token.into();
    }

    pub fn push_err(&mut self, err: impl Into<ErrorOnce>) {
        self.errors.push(err);
    }

    pub fn compiler_error(&mut self, err: impl Into<String>) {
        self.errors.push(ErrorOnce::Other(err.into()));
    }

    pub fn push_token(&mut self, token: impl Into<Token>) {
        self.tokens.push(token.into());
    }

    /// push an expr
    ///
    /// # NOTE
    ///
    /// The span inputted with this will always be empty, so this
    /// function should only be used by unit items.
    pub fn push_expr(&mut self, kind: impl Into<ExprKind>) {
        self.push_token(Expr {
            end: self.len(),
            kind: kind.into(),
        });
    }

    pub fn pop_token(&mut self) -> Option<Token> {
        self.tokens.pop()
    }

    #[must_use]
    pub const fn src(&self) -> &str {
        self.cursor.src()
    }

    pub fn span(&self, span: impl Into<AsBSpan>) -> BSpan {
        match span.into() {
            AsBSpan::Len(len) => self.token_span(len),
            AsBSpan::Lex(token) => self.token_span(token.len),
            AsBSpan::Span(span) => span,
        }
    }

    #[must_use]
    pub fn str(&self, s: impl Into<AsStr<'a>>) -> &str {
        use AsStr::*;
        match s.into() {
            Span(s) => {
                let span = self.span(s);
                &self.src()[span.from..span.to]
            }
            Symbol(s) => s.as_str(),
            Str(s) => s,
        }
    }

    #[must_use]
    pub fn symbol(&self, s: impl Into<AsStr<'a>>) -> Symbol {
        self.str(s).into()
    }

    #[must_use]
    pub fn get_token(&self, index: usize) -> Option<Token> {
        self.tokens.get(index).copied()
    }

    #[must_use]
    pub fn last_token(&self) -> Option<Token> {
        self.tokens.last().copied()
    }

    #[must_use]
    pub const fn lex_pos(&self) -> usize {
        self.cursor.lex_pos()
    }

    #[must_use]
    const fn token_span(&self, len: usize) -> BSpan {
        BSpan::new(self.lex_pos(), self.lex_pos() + len)
    }
}
