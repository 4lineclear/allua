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

// TODO: create composable or improved expected token system
//
// a more centralised system would be very nice

// TODO: turn get most ident/rawident parsing to work the same

// TODO: FIX: expr system is broken
//
// should be fixed before adding operators.
// other errors should benefit from this as well

use self::token::*;

use crate::{
    error::{ErrorMulti, ErrorOnce, LexicalError},
    lex::*,
    span::{BSpan, TSpan},
    util::*,
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
        let (cursor, mut errors, mut tokens, blocks) = self.into_parts();

        for (pos, span) in blocks {
            let span = BSpan::new(span.from, cursor.pos());
            errors.push(LexicalError::Unclosed(span));
            tokens.truncate(pos);
        }

        (Module::new(name, tokens), errors)
    }

    fn next(&mut self) -> bool {
        let lex = self.cursor.advance();
        match self.next_or_close_brace(lex) {
            X(()) => false,
            Y(()) => true,
            Z(()) => {
                self.set_block(lex);
                true
            }
        }
    }

    /// `X` = `Eof` `Y` = `Token` `Z` = `CloseBrace`
    fn next_or_close_brace(&mut self, lex: Lexeme) -> Either3<(), (), ()> {
        let span = self.span(lex);
        let kind = lex.kind;
        match kind {
            // (?doc)comments or whitespace. skip normal comments
            _ if self.filter_comment_or_whitespace(lex) => (),
            Ident | RawIdent => self.ident(span),
            OpenBrace => {
                self.push_block(self.len());
                self.dummy();
            }
            // code block end
            CloseBrace => return Z(()),
            Eof => return X(()),
            _ => self.after_ident(lex),
        };

        Y(())
    }

    fn after_ident(&mut self, lex: Lexeme) {
        let (pos, expr) = match self.get_expr() {
            Ok(out) => out,
            Err(true) => {
                self.top_level_expected(lex);
                return;
            }
            Err(false) => return,
        };
        // println!("{pos}: {expr:#?}");

        if (expr.end != pos) && expr.end != self.len() {
            self.pop_ident();
            self.top_level_expected(lex);
            return;
        }

        match expr.kind {
            ExprKind::FnCall(call) => {
                todo!()
                // self.after_fn_call(lex, pos, expr.end, call);
            }
            ExprKind::Var(name) => match lex.kind {
                OpenParen => self.parse_fn_call(pos, expr.end, name),
                _ => self.top_level_expected(lex),
            },
            ExprKind::Value(_) => {
                panic!("values not handled yet");
            }
        }
    }

    fn parse_fn_call(&mut self, pos: usize, end: usize, name: Symbol) {
        println!("here");
        let mut comma = true;
        let out = look_for!(match (self, lex, [Ident, RawIdent, LITERAL], span) {
            Comma if comma => self.push_err(LexicalError::DupeComma(span)),
            Comma => comma = true,
            CloseParen => break ().into(),
            Ident | RawIdent => {
                self.ident(span);
                comma = false;
            }
            Literal { kind, suffix_start } if comma => {
                self.push_expr(Value::new(self.symbol(lex), kind, suffix_start));
                comma = false;
            }
            _ => {
                // self.after_ident(lex);
                // println!("other: {lex:#?}");
                // self.err_expected(span, [Ident, RawIdent, LITERAL]);
            }
        });
        println!("there");
        match out {
            Correct(()) => (),
            InputEnd | Other(_) => {
                return;
            }
        }
        println!("hare");

        let kind = ExprKind::FnCall(FnCall { name, comma: false });
        let expr = Expr { end, kind };
        self.set_at(pos, expr);
    }

    // fn after_fn_call(&mut self, lex: Lexeme, pos: usize, mut end: usize, mut call: FnCall) {
    //     let mut expr = Expr {
    //         kind: call.into(),
    //         end,
    //     };
    //
    //     if let CloseParen = lex.kind {
    //         self.top_level_expected(lex);
    //         expr.kind = call.into();
    //         self.set_at(pos, expr);
    //         return;
    //     }
    //
    //     match lex.kind {
    //         Comma if end == pos => {
    //             self.err_expected(self.span(lex), [Ident, RawIdent, LITERAL, CloseParen]);
    //             return;
    //         }
    //         Comma if call.comma => {
    //             self.push_err(LexicalError::DupeComma(self.span(lex)));
    //             return;
    //         }
    //         Comma => {
    //             call.comma = true;
    //             expr.kind = call.into();
    //             self.set_at(pos, expr);
    //             return;
    //         }
    //         _ => (),
    //     }
    //
    //     match lex.kind {
    //         Literal { kind, suffix_start } => {
    //             self.push_expr(ExprKind::Value(Value::new(
    //                 self.symbol(lex),
    //                 kind,
    //                 suffix_start,
    //             )));
    //             end = self.len();
    //             expr.end = end;
    //             self.set_at(pos, expr);
    //         }
    //         _ => self.top_level_expected(lex),
    //     }
    // }

    // OpenParen => {
    //     let Some(pos) = self.last_ident() else {
    //         self.top_level_expected(span);
    //         return Either3::Y(());
    //     };
    //
    //     let ident = match self.get_token(pos) {
    //         Some(Token::Expr(Expr::Var(token))) => token,
    //         other => {
    //             self.push_err(ErrorOnce::Other(format!(
    //                 "invalid ident received at {pos}: {other:#?}"
    //             )));
    //             return Either3::Y(());
    //         }
    //     };
    //
    //     self.fn_call(ident, false);
    //     eprintln!("{:#?}", self.get_token(pos));
    // }

    fn ident(&mut self, span: BSpan) {
        match self.str(span) {
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
            _ => self.push_ident(span),
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
                name = self.str(first);
                type_name = None;
            }
            Correct(B(second)) => {
                if !self.until_eq().is_correct() {
                    return false;
                };
                self.expr();
                value = is_expr(self.get_token(set_idx + 1));
                name = self.str(second);
                type_name = Some(self.symbol(first));
            }
            InputEnd | Other(_) => return false,
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

    // /// (..) | ..)
    // fn fn_call(&mut self, s: impl Into<AsStr<'a>>, check_paren: bool) {
    //     if check_paren && !self.open_paren().is_correct() {
    //         return;
    //     }
    //
    //     let from = self.dummy();
    //
    //     let mut comma = true;
    //     loop {
    //         match self.call_params(comma) {
    //             Correct(None) => break,
    //             Correct(Some(found)) => comma = found,
    //             InputEnd | OtherToken(_) => {
    //                 self.truncate(from);
    //                 return;
    //             }
    //         };
    //     }
    //
    //     let set_idx = from;
    //     let to = self.len();
    //     let from = from + 1;
    //     self.set_at(set_idx, Expr::FnCall(self.symbol(s), TSpan { from, to }));
    // }
    // /// ..)
    // ///
    // /// `true` = `CloseParen`
    // fn call_params(&mut self, mut comma_found: bool) -> Filtered<Option<bool>> {
    //     let err = |comma_found: bool| match comma_found {
    //         true => vec![CloseParen, Ident, RawIdent, LITERAL],
    //         false => vec![Comma, CloseParen, Ident, RawIdent, LITERAL],
    //     };
    //     look_for!(match (self, token, err(comma_found), span) {
    //         CloseParen => break None.into(),
    //         Comma if comma_found => self.push_err(LexicalError::DupeComma(span)),
    //         Comma => comma_found = true,
    //         Ident | RawIdent => break self.call_param_ident(self.span(token)),
    //         Literal { kind, suffix_start } => {
    //             self.push_token(Value::new(self.symbol(token), kind, suffix_start));
    //             break Some(false).into();
    //         }
    //     })
    // }
    //
    // fn call_param_ident(&mut self, ident: BSpan) -> Filtered<Option<bool>> {
    //     look_for!(match (self, token, [OpenParen, CloseParen, Comma]) {
    //         CloseParen => {
    //             self.push_token(Expr::Var(self.symbol(ident)));
    //             break None.into();
    //         }
    //         OpenParen => {
    //             self.fn_call(ident, false);
    //             break Some(false).into();
    //         }
    //         Comma => {
    //             self.push_token(Expr::Var(self.symbol(ident)));
    //             break Some(true).into();
    //         }
    //     })
    // }

    /// return ..
    fn parse_return(&mut self) -> Filtered<()> {
        look_for!(match (self, lex, [Ident, RawIdent, LITERAL], span) {
            // Ident | RawIdent => {
            //     break look_for!(match (self, after, [CloseParen]) {
            //         OpenParen => {
            //             self.fn_call(span, false);
            //             break ().into();
            //         }
            //     });
            // }
            Literal { kind, suffix_start } => {
                self.push_expr(Value::new(self.symbol(lex), kind, suffix_start));
                break ().into();
            }
        })
    }

    /// if <cond> {<token>}
    ///
    /// false if parse not success
    fn parse_if(&mut self) -> bool {
        let set_idx = self.dummy();

        let open = look_for!(match (self, lex, [Ident, RawIdent]) {
            OpenBrace => break true.into(),
            Ident | RawIdent => {
                break self
                    .ident_until(
                        lex,
                        |lex| match lex.kind {
                            OpenBrace => Some(()),
                            _ => None,
                        },
                        &[OpenBrace, Eof],
                    )
                    .map(|_| true);
            }
        });

        let Correct(open) = open else {
            self.truncate(set_idx);
            return false;
        };

        if !is_expr(self.get_token(set_idx + 1)) {
            self.truncate(set_idx);
            return false;
        }

        // expect open brace if not found
        if !open && !self.open_brace().is_correct() {
            self.truncate(set_idx);
            return false;
        };

        let token_start = self.len();
        loop {
            let lex = self.cursor.advance();
            match self.next_or_close_brace(lex) {
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

    /// false if parse not success
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
            self.top_level_expected(BSpan::new(self.lex_pos(), self.lex_pos() + 4));
            return false;
        }

        let Correct(after_else) = self.open_brace_or_ident() else {
            return false;
        };

        // catch else - if
        match after_else {
            B(ident) if self.str(ident) == "if" => {
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
            let lex = self.cursor.advance();
            match self.next_or_close_brace(lex) {
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
                InputEnd | Other(_) => return,
            },
            InputEnd | Other(_) => return,
        };

        let set_idx = self.dummy();
        loop {
            match self.def_params() {
                Correct(true) => break,
                Correct(false) => (),
                Other(_) | InputEnd => {
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
            let lex = self.cursor.advance();
            match self.next_or_close_brace(lex) {
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
    /// `true` = `CloseParen`
    fn def_params(&mut self) -> Filtered<bool> {
        look_for!(match (self, lex, [Ident, RawIdent, CloseParen], first) {
            CloseParen => break true.into(),
            Ident | RawIdent => break Self::def_params_ident(self, first),
        })
    }

    fn def_params_ident(&mut self, first: BSpan) -> Filtered<bool> {
        let filtered = self.until_ident();
        let Correct(second) = filtered else {
            return filtered.map(|_| false);
        };
        let set_idx = self.dummy();
        let close = look_for!(match (self, lex, [Eq, Comma, CloseParen]) {
            CloseParen => return true.into(),
            Comma => break false.into(),
            Eq => {
                let out = self.expr();
                if !out.is_correct() {
                    return out.map(|_| false);
                };
                break false.into();
            }
        });
        if !close.is_correct() {
            return close;
        }
        let fn_def_param = FnDefParam {
            type_name: self.str(first).into(),
            name: self.str(second).into(),
            value: is_expr(self.get_token(set_idx + 1)),
        };
        self.set_at(set_idx, fn_def_param);
        close
    }

    fn ident_until<F, T>(
        &mut self,
        ident: impl Into<AsStr<'a>>,
        until: F,
        uncaught: &[LexKind],
    ) -> Filtered<T>
    where
        F: Fn(Lexeme) -> Option<T>,
    {
        self.push_ident(ident);
        loop {
            let Some(lex) = self.lex_non_wc() else {
                continue;
            };
            if let Eof = lex.kind {
                self.err_eof();
                break InputEnd;
            };
            if let Some(t) = until(lex) {
                break t.into();
            }
            self.err_expected(lex, uncaught);
            break Other(lex);
        }
    }

    /// parse an expr, until the given lexeme
    fn expr(&mut self) -> Filtered<()> {
        look_for!(match (self, lex, [Ident, RawIdent, LITERAL], span) {
            Ident | RawIdent => {
                self.push_ident(span);
                break ().into();
            }
            Literal { kind, suffix_start } => {
                self.push_expr(Value::new(self.str(lex.len).into(), kind, suffix_start));
                break ().into();
            }
        })
    }

    /// `A` = `OpenParen`, `C` = `Ident`
    fn open_brace_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, lex, [OpenBrace, Ident, RawIdent]) {
            OpenBrace => break A(()).into(),
            Ident | RawIdent => break B(self.span(lex)).into(),
        })
    }

    /// `A` = `OpenParen`, `C` = `Ident`
    fn open_paren_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, lex, [OpenParen, Ident, RawIdent]) {
            OpenParen => break A(()).into(),
            Ident | RawIdent => break B(self.span(lex)).into(),
        })
    }

    /// `A` `Eq`, `B(len)` `Ident`
    fn eq_or_ident(&mut self) -> Filtered<Either<(), BSpan>> {
        look_for!(match (self, lex, [Eq, Ident]) {
            Eq => break A(()).into(),
            Ident | RawIdent => break B(self.span(lex)).into(),
        })
    }

    fn until_eq(&mut self) -> Filtered<()> {
        look_for!(match (self, lex, [Eq]) {
            Eq => break ().into(),
        })
    }

    /// `A(true)` if eof, `A(true)` if non ident, else `B(Ident)`
    fn until_ident(&mut self) -> Filtered<BSpan> {
        look_for!(match (self, lex, [Ident, RawIdent]) {
            Ident | RawIdent => break self.span(lex).into(),
        })
    }

    fn open_paren(&mut self) -> Filtered<()> {
        look_for!(match (self, lex, [OpenParen]) {
            OpenParen => break ().into(),
        })
    }

    fn open_brace(&mut self) -> Filtered<()> {
        look_for!(match (self, lex, [OpenBrace]) {
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
        self.push_err(LexicalError::Eof(self.lex_pos()));
    }

    fn lex_non_wc(&mut self) -> Option<Lexeme> {
        let lex = self.cursor.advance();
        (!self.filter_comment_or_whitespace(lex)).then_some(lex)
    }

    // TODO: consider also having a flag for parsing when there is a doc comment
    fn filter_comment_or_whitespace(&mut self, lex: Lexeme) -> bool {
        match lex.kind {
            BlockComment { terminated, .. } if !terminated => {
                self.push_err(LexicalError::Unclosed(self.span(lex)));
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

macro_rules! look_for {
    (match ($this:ident, $lex:ident, $expected: expr $(, $span:ident)?) {
        $($matcher:pat $(if $pred:expr)? => $result:expr $(,)?)*
    }) => {{
        use LexKind::*;
        loop {
            let Some($lex) = $this.lex_non_wc() else {
                continue;
            };
            #[allow(unused)]
            $(let $span = $this.span($lex);)?
            match $lex.kind {
                $($matcher $(if $pred)? => $result,)*
                #[allow(unreachable_patterns)]
                Eof => {
                    $this.err_eof();
                    break InputEnd;
                }
                #[allow(unreachable_patterns)]
                _ => {
                    $this.err_expected($lex, $expected);
                    break Other($lex);
                }
            }
        }
    }};
}

use look_for;
