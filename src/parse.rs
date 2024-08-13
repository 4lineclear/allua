// TODO: add tuples
//
// would be in the same style as rust tuples

// TODO: create a compiler error type.
//
// currently have a generic Other(String) error that should be replaced

// TODO: add visibility item to Fn
//
// should probably add it to other constructs as well

// TODO: add operators
//
// should support every operator that rust does

// TODO: add incomplete-expr system
//
// A single ident could be interpreted as an incomplete-expr

// TODO: FIX: expr system is broken
//
// should be fixed before adding operators.
// other errors should benefit from this as well

use self::token::*;

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::*,
    span::{BSpan, TSpan},
};

pub use secure::Reader;

/// a secure module for keeping certain fields safe.
mod secure;
#[cfg(test)]
pub mod test;
pub mod token;

pub const EXPECTED_CLOSE: [LexKind; 5] = [Ident, RawIdent, OpenBrace, CloseBrace, Eof];
pub const EXPECTED: [LexKind; 4] = [Ident, RawIdent, OpenBrace, Eof];

impl<'a> Reader<'a> {
    /// Parse a module
    #[must_use]
    pub fn module(mut self, name: &str) -> (Module, ErrorMulti) {
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

        (Module::new(name, tokens), errors)
    }

    fn next(&mut self) -> bool {
        let token = self.cursor.advance_token();
        match self.next_or_close_brace(token) {
            X(()) => false,
            Y(()) => true,
            Z(()) => {
                self.set_block(token);
                true
            }
        }
    }

    /// `X` = `Eof` `Y` = `Token` `Z` = `CloseBrace`
    fn next_or_close_brace(&mut self, token: Lexeme) -> Either3<(), (), ()> {
        let span = self.span(token);
        let kind = token.kind;
        match kind {
            // (?doc)comments or whitespace. skip normal comments
            _ if self.filter_comment_or_whitespace(token) => (),
            Ident | RawIdent => self.ident(span),
            OpenBrace => {
                self.push_block(self.len());
                self.dummy();
            }
            // code block end
            CloseBrace => return Z(()),
            Eof => return X(()),
            _ => self.top_level_expected(token),
        };

        Y(())
    }

    fn ident(&mut self, span: BSpan) {
        match self.range(span) {
            "let" => {
                self.decl(DeclKind::Let);
            }
            "const" => {
                self.decl(DeclKind::Const);
            }
            "fn" => {
                self.fn_def();
            }
            "if" => {
                self.parse_if();
            }
            "else" => {
                if !self.last_flow(Self::parse_else) {
                    self.top_level_expected(span);
                }
            }
            "return" => {
                let set_idx = self.dummy();
                if self.parse_return().is_correct() {
                    self.set_at(set_idx, Token::Return);
                } else {
                    self.truncate(set_idx);
                };
            }
            _ => {
                self.fn_call(span, true);
            }
        }
    }

    /// `let|const` `<name>` `(?= <expr>)`;
    fn decl(&mut self, kind: DeclKind) -> bool {
        // get either var-name or type-name
        let Correct(first) = self.until_ident() else {
            return false;
        };
        let set_idx = self.dummy();

        let name;
        let value;
        let type_name;

        match self.eq_or_ident() {
            Correct(A(())) => {
                self.expr();
                value = is_expr(self.get_token(set_idx + 1));
                name = self.range(first);
                type_name = None;
            }
            Correct(B(second)) => {
                if !self.until_eq().is_correct() {
                    return false;
                };
                self.expr();
                value = is_expr(self.get_token(set_idx + 1));
                name = self.range(second);
                type_name = Some(self.symbol(first));
            }
            InputEnd | OtherToken(_) => return false,
        };
        let decl = Decl {
            kind,
            type_name,
            name: name.into(),
            value,
        };
        self.set_at(set_idx, decl);
        true
    }

    /// (..) | ..)
    fn fn_call(&mut self, span: BSpan, check_paren: bool) {
        if check_paren && !self.open_paren().is_correct() {
            return;
        }

        let from = self.dummy();

        let mut comma = true;
        loop {
            match self.call_params(comma) {
                Correct(None) => break,
                Correct(Some(found)) => comma = found,
                InputEnd | OtherToken(_) => {
                    self.truncate(from);
                    return;
                }
            };
        }

        let set_idx = from;
        let to = self.len();
        let from = from + 1;
        self.set_at(set_idx, Expr::FnCall(self.symbol(span), TSpan { from, to }));
    }

