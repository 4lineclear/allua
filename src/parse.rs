// TODO: create pattern-composer macro
// TODO: consider adding system where doc comments can be anywhere?
// maybe change how doc comments are considered compared to rust.
// TODO: consider rewriting everything
// TODO: add tuples
// TODO: allow for parsing code blocks in other areas.
// code block
// TODO: create a compiler error type.
// TODO: add visibility item to Fn
// TODO: consider unifying the "different kinds" of expr syntax into one
// TODO: add operators
// TODO: have parser fail fast
// TODO: test fail fast changes
// TODO: consider renaming lex::Token && lex::TokenKind
// TODO: move most err_unexpected to err_expected.
// TODO: use this: https://github.com/marketplace/actions/todo-actions
// TODO: go back to using semicolons everywhere? either use semi or make
// single item tupls "(item)" how single value exprs can be returned?
// FIX: expr system is kinda broken, should fix before adding operators
// lots of this above is addressed by this
#![allow(clippy::cast_possible_truncation)]

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::{self},
    span::{BSpan, TSpan},
};

pub use secure::Reader;

/// a secure module for keeping certain fields safe.
mod secure;
#[cfg(test)]
pub mod test;
pub mod token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    Module,
    Fn,
}

impl<'a> Reader<'a> {
    /// Parse a module
    #[must_use]
    pub fn module(mut self, name: &str) -> (token::Module, ErrorMulti) {
        while self.next() {}
        let (cursor, mut errors, mut tokens, spans, blocks) = self.into_parts();

        for pos in blocks {
            let Some(span) = spans.get(&pos) else {
                errors.push(ErrorOnce::Other(format!(
                    "found pos in block backlog that was out of bounds: {pos}"
                )));
                continue;
            };
            let span = BSpan::new(span.from, cursor.pos());
            errors.push(LexicalError::Unclosed(span));
            tokens.truncate(pos);
        }

        (token::Module::new(name, tokens), errors)
    }

    fn next(&mut self) -> bool {
        let token = self.cursor.advance_token();
        match self.next_or_close_brace(token) {
            Either3::A(()) => false,
            Either3::B(()) => true,
            Either3::C(()) => {
                self.set_block(token);
                true
            }
        }
    }

    /// `A` = `Eof` `B` = `Token` `C` = `CloseBrace`
    fn next_or_close_brace(&mut self, token: lex::Token) -> Either3<(), (), ()> {
        use lex::token::TokenKind::*;
        let span = self.span(token);
        let kind = token.kind;
        match kind {
            // (?doc)comments or whitespace. skip normal comments
            _ if self.filter_comment_or_whitespace(token) => (),
            Ident | RawIdent => self.parse_ident(span),
            OpenBrace => {
                self.push_block(self.len());
                self.push_token(token::Token::Dummy);
            }
            // code block end
            CloseBrace => return Either3::C(()),
            Eof => return Either3::A(()),
            _ => match self.blocks_left() {
                true => self.err_expected(token, [Ident, RawIdent, OpenBrace, CloseBrace, Eof]),
                false => self.err_expected(token, [Ident, RawIdent, OpenBrace, Eof]),
            },
        };

        Either3::B(())
    }

    fn parse_ident(&mut self, span: BSpan) {
        match self.range(span) {
            "let" => {
                self.parse_decl(token::DeclKind::Let);
            }
            "const" => {
                self.parse_decl(token::DeclKind::Const);
            }
            "fn" => {
                self.parse_fn_def();
            }
            "if" => {
                todo!()
            }
            "return" => {
                let set_idx = self.len();
                self.push_token(token::Token::Dummy);
                if !matches!(self.parse_return(), Correct(())) {
                    self.truncate(set_idx);
                    return;
                };
                self.set_at(set_idx, token::Token::Return(set_idx + 1));
            }
            _ => {
                self.parse_fn_call(span, true);
            }
        }
    }

