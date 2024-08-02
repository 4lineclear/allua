use expect_test::expect;

use super::Reader;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

#[test]
fn unexpected_punct() {
    let mut reader = Reader::from(PUNCT_SRC);
    let token = reader.next(crate::parse::FnParseMode::Module);
    assert_eq!(token, None);

    let expected_errors = expect!["ErrorMulti { errors: [Lexical(UnexpectedRange(0, 24))] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn unclosed_block_comment() {
    let src = "/*/*/**/*/";
    let mut reader = Reader::from(src);
    let token = reader.next(crate::parse::FnParseMode::Module);
    assert_eq!(token, None);

    let expected_errors = expect!["ErrorMulti { errors: [Lexical(UnclosedBlockComment(0))] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_punct_fail() {
    let src = "let".to_owned() + PUNCT_SRC;
    let mut reader = Reader::from(src);
    let token = reader.next(crate::parse::FnParseMode::Module);
    assert_eq!(token, None);

    let expected_errors = expect![
        "ErrorMulti { errors: [Lexical(UnexpectedRange(3, 27)), Lexical(NameNotFound(28))] }"
    ];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_module() {
    let src = "let yeah = 3";
    let mut reader = Reader::from(src);
    let token = reader.module("test");
    let expected_token = expect![["Module { name: u!(\"test\"), tokens: Span { \
        from: 1, to: 2, kind: PhantomData<allua::parse::token::Token> }, items: [Decl(\
        Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]];
    expected_token.assert_eq(&format!("{token:?}",));

    let expected_errors = expect!["ErrorMulti { errors: [] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_chain() {
    let src = "let yeah = 3;".repeat(10);
    let mut reader = Reader::from(src);
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
    let src = "\
        let yeah = 3;\n\
        print(yeah);
        ";
    let mut reader = Reader::from(src);
    let token = reader.module("test");
    let expected_token = expect![["Module { name: u!(\"test\"), tokens: Span { \
        from: 1, to: 2, kind: PhantomData<allua::parse::token::Token> }, items: [Decl(\
        Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value { value: u!(\"3\"), \
        kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) })] }"]];
    expected_token.assert_eq(&format!("{token:?}",));

    let expected_errors = expect!["ErrorMulti { errors: [] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn multi_err() {
    let src = "\
        let aa = // \n\
        /**/ ^@@ # !/*/*/**/*/";
    let mut reader = Reader::from(src);
    let token = reader.next(crate::parse::FnParseMode::Module);
    let expected_token =
        expect![[r#"Some(Decl(Decl { kind: Let, name: u!("aa"), value: None }))"#]];
    expected_token.assert_eq(&format!("{token:?}",));

    let expected_errors = expect![
        "ErrorMulti { errors: \
        [Lexical(UnexpectedRange(18, 20)), Lexical(UnexpectedPunct('#', 22)), \
        Lexical(UnexpectedPunct('!', 24)), Lexical(UnclosedBlockComment(35))] }"
    ];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}
