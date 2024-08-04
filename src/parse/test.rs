use expect_test::{expect, Expect};

use super::Reader;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

fn do_test(src: &str, expected_tokens: Expect, expected_errors: Expect) {
    let mut reader = Reader::new(src);

    let token = reader.module("test");
    let errors = reader.errors;

    expected_tokens.assert_eq(&format!("{token:?}",));
    expected_errors.assert_eq(&format!("{errors:?}",));
}

#[test]
fn unexpected_punct() {
    do_test(
        PUNCT_SRC,
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect!["ErrorMulti { errors: [Lexical(UnexpectedRange(0, 24))] }"],
    );
}

#[test]
fn unclosed_block_comment() {
    do_test(
        "/*/*/**/*/",
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect!["ErrorMulti { errors: [Lexical(UnclosedBlockComment(0))] }"],
    );
}

#[test]
fn let_punct_fail() {
    do_test(
        &("let ".to_owned() + PUNCT_SRC),
        expect!["Module { name: u!(\"test\"), items: [] }"],
        expect![
            "ErrorMulti { errors: [Lexical(UnexpectedRange(4, 28)), Lexical(NameNotFound(29))] }"
        ],
    );
}

#[test]
fn normal_let() {
    do_test(
        "let yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]],
        expect!["ErrorMulti { errors: [] }"],
    );
}

#[test]
fn decl_with_type() {
    do_test(
        "string yeah = 3;",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Type(u!(\"string\")), name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]],
        expect![["ErrorMulti { errors: [] }"]]
    );
}

#[test]
fn let_chain() {
    let src = "let yeah = 3;".repeat(10);
    let mut reader = Reader::new(&src);
    let token = reader.next(crate::parse::FnParseMode::Module);
    for _ in 0..10 {
        let expected_token = expect![[
            "Some(Decl(Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value \
            { value: u!(\"3\"), kind: Int { base: Decimal, empty_int: false }, \
            suffix_start: 1 })) }))"
        ]];
        expected_token.assert_eq(&format!("{token:?}",));
    }

    let expected_errors = expect!["ErrorMulti { errors: [] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_and_fn() {
    do_test(
        "\
        let yeah = 3;\n\
        print(yeah);
        ",
        expect![["Module { name: u!(\"test\"), items: [Decl(\
        Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]],
        expect!["ErrorMulti { errors: [] }"],
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
            name: u!(\"aa\"), value: None })] }"
        ]],
        expect![
            "ErrorMulti { errors: [\
        Lexical(UnexpectedRange(18, 20)), Lexical(UnexpectedPunct('#', 22)), \
        Lexical(UnexpectedPunct('!', 24)), Lexical(UnclosedBlockComment(25)), \
        Lexical(MissingSemi(35, 0))] }"
        ],
    );
}
