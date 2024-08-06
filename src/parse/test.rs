use expect_test::{expect, Expect};

use crate::error::ErrorMulti;

use super::token::Module;
use super::Reader;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

fn do_test(src: &str, expected_tokens: Expect, expected_errors: Expect) {
    let (token, errors) = Reader::new(src).module("test");

    expected_tokens.assert_eq(&format!("{token:?}",));
    expected_errors.assert_eq(&write_errs(src, &errors));
}

#[test]
fn unexpected_punct() {
    do_test(
        PUNCT_SRC,
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect![[r#"unexpected 0,25 = "}()[],.@#~?:$=!<>-&|+*/^%""#]],
    );
}

#[test]
fn unclosed_block_comment() {
    do_test(
        "/*/*/**/*/",
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect![[r#"unclosed 0,10 = "/*/*/**/*/""#]],
    );
}

#[test]
fn let_punct_fail() {
    do_test(
        &("let ".to_owned() + PUNCT_SRC),
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect![[r#"
            unexpected 4,29 = "}()[],.@#~?:$=!<>-&|+*/^%"
            eof 29"#]],
    );
}

#[test]
fn multi_err() {
    do_test(
        "\
        let aa = // \n\
        /**/ ^@@ # !/*/*/**/*/",
        expect![[
            "Module { name: u!(\"test\"), items: [Decl(Decl { kind: Let, \
            type_name: None, name: u!(\"aa\"), value: None })] }"
        ]],
        expect![[r##"
            unexpected 18,21 = "^@@"
            unexpected 22,23 = "#"
            unexpected 24,25 = "!"
            unclosed 25,35 = "/*/*/**/*/"
            missing semi 35"##]],
    );
}

#[test]
fn decl() {
    do_test(
        "let yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Let, type_name: None, name: u!(\"yeah\"), value: Some(Value(Value \
        { value: u!(\"3\"), kind: Int { base: Decimal, empty_int: false }, suffix_start: \
        1 })) })] }"]],
        expect![""],
    );
    do_test(
        "const yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Const, type_name: None, name: u!(\"yeah\"), value: Some(Value(Value { \
        value: u!(\"3\"), kind: Int { base: Decimal, empty_int: false }, suffix_start: \
        1 })) })] }"]],
        expect![""],
    )
}

#[test]
fn decl_with_type() {
    do_test(
        "let string yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Let, type_name: Some(u!(\"string\")), name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]],
        expect![""]
    );
    do_test(
        "const string yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Const, type_name: Some(u!(\"string\")), name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]],
        expect![""],
    )
}

#[test]
fn let_chain() {
    let src = "let yeah = 3;".repeat(10);
    let mut reader = Reader::new(&src);
    let token = reader.next(crate::parse::ParseMode::Module);
    for _ in 0..10 {
        let expected_token = expect![[
            "Some(Decl(Decl { kind: Let, type_name: None, name: u!(\"yeah\"), \
            value: Some(Value(Value { value: u!(\"3\"), kind: Int { base: Decimal, \
            empty_int: false }, suffix_start: 1 })) }))"
        ]];
        expected_token.assert_eq(&format!("{token:?}",));
    }

    let expected_errors = expect!["ErrorMulti { lex: [], other: [] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_and_fn() {
    do_test(
        "\
        let yeah = 3;\n\
        print(yeah);\
        ",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Let, type_name: None, name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) }), \
        Expr(FnCall(u!(\"print\"), TSpan { from: 1, to: 2 })), Expr(Var(u!(\"yeah\")))] }"]],
        expect![""],
    );
}

#[test]
fn fn_call_str() {
    // FIX: "yeah" is added twice
    do_test(
        r#"print("yeah");"#,
        expect![["Module { name: u!(\"test\"), items: [Expr(FnCall(u!(\"print\"), \
            TSpan { from: 0, to: 2 })), Expr(Value(Value { value: u!(\"\\\"yeah\\\"\"), kind: Str \
            { terminated: true }, suffix_start: 6 })), Expr(Value(Value { value: u!(\"\\\"yeah\\\"\"), \
            kind: Str { terminated: true }, suffix_start: 6 }))] }"]],
        expect![""],
    );
}

impl Module {
    pub fn to_str(&self) -> String {
        todo!()
        // self.
    }
}

fn write_errs(src: &str, errs: &ErrorMulti) -> String {
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