    /// `let|const` `<name>` `(?= <expr>)`;
    fn parse_decl(&mut self, kind: token::DeclKind) -> bool {
        // get either var-name or type-name
        let Correct(first) = self.ident() else {
            return false;
        };
        let dummy_pos = self.len();
        self.push_token(token::Token::Dummy);

        let name;
        let value;
        let type_name;

        match self.eq_or_ident() {
            Filtered::Correct(Either::A(())) => {
                self.parse_expr();
                value = is_expr(self.get_token(dummy_pos + 1));
                name = self.range(first);
                type_name = None;
            }
            Filtered::Correct(Either::B(second)) => {
                let Filtered::Correct(()) = self.until_eq() else {
                    return false;
                };
                self.parse_expr();
                value = is_expr(self.get_token(dummy_pos + 1));
                name = self.range(second);
                type_name = Some(self.symbol(first));
            }
            Filtered::InputEnd | Filtered::OtherToken(_) => return false,
        };
        let decl = token::Decl::new(kind, type_name, name.into(), value);
        self.set_at(dummy_pos, decl);
        true
    }

    /// (..) | ..)
    fn parse_fn_call(&mut self, span: BSpan, check_paren: bool) {
        if check_paren && !matches!(self.open_paren(), Correct(())) {
            return;
        }

        let from = self.len();
        self.push_token(token::Token::Dummy);

        loop {
            match self.parse_call_params() {
                Correct(true) => break,
                Correct(false) => (),
                InputEnd | OtherToken(_) => {
                    self.truncate(from);
                    self.err_eof();
                    return;
                }
            };
        }

        let set_idx = from;
        let to = self.len();
        let from = from + 1;
        self.set_at(
            set_idx,
            token::Expr::FnCall(self.symbol(span), TSpan { from, to }),
        );
    }

