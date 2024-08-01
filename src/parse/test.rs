use expect_test::expect;

use super::Reader;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

#[test]
fn unexpected_punct() {
    let mut reader = Reader::new(PUNCT_SRC);
    let token = reader.next(crate::parse::Mode::Module);
    assert_eq!(token, None);

    let expected_errors = expect![
        "ErrorMulti { errors: [\
        Lecical(UnexpectedPunct('}', 0)), \
        Lecical(UnexpectedPunct('(', 1)), \
        Lecical(UnexpectedPunct(')', 2)), \
        Lecical(UnexpectedPunct('[', 3)), \
        Lecical(UnexpectedPunct(']', 4)), \
        Lecical(UnexpectedPunct(',', 5)), \
        Lecical(UnexpectedPunct('.', 6)), \
        Lecical(UnexpectedPunct('@', 7)), \
        Lecical(UnexpectedPunct('#', 8)), \
        Lecical(UnexpectedPunct('~', 9)), \
        Lecical(UnexpectedPunct('?', 10)), \
        Lecical(UnexpectedPunct(':', 11)), \
        Lecical(UnexpectedPunct('$', 12)), \
        Lecical(UnexpectedPunct('=', 13)), \
        Lecical(UnexpectedPunct('!', 14)), \
        Lecical(UnexpectedPunct('<', 15)), \
        Lecical(UnexpectedPunct('>', 16)), \
        Lecical(UnexpectedPunct('-', 17)), \
        Lecical(UnexpectedPunct('&', 18)), \
        Lecical(UnexpectedPunct('|', 19)), \
        Lecical(UnexpectedPunct('+', 20)), \
        Lecical(UnexpectedPunct('*', 21)), \
        Lecical(UnexpectedPunct('/', 22)), \
        Lecical(UnexpectedPunct('^', 23)), \
        Lecical(UnexpectedPunct('%', 24))] }"
    ];

    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn unclosed_block_comment() {
    let src = "/*/*/**/*/";
    let mut reader = Reader::new(src);
    let token = reader.next(crate::parse::Mode::Module);
    assert_eq!(token, None);

    let expected_errors = expect![
        "ErrorMulti { errors: [\
        Lecical(UnclosedBlockComment(0))] }"
    ];

    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}
#[test]
fn let_punct_fail() {
    let src = "let".to_owned() + PUNCT_SRC;
    let mut reader = Reader::new(&src);
    let token = reader.next(crate::parse::Mode::Module);
    assert_eq!(token, None);

    let expected_errors = expect![
        "ErrorMulti { errors: [\
        Lecical(UnexpectedPunct('}', 3)), \
        Lecical(UnexpectedPunct('(', 4)), \
        Lecical(UnexpectedPunct(')', 5)), \
        Lecical(UnexpectedPunct('[', 6)), \
        Lecical(UnexpectedPunct(']', 7)), \
        Lecical(UnexpectedPunct(',', 8)), \
        Lecical(UnexpectedPunct('.', 9)), \
        Lecical(UnexpectedPunct('@', 10)), \
        Lecical(UnexpectedPunct('#', 11)), \
        Lecical(UnexpectedPunct('~', 12)), \
        Lecical(UnexpectedPunct('?', 13)), \
        Lecical(UnexpectedPunct(':', 14)), \
        Lecical(UnexpectedPunct('$', 15)), \
        Lecical(UnexpectedPunct('=', 16)), \
        Lecical(UnexpectedPunct('!', 17)), \
        Lecical(UnexpectedPunct('<', 18)), \
        Lecical(UnexpectedPunct('>', 19)), \
        Lecical(UnexpectedPunct('-', 20)), \
        Lecical(UnexpectedPunct('&', 21)), \
        Lecical(UnexpectedPunct('|', 22)), \
        Lecical(UnexpectedPunct('+', 23)), \
        Lecical(UnexpectedPunct('*', 24)), \
        Lecical(UnexpectedPunct('/', 25)), \
        Lecical(UnexpectedPunct('^', 26)), \
        Lecical(UnexpectedPunct('%', 27)), \
        Lecical(NameNotFound(28))] }"
    ];

    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}

#[test]
fn let_success() {
    let src = "let yeah = 3";
    let mut reader = Reader::new(src);
    let token = reader.next(crate::parse::Mode::Module);
    let expected_token = expect![
        "Some(Decl(Decl { kind: Let, name: u!(\"yeah\"), value: Some(Value(Value \
            { value: u!(\"3\"), kind: Int { base: Decimal, empty_int: false }, suffix_start: 1 })) }))"
    ];
    expected_token.assert_eq(&format!("{token:?}",));

    let expected_errors = expect!["ErrorMulti { errors: [] }"];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}
