// TODO: add detail to printing
use crate::{
    error::ErrorMulti,
    parse::token::{Module, *},
    util::Symbol,
};

#[derive(Debug)]
struct Writer<'a> {
    #[allow(dead_code)]
    src: &'a str,
    items: &'a [Token],
    pos: usize,
    out: &'a mut Vec<String>,
    scheduled: Vec<(usize, &'static str)>,
}

impl<'a> Writer<'a> {
    fn push(&mut self, s: impl Into<AS<'a>>) {
        self.out.push(match s.into() {
            AS::Str(s) => s.to_owned(),
            AS::String(s) => s,
            AS::Symbol(s) => s.as_str().to_owned(),
        })
    }

    fn write_close(&mut self) {
        loop {
            let Some(&(i, s)) = self.scheduled.last() else {
                break;
            };

            if i != self.pos + 1 {
                break;
            }
            self.scheduled.pop();
            self.push(s);
        }
    }

    fn write_token(&mut self, token: Token) {
        match token {
            Token::FnDef(FnDef {
                name, type_name, ..
            }) => {
                self.push("fn");
                type_name.inspect(|s| self.push(s.as_str()));
                self.push(name.as_str());
            }
            Token::Decl(decl) => {
                self.push(match decl.kind {
                    DeclKind::Let => "let",
                    DeclKind::Const => "const",
                });

                if let Some(type_name) = decl.type_name {
                    self.push(type_name.as_str());
                }
                self.push(decl.name.as_str());

                if decl.value {
                    self.push("=");
                }
            }
            Token::Expr(expr) => self.write_expr(expr),
            Token::Return => self.push("return"),
            // Token::Value(val) => self.write_val(val),
            Token::Import(_) => todo!("imports not added yet"),
            Token::Block(span) => {
                self.push("{");
                self.scheduled.push((span.to, "}"));
            }
            Token::FnDefParam(FnDefParam {
                type_name,
                name,
                value,
            }) => {
                self.push(type_name);
                self.push(name);
                if value {
                    self.push("=");
                }
            }
            Token::Dummy => self.push("dummy"),
            Token::Flow(Flow::If(s, el)) => {
                self.push("if");
                if el.is_some() {
                    self.scheduled.push((s.to, "else"));
                }
            }
        };

        self.write_close();
    }

    fn write_expr(&mut self, expr: Expr) {
        match expr.kind {
            ExprKind::FnCall(call) => self.write_fn_call(call, expr),
            ExprKind::Var(name) => self.write_var(name),
            ExprKind::Value(val) => self.write_val(val),
        };
    }

    fn write_fn_call(&mut self, call: FnCall, expr: Expr) {
        self.push(call.name);
        self.push("(");
        if self.pos == expr.end {
            // if call.closed {
            self.push(")");
            // }
            return;
        }

        while self.pos + 1 < expr.end {
            self.pos += 1;
            let token = self.items[self.pos as usize];
            if let Token::Expr(new_expr) = token {
                // prevent infinite recursion
                if token == Token::Expr(expr) {
                    panic!("same expr at index {} found: '{new_expr:#?}'", self.pos)
                }
                self.write_expr(new_expr)
            } else {
                self.write_token(token)
            };
            self.push(",");
        }
        self.out.pop();
        self.push(")");
    }

    fn write_var(&mut self, name: Symbol) {
        self.push(name);
    }

    fn write_val(&mut self, val: Value) {
        self.push(val.value.as_str());
    }
}

pub fn write_module(src: &str, module: &Module) -> Vec<String> {
    let mut out = Vec::new();
    let mut writer = Writer {
        src,
        items: &module.items,
        pos: 0,
        out: &mut out,
        scheduled: Vec::new(),
    };

    loop {
        let Some(&token) = module.items.get(writer.pos as usize) else {
            break;
        };
        writer.write_token(token);
        writer.pos += 1;
    }

    writer.write_close();

    out
}

pub fn write_errs(src: &str, errs: &ErrorMulti) -> String {
    use crate::error::LexicalError::{self, *};
    use std::fmt::Write;
    let mut out = String::new();

    let write_lex = |err: &LexicalError| {
        if out.get(out.len().saturating_sub(2)..) == Some(" \n") {
            out.remove(out.len() - 2);
        }
        match err {
            Unclosed(s) => {
                writeln!(
                    out,
                    r#"unclosed {},{} = "{}" "#,
                    s.from,
                    s.to,
                    &src[s.from..s.to]
                )
            }
            Expected(s, token) => {
                let mut name = String::new();
                for token in token {
                    name.push_str(token.name());
                    name.push_str(" | ");
                }
                if name.ends_with(" | ") {
                    name.pop();
                    name.pop();
                    name.pop();
                }
                writeln!(
                    out,
                    r#"expected pos {},{} to be "{}" but was "{}" "#,
                    s.from,
                    s.to,
                    name,
                    &src[s.from..s.to]
                )
            }
            Eof(pos) => writeln!(out, r#"eof {pos} "#),
            DupeComma(s) => writeln!(
                out,
                r#"dupe commas {},{} = "{}" "#,
                s.from,
                s.to,
                &src[s.from..s.to]
            ),
        }
    };
    errs.lex.iter().try_for_each(write_lex).unwrap();
    errs.other
        .iter()
        .try_for_each(|err| writeln!(out, r#"other error = "{err}""#))
        .unwrap();

    if out.get(out.len().saturating_sub(2)..) == Some(" \n") {
        out.pop();
        out.pop();
    }
    out
}

enum AS<'a> {
    Str(&'a str),
    String(String),
    Symbol(Symbol),
}

impl<'a> From<&'a str> for AS<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(value)
    }
}
impl<'a> From<String> for AS<'a> {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl<'a> From<Symbol> for AS<'a> {
    fn from(value: Symbol) -> Self {
        Self::Symbol(value)
    }
}