    // TODO: have the below use Filtered
    //
    /// ..)
    ///
    /// `true` = `CloseParen`
    fn parse_call_params(&mut self) -> Filtered<bool> {
        // TODO: consider not returning a, token
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            if let Comma = token.kind {
                continue;
            }
            break match token.kind {
                CloseParen => true.into(),
                Ident | RawIdent => self.parse_call_param_ident(self.span(token)),
                Literal { kind, suffix_start } => {
                    self.push_token(token::Value::new(
                        self.symbol(self.span(token)),
                        kind,
                        suffix_start,
                    ));
                    false.into()
                }
                Eof => InputEnd,
                _ => {
                    self.err_expected(token, [Comma, CloseParen, Ident, RawIdent, LITERAL]);
                    OtherToken(token)
                }
            };
        }
    }

    fn parse_call_param_ident(&mut self, ident: BSpan) -> Filtered<bool> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            break match token.kind {
                OpenParen => {
                    self.parse_fn_call(ident, false);
                    false.into()
                }
                CloseParen => {
                    self.push_token(token::Expr::Var(self.symbol(ident)));
                    true.into()
                }
                Comma => {
                    self.push_token(token::Expr::Var(self.symbol(ident)));
                    false.into()
                }
                Eof => InputEnd,
                _ => {
                    self.err_expected(token, [OpenParen, CloseParen, Comma, Eof]);
                    OtherToken(token)
                }
            };
        }
    }

    /// `fn` (?`<type>`) `<name>` ((?`<param>`?,)) { (?`<token>`?,) }
    fn parse_fn_def(&mut self) {
        let Correct(first) = self.ident() else {
            return;
        };

        let (name, type_name) = match self.open_paren_or_ident() {
            Correct(Either::A(())) => (first, None),
            Correct(Either::B(span)) => match self.open_paren() {
                Correct(()) => (span, Some(first)),
                InputEnd | OtherToken(_) => return,
            },
            InputEnd | OtherToken(_) => return,
        };

        let dummy_pos = self.len();
        self.push_token(token::Token::Dummy);

        loop {
            match self.parse_def_params() {
                Correct(true) => break,
                Correct(false) => (),
                OtherToken(_) | InputEnd => {
                    self.truncate(dummy_pos);
                    return;
                }
            };
        }

        let param_start = dummy_pos + 1;
        let param_end = self.len();
        let params = TSpan {
            from: param_start,
            to: param_end,
        };

        let Correct(()) = self.open_brace() else {
            self.truncate(dummy_pos);
            return;
        };

        loop {
            let token = self.cursor.advance_token();
            match self.next_or_close_brace(token) {
                Either3::A(()) => {
                    self.truncate(dummy_pos);
                    self.err_eof();
                    return;
                }
                Either3::B(()) => (),
                Either3::C(()) => break,
            };
        }

        let tokens = TSpan {
            from: param_end,
            to: self.len(),
        };

        self.set_at(
            dummy_pos,
            token::FnDef {
                name: self.symbol(name),
                type_name: type_name.map(|span| self.symbol(span)),
                params,
                tokens,
            },
        );
    }

    /// return ..
    fn parse_return(&mut self) -> Filtered<()> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.span(token);
            break match token.kind {
                // TODO: consider moving this to it's own function
                Ident | RawIdent => loop {
                    let Some(after_ident) = self.lex_non_wc() else {
                        continue;
                    };

                    break match after_ident.kind {
                        OpenParen => {
                            self.parse_fn_call(span, false);
                            ().into()
                        }
                        Eof => {
                            self.err_eof();
                            InputEnd
                        }
                        _ => {
                            self.err_expected(token, [CloseParen]);
                            OtherToken(token)
                        }
                    };
                },
                Literal { kind, suffix_start } => {
                    self.push_token(token::Value::new(self.symbol(span), kind, suffix_start));
                    ().into()
                }
                Eof => {
                    self.err_eof();
                    InputEnd
                }
                _ => {
                    self.err_expected(token, [Ident, RawIdent, LITERAL]);
                    OtherToken(token)
                }
            };
        }
    }

    /// ..)
    ///
    /// `true` = `CloseParen`, `false` = `Param`
    fn parse_def_params(&mut self) -> Filtered<bool> {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            if let Comma = token.kind {
                continue;
            }
            break match token.kind {
                CloseParen => true.into(),
                Ident | RawIdent => self.parse_def_decl(self.span(token)),
                Eof => {
                    self.err_eof();
                    InputEnd
                }
                _ => {
                    self.err_expected(token, [Ident, RawIdent, CloseParen]);
                    OtherToken(token)
                }
            };
        }
    }

    /// similar to [`Self::parse_decl`], but detecting a closing paren
    ///
    /// `true` = `CloseParen`, `false` = `Param`
    fn parse_def_decl(&mut self, first: BSpan) -> Filtered<bool> {
        use lex::token::TokenKind::*;
        let filtered = self.ident();
        let Correct(second) = filtered else {
            return filtered.map(|_| false);
        };
        let dummy_pos = self.len();
        self.push_token(token::Token::Dummy);
        let close = loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            match token.kind {
                // parse param with default val
                Eq => {
                    self.parse_expr();
                    break false;
                }
                // simple param found, cont parse
                Comma => break false,
                // all params found, stop parse
                CloseParen => break true,
                // eof with no close, err
                Eof => {
                    self.err_eof();
                    return InputEnd;
                }
                _ => {
                    self.err_expected(token, [Eq, Comma, CloseParen]);
                    return OtherToken(token);
                }
            }
        };
        let value = matches!(self.get_token(dummy_pos + 1), Some(token::Token::Expr(_)));
        let fn_def_param = token::FnDefParam {
            type_name: self.range(first).into(),
            name: self.range(second).into(),
            value,
        };
        self.set_at(dummy_pos, fn_def_param);
        close.into()
    }

    /// parse a top level expr
    fn parse_expr(&mut self) {
        use lex::token::TokenKind::*;
        loop {
            let Some(token) = self.lex_non_wc() else {
                continue;
            };
            let span = self.span(token);
            match token.kind {
                Ident | RawIdent => break self.parse_fn_call(span, true),
                Literal { kind, suffix_start } => {
                    break self.push_token(token::Expr::Value(token::Value::new(
                        self.current_range(token.len).into(),
                        kind,
                        suffix_start,
                    )))
                }
                Eof => break,
                _ => self.err_expected(token, [Ident, RawIdent, LITERAL]),
            }
        }
    }

    /// `A(true)` if eof, `A(true)` if non ident, else `B(Ident)`
    fn ident(&mut self) -> Filtered<BSpan> {
        look_for!(match (self, token, [Ident, RawIdent]) {
            Ident | RawIdent => break self.span(token).into(),
        })
    }

    fn open_paren(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenParen]) {
            OpenParen => break (()).into(),
        })
    }

    fn open_brace(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenBrace]) {
            OpenBrace => break (()).into(),
        })
    }

    /// `A` = `OpenParen`, `C` = `Ident`
    fn open_paren_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, token, [OpenParen, Ident]) {
            OpenParen => break Either::A(()).into(),
            Ident | RawIdent => return Either::B(self.span(token)).into(),
        })
    }

    /// `A` `Eq`, `B(len)` `Ident`
    fn eq_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, token, [Eq, Ident]) {
            Eq => break Either::A(()).into(),
            Ident | RawIdent => break Either::B(self.span(token)).into(),
        })
    }

    fn until_eq(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [Eq]) {
            Eq => break (()).into(),
        })
    }

    fn err_expected(&mut self, span: impl Into<AsBSpan>, expected: impl Into<Vec<lex::TokenKind>>) {
        self.push_err(LexicalError::Expected(self.span(span), expected.into()));
    }

    fn err_eof(&mut self) {
        self.push_err(LexicalError::Eof(self.token_pos()));
    }

    fn lex_non_wc(&mut self) -> Option<lex::Token> {
        let token = self.cursor.advance_token();
        (!self.filter_comment_or_whitespace(token)).then_some(token)
    }

    // TODO: consider also having a flag for parsing when there is a doc comment
    fn filter_comment_or_whitespace(&mut self, token: lex::Token) -> bool {
        use lex::TokenKind::*;
        match token.kind {
            BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::Unclosed(self.span(token)));
                true
            }
            LineComment { .. } | BlockComment { .. } | Whitespace => true,
            _ => false,
        }
    }
}

