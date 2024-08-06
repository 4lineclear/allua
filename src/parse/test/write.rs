use crate::{
    error::ErrorMulti,
    parse::token::{Module, *},
    util::Symbol,
};

use std::fmt::{self, Write};
type Result = std::result::Result<bool, fmt::Error>;

struct Writer<'a> {
    #[allow(dead_code)]
    src: &'a str,
    items: &'a [Token],
    pos: usize,
    out: &'a mut String,
}

impl<'a> Writer<'a> {
    fn write_token(&mut self, token: Token) -> Result {
        if self.out.get(self.out.len().saturating_sub(2)..) == Some(" \n") {
            self.out.remove(self.out.len() - 2);
        }

        let cont = match token {
            Token::Fn(_) => todo!("fns not added yet"),
            Token::Decl(decl) => {
                match decl.kind {
                    DeclKind::Let => write!(self.out, "let")?,
                    DeclKind::Const => write!(self.out, "const")?,
                };
                if let Some(type_name) = decl.type_name {
                    write!(self.out, " {}", type_name.as_str())?;
                }
                write!(self.out, " {}", decl.name.as_str())?;

                if let Some(expr) = decl.value {
                    write!(self.out, " = ")?;
                    self.write_expr(expr)?;
                }

                Ok(true)
            }
            Token::Expr(expr) => self.write_expr(expr),
            Token::Value(val) => self.write_val(val),
            Token::Import(_) => todo!("imports not added yet"),
            Token::Dummy => {
                writeln!(self.out, "dummy")?;
                Ok(true)
            }
        }?;

        writeln!(self.out, ";")?;

        Ok(cont)
    }

    fn write_expr(&mut self, expr: Expr) -> Result {
        match expr {
            Expr::FnCall(name, param_span) => {
                write!(self.out, "{}(", name.as_str())?;
                for &token in &self.items[param_span.from as usize..param_span.to as usize] {
                    self.pos += 1;
                    let cont = match token {
                        _ if token == Token::Expr(expr) => Ok(true),
                        Token::Expr(expr) => self.write_expr(expr),
                        _ => self.write_token(token),
                    }?;
                    if !cont {
                        return Ok(false);
                    }
                }
                write!(self.out, ")")?;
                Ok(true)
            }
            Expr::Var(name) => self.write_var(name),
            Expr::Value(val) => self.write_val(val),
        }?;

        Ok(true)
    }
    fn write_var(&mut self, name: Symbol) -> Result {
        self.out.write_str(name.as_str())?;
        Ok(true)
    }

    fn write_val(&mut self, val: Value) -> Result {
        self.out.write_str(val.value.as_str())?;
        Ok(true)
    }
}

pub fn write_module(src: &str, module: &Module) -> String {
    let mut out = String::new();
    let mut writer = Writer {
        src,
        items: &module.items,
        pos: 0,
        out: &mut out,
    };

    loop {
        let Some(&token) = module.items.get(writer.pos) else {
            break;
        };

        if !writer.write_token(token).unwrap() {
            break;
        }
        writer.pos += 1;
    }

    if out.get(out.len().saturating_sub(2)..) == Some(" \n") {
        out.pop();
        out.pop();
    }
    out
}

pub fn write_errs(src: &str, errs: &ErrorMulti) -> String {
    use crate::error::LexicalError::{self, *};
    use std::fmt::Write;
    let mut out = String::new();

    let try_each = |err: &LexicalError| {
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
    errs.lex.iter().try_for_each(try_each).unwrap();

    if out.get(out.len().saturating_sub(2)..) == Some(" \n") {
        out.pop();
        out.pop();
    }
    out
}