    /// ..)
    ///
    /// `true` = `CloseParen`
    fn call_params(&mut self, mut comma_found: bool) -> Filtered<Option<bool>> {
        let err = |comma_found: bool| match comma_found {
            true => vec![CloseParen, Ident, RawIdent, LITERAL],
            false => vec![Comma, CloseParen, Ident, RawIdent, LITERAL],
        };
        look_for!(match (self, token, err(comma_found), span) {
            CloseParen => break None.into(),
            Comma if comma_found => self.push_err(LexicalError::DupeComma(span)),
            Comma => comma_found = true,
            Ident | RawIdent => break self.call_param_ident(self.span(token)),
            Literal { kind, suffix_start } => {
                self.push_token(Value::new(self.symbol(token), kind, suffix_start));
                break Some(false).into();
            }
        })
    }

    fn call_param_ident(&mut self, ident: BSpan) -> Filtered<Option<bool>> {
        look_for!(match (self, token, [OpenParen, CloseParen, Comma]) {
            CloseParen => {
                self.push_token(Expr::Var(self.symbol(ident)));
                break None.into();
            }
            OpenParen => {
                self.fn_call(ident, false);
                break Some(false).into();
            }
            Comma => {
                self.push_token(Expr::Var(self.symbol(ident)));
                break Some(true).into();
            }
        })
    }