#[inline]
#[must_use]
const fn is_expr(token: Option<token::Token>) -> bool {
    matches!(token, Some(token::Token::Expr(_)))
}

const LITERAL: lex::TokenKind = lex::TokenKind::Literal {
    kind: lex::LiteralKind::Int {
        base: lex::Base::Binary,
        empty_int: false,
    },
    suffix_start: 0,
};

#[derive(Debug)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

// TODO: consider renaming A B C to One Two Three
#[derive(Debug)]
pub enum Either3<A, B, C> {
    A(A),
    B(B),
    C(C),
}

/// A filtered [`lex::Token`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filtered<T> {
    InputEnd,
    Correct(T),
    /// !(`Whitespace` | `Eof` | `Correct(T)`)
    OtherToken(lex::Token),
}

impl<T> Filtered<T> {
    fn map<T2>(self, map: impl Fn(T) -> T2) -> Filtered<T2> {
        match self {
            InputEnd => InputEnd,
            Correct(t) => map(t).into(),
            OtherToken(t) => OtherToken(t),
        }
    }
}

use Filtered::*;

impl<T> From<T> for Filtered<T> {
    fn from(value: T) -> Self {
        Self::Correct(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsBSpan {
    // Current span used as start
    Len(usize),
    Token(lex::Token),
    // Uses given
    Span(BSpan),
}

impl From<usize> for AsBSpan {
    fn from(value: usize) -> Self {
        Self::Len(value)
    }
}
impl From<lex::Token> for AsBSpan {
    fn from(value: lex::Token) -> Self {
        Self::Token(value)
    }
}
impl From<BSpan> for AsBSpan {
    fn from(value: BSpan) -> Self {
        Self::Span(value)
    }
}

macro_rules! look_for {
    (match ($this:ident, $token:ident, $expected: tt) {
        $($matcher:pat $(if $pred:expr)* => $result:expr),* $(,)?
    }) => {{
        use lex::token::TokenKind::*;
        loop {
            let Some($token) = $this.lex_non_wc() else {
                continue;
            };
            match $token.kind {
                $($matcher $(if $pred)* => $result,)*
                Eof => {
                    $this.err_eof();
                    break InputEnd;
                }
                _ => {
                    $this.err_expected($token, $expected);
                    break OtherToken($token);
                }
            }
        }
    }};
}

use look_for;
