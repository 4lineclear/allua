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
    pos: u32,
    out: &'a mut Vec<String>,
    blocks: Vec<u32>,
}

impl<'a> Writer<'a> {
    fn push(&mut self, s: impl Into<AS<'a>>) {
        self.out.push(match s.into() {
            AS::Str(s) => s.to_owned(),
            AS::String(s) => s,
            AS::Symbol(s) => s.as_str().to_owned(),
        })
    }

    fn write_block_end(&mut self) {
        while self.blocks.last() == Some(&(self.pos + 1)) {
            self.blocks.pop();
            self.push("}");
        }
    }

    fn write_token(&mut self, token: Token) {
        match token {
            Token::Fn(_) => todo!("fns not added yet"),
            Token::Decl(decl) => {
                self.push(match decl.kind {
                    DeclKind::Let => "let",
                    DeclKind::Const => "const",
                });

                if let Some(type_name) = decl.type_name {
                    self.push(type_name.as_str());
                }
                self.push(decl.name.as_str());

                if let Some(expr) = decl.value {
                    self.push("=");
                    self.write_expr(expr);
                }
            }
            Token::Expr(expr) => self.write_expr(expr),
            Token::Value(val) => self.write_val(val),
            Token::Import(_) => todo!("imports not added yet"),
            Token::Block(span) => {
                self.push("{");
                self.blocks.push(span.to);
            }
            Token::Dummy => {
                self.push("dummy");
            }
        };

        self.write_block_end();
    }

    fn write_expr(&mut self, expr: Expr) {
        match expr {
            Expr::FnCall(name, param_span) => {
                self.push(name.as_str());
                self.push("(");
                if !param_span.is_empty() {
                    while self.pos + 1 < param_span.to {
                        self.pos += 1;
                        let token = self.items[self.pos as usize];
                        match token {
                            // prevent infinite recursion
                            _ if token == Token::Expr(expr) => {
                                panic!("same expr at index {} found: '{expr:#?}'", self.pos)
                            }
                            Token::Expr(expr) => self.write_expr(expr),
                            _ => self.write_token(token),
                        };
                        self.push(",");
                    }
                    self.out.pop();
                }
                self.push(")");
            }
            Expr::Var(name) => self.write_var(name),
            Expr::Value(val) => self.write_val(val),
        };
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
        blocks: Vec::new(),
    };

    loop {
        let Some(&token) = module.items.get(writer.pos as usize) else {
            break;
        };
        writer.write_token(token);
        writer.pos += 1;
    }

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
                let range = s.from as usize..s.to as usize;
                writeln!(out, r#"unclosed {},{} = "{}" "#, s.from, s.to, &src[range])
            }
            Unexpected(s) => {
                let range = s.from as usize..s.to as usize;
                writeln!(
                    out,
                    r#"unexpected {},{} = "{}" "#,
                    s.from, s.to, &src[range]
                )
            }
            Eof(pos) => writeln!(out, r#"eof {pos} "#),
            MissingSemi(pos) => writeln!(out, r#"missing semi {pos} "#),
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