    /// return ..
    fn parse_return(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [Ident, RawIdent, LITERAL], span) {
            Ident | RawIdent => {
                break look_for!(match (self, after, [CloseParen]) {
                    OpenParen => {
                        self.fn_call(span, false);
                        break ().into();
                    }
                });
            }
            Literal { kind, suffix_start } => {
                self.push_token(Value::new(self.symbol(token), kind, suffix_start));
                break ().into();
            }
        })
    }

    /// if <cond> {<token>}
    fn parse_if(&mut self) -> bool {
        let set_idx = self.dummy();

        let close = look_for!(match (self, token, [Ident, RawIdent]) {
            OpenBrace => break true.into(),
            Ident | RawIdent => {
                let ident = self.span(token);
                break look_for!(match (self, token, [OpenParen, CloseParen, Eof]) {
                    OpenParen => {
                        self.fn_call(ident, false);
                        break false.into();
                    }
                    OpenBrace => {
                        self.push_token(Expr::Var(self.symbol(ident)));
                        break true.into();
                    }
                });
            }
        });

        let Correct(close) = close else {
            self.truncate(set_idx);
            return false;
        };

        if !is_expr(self.get_token(set_idx + 1)) {
            self.truncate(set_idx);
            return false;
        }

        // expect open brace if not found
        if !close && !self.open_brace().is_correct() {
            self.truncate(set_idx);
            return false;
        };

        let token_start = self.len();
        loop {
            let token = self.cursor.advance_token();
            match self.next_or_close_brace(token) {
                X(()) => {
                    self.truncate(set_idx);
                    self.err_eof();
                    return false;
                }
                Y(()) => (),
                Z(()) => break,
            };
        }

        let token = Flow::If(
            TSpan {
                from: token_start,
                to: self.len(),
            },
            None,
        );
        self.push_flow(set_idx);
        self.set_at(set_idx, token);
        true.into()
    }

    /// returns false if parse not success
    ///
    /// should be run with [`Self::last_flow`]
    fn parse_else(&mut self, orig_pos: usize) -> bool {
        let orig_span = match self.get_token(orig_pos) {
            Some(Token::Flow(Flow::If(span, _))) => span,
            other => {
                self.push_err(ErrorOnce::Other(format!(
                    "invalid flow received at {orig_pos}: {other:#?}"
                )));
                return false;
            }
        };

        if self.len() != orig_span.to {
            self.top_level_expected(BSpan::new(self.len(), self.len() + 4));
            return false;
        }

        let Correct(after_else) = self.open_brace_or_ident() else {
            return false;
        };

        // catch else - if
        match after_else {
            B(ident) if self.range(ident) == "if" => {
                let start = self.len();
                let parsed = self.parse_if();
                if parsed {
                    let token = Flow::If(
                        orig_span,
                        Some(TSpan {
                            from: start,
                            to: self.len(),
                        }),
                    );
                    self.set_at(orig_pos, token);
                }
                return parsed;
            }
            B(ident) => {
                self.top_level_expected(ident);
                return false;
            }
            A(()) => (),
        }

        let token_start = self.len();
        loop {
            let token = self.cursor.advance_token();
            match self.next_or_close_brace(token) {
                X(()) => {
                    self.truncate(token_start);
                    self.err_eof();
                    return false;
                }
                Y(()) => (),
                Z(()) => break,
            };
        }

        let token = Flow::If(
            orig_span,
            Some(TSpan {
                from: token_start,
                to: self.len(),
            }),
        );
        self.set_at(orig_pos, token);
        true
    }

    /// `fn` (?`<type>`) `<name>` ((?`<param>`?,)) { (?`<token>`?,) }
    fn fn_def(&mut self) {
        let Correct(first) = self.until_ident() else {
            return;
        };

        let (name, type_name) = match self.open_paren_or_ident() {
            Correct(A(())) => (first, None),
            Correct(B(span)) => match self.open_paren() {
                Correct(()) => (span, Some(first)),
                InputEnd | OtherToken(_) => return,
            },
            InputEnd | OtherToken(_) => return,
        };

        let set_idx = self.dummy();
        loop {
            match self.def_params() {
                Correct(true) => break,
                Correct(false) => (),
                OtherToken(_) | InputEnd => {
                    self.truncate(set_idx);
                    return;
                }
            };
        }
        let param_end = self.len();

        if !self.open_brace().is_correct() {
            self.truncate(set_idx);
            return;
        };

        loop {
            let token = self.cursor.advance_token();
            match self.next_or_close_brace(token) {
                X(()) => {
                    self.truncate(set_idx);
                    self.err_eof();
                    return;
                }
                Y(()) => (),
                Z(()) => break,
            };
        }

        let token = FnDef {
            name: self.symbol(name),
            type_name: type_name.map(|span| self.symbol(span)),
            params: TSpan {
                from: set_idx + 1,
                to: param_end,
            },
            tokens: TSpan {
                from: param_end,
                to: self.len(),
            },
        };
        self.set_at(set_idx, token);
    }

    /// ..)
    ///
    /// `true` = `CloseParen`, `false` = `Param`
    fn def_params(&mut self) -> Filtered<bool> {
        look_for!(match (self, token, [Ident, RawIdent, CloseParen], first) {
            CloseParen => break true.into(),
            Ident | RawIdent => {
                let filtered = self.until_ident();
                let Correct(second) = filtered else {
                    return filtered.map(|_| false);
                };
                let set_idx = self.dummy();
                let close = look_for!(match (self, token, [Eq, Comma, CloseParen]) {
                    CloseParen => break true.into(),
                    Comma => break false.into(),
                    Eq => {
                        self.expr();
                        break false.into();
                    }
                });
                if !close.is_correct() {
                    return close;
                }
                let fn_def_param = FnDefParam {
                    type_name: self.range(first).into(),
                    name: self.range(second).into(),
                    value: is_expr(self.get_token(set_idx + 1)),
                };
                self.set_at(set_idx, fn_def_param);
                break close;
            }
        })
    }

    /// parse a top level expr
    fn expr(&mut self) {
        look_for!(match (self, token, [], span) {
            Ident | RawIdent => {
                self.fn_call(span, true);
                break ().into();
            }
            Literal { kind, suffix_start } => {
                self.push_token(Expr::Value(Value::new(
                    self.current_range(token.len).into(),
                    kind,
                    suffix_start,
                )));
                break ().into();
            }
            Eof => break ().into(),
            _ => self.err_expected(token, [Ident, RawIdent, LITERAL]),
        });
    }

    /// `A` = `OpenParen`, `C` = `Ident`
    fn open_brace_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, token, [OpenBrace, Ident, RawIdent]) {
            OpenBrace => break A(()).into(),
            Ident | RawIdent => return B(self.span(token)).into(),
        })
    }

    /// `A` = `OpenParen`, `C` = `Ident`
    fn open_paren_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, token, [OpenParen, Ident, RawIdent]) {
            OpenParen => break A(()).into(),
            Ident | RawIdent => return B(self.span(token)).into(),
        })
    }

    /// `A` `Eq`, `B(len)` `Ident`
    fn eq_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, token, [Eq, Ident]) {
            Eq => break A(()).into(),
            Ident | RawIdent => break B(self.span(token)).into(),
        })
    }

    fn until_eq(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [Eq]) {
            Eq => break ().into(),
        })
    }

    /// `A(true)` if eof, `A(true)` if non ident, else `B(Ident)`
    fn until_ident(&mut self) -> Filtered<BSpan> {
        look_for!(match (self, token, [Ident, RawIdent]) {
            Ident | RawIdent => break self.span(token).into(),
        })
    }

    fn open_paren(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenParen]) {
            OpenParen => break ().into(),
        })
    }

    fn open_brace(&mut self) -> Filtered<()> {
        look_for!(match (self, token, [OpenBrace]) {
            OpenBrace => break ().into(),
        })
    }

    fn err_expected(&mut self, span: impl Into<AsBSpan>, expected: impl Into<Vec<LexKind>>) {
        self.push_err(LexicalError::Expected(self.span(span), expected.into()));
    }

    fn top_level_expected(&mut self, span: impl Into<AsBSpan>) {
        match self.blocks_left() {
            true => self.err_expected(span, EXPECTED_CLOSE),
            false => self.err_expected(span, EXPECTED),
        }
    }

    fn err_eof(&mut self) {
        self.push_err(LexicalError::Eof(self.token_pos()));
    }

    fn lex_non_wc(&mut self) -> Option<Lexeme> {
        let token = self.cursor.advance_token();
        (!self.filter_comment_or_whitespace(token)).then_some(token)
    }

    // TODO: consider also having a flag for parsing when there is a doc comment
    fn filter_comment_or_whitespace(&mut self, token: Lexeme) -> bool {
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
const fn is_expr(token: Option<Token>) -> bool {
    matches!(token, Some(Token::Expr(_)))
}

const LITERAL: LexKind = LexKind::Literal {
    kind: LiteralKind::Int {
        base: Base::Binary,
        empty_int: false,
    },
    suffix_start: 0,
};

use Either::*;
use Either3::*;
use Filtered::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either<A, B> {
    A(A),
    B(B),
}

impl<A, B> Either<A, B> {
    pub fn map_a<C>(self, map: impl Fn(A) -> C) -> Either<C, B> {
        match self {
            A(a) => Either::A(map(a)),
            B(b) => Either::B(b),
        }
    }

    pub fn map_b<C>(self, map: impl Fn(B) -> C) -> Either<A, C> {
        match self {
            A(a) => Either::A(a),
            B(b) => Either::B(map(b)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Either3<X, Y, Z> {
    X(X),
    Y(Y),
    Z(Z),
}

/// A filtered [`lex::Lexeme`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Filtered<T> {
    InputEnd,
    Correct(T),
    /// !(`Whitespace` | `Eof` | `Correct(T)`)
    OtherToken(Lexeme),
}

impl<T> Filtered<T> {
    fn map<T2>(self, map: impl Fn(T) -> T2) -> Filtered<T2> {
        match self {
            InputEnd => InputEnd,
            Correct(t) => map(t).into(),
            OtherToken(t) => OtherToken(t),
        }
    }

    const fn is_correct(&self) -> bool {
        matches!(self, Correct(_))
    }
}

impl<T> From<T> for Filtered<T> {
    fn from(value: T) -> Self {
        Correct(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AsBSpan {
    // Current span used as start
    Len(usize),
    Token(Lexeme),
    // Uses given
    Span(BSpan),
}

impl From<usize> for AsBSpan {
    fn from(value: usize) -> Self {
        Self::Len(value)
    }
}
impl From<Lexeme> for AsBSpan {
    fn from(value: Lexeme) -> Self {
        Self::Token(value)
    }
}
impl From<BSpan> for AsBSpan {
    fn from(value: BSpan) -> Self {
        Self::Span(value)
    }
}

macro_rules! look_for {
    (match ($this:ident, $token:ident, $expected: expr $(, $span:ident)?) {
        $($matcher:pat $(if $pred:expr)? => $result:expr $(,)?)*
    }) => {{
        use LexKind::*;
        loop {
            let Some($token) = $this.lex_non_wc() else {
                continue;
            };
            $(let $span = $this.span($token);)?
            match $token.kind {
                $($matcher $(if $pred)? => $result,)*
                #[allow(unreachable_patterns)]
                Eof => {
                    $this.err_eof();
                    break InputEnd;
                }
                #[allow(unreachable_patterns)]
                _ => {
                    $this.err_expected($token, $expected);
                    break OtherToken($token);
                }
            }
        }
    }};
}

use look_for;
