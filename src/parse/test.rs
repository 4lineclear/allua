use expect_test::{expect, Expect};

use super::Reader;

use write::*;
mod write;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

fn do_test(src: &str, expected_tokens: Expect, expected_errors: Expect) {
    let (module, errors) = Reader::new(src).module("test");

    expected_tokens.assert_eq(&write_module(src, &module));
    expected_errors.assert_eq(&write_errs(src, &errors));
}

#[test]
fn unexpected_punct() {
    do_test(
        PUNCT_SRC,
        expect![""],
        expect![[r#"unexpected 0,25 = "}()[],.@#~?:$=!<>-&|+*/^%""#]],
    );
}

#[test]
fn unclosed_block_comment() {
    do_test(
        "/*/*/**/*/",
        expect![""],
        expect![[r#"unclosed 0,10 = "/*/*/**/*/""#]],
    );
}

#[test]
fn let_punct_fail() {
    do_test(
        &("let ".to_owned() + PUNCT_SRC),
        expect![""],
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
        expect![[r#"
            let aa;
        "#]],
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
        expect![[r#"
            let yeah = 3;
        "#]],
        expect![""],
    );
    do_test(
        "const yeah = 3;",
        expect![[r#"
            const yeah = 3;
        "#]],
        expect![""],
    )
}

#[test]
fn decl_with_type() {
    do_test(
        "let string yeah = 3;",
        expect![[r#"
            let string yeah = 3;
        "#]],
        expect![""],
    );
    do_test(
        "const string yeah = 3;",
        expect![[r#"
            const string yeah = 3;
        "#]],
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
        expect![[r#"
            let yeah = 3;
            print(yeah);
        "#]],
        expect![""],
    );
}

#[test]
fn fn_call_str() {
    do_test(
        r#"print("yeah");"#,
        expect![[r#"
            print("yeah");
        "#]],
        expect![""],
    );
}

#[test]
fn fn_fail_single_param() {
    do_test(r#"print("""#, expect![[r""]], expect!["eof 8"]);
    do_test(r#"print("#, expect![[r""]], expect!["eof 6"]);
    do_test(r#"print(print"#, expect![[r""]], expect!["eof 11"]);
    do_test(
        r#"print(print("#,
        expect![[r""]],
        expect![[r#"
        eof 12
        eof 12"#]],
    );
    do_test(
        r#"print"#,
        expect![[r""]],
        expect![r#"unexpected 0,5 = "print""#],
    );
}

#[test]
fn fn_fail_multi_param() {
    do_test(r#"print("yeah","""#, expect![[r""]], expect!["eof 15"]);
    do_test(r#"print(one"#, expect![[r""]], expect!["eof 9"]);
    do_test(
        r#"print(yeah, yeah(), """#,
        expect![[r""]],
        expect![[r#"eof 22"#]],
    );
    do_test(
        r#"print(print(), print("#,
        expect![[r""]],
        expect![[r#"
            eof 21
            eof 21"#]],
    );
    do_test(r#"print(yeah, 1, print"#, expect![[r""]], expect!["eof 20"]);
}

#[test]
fn nested_fn() {
    do_test(
        r#"print(print());"#,
        expect![[r"
            print(print());
        "]],
        expect![[r#""#]],
    );
    do_test(
        r#"print(print(), print());"#,
        expect![[r"
            print(print(), print());
        "]],
        expect![[r#""#]],
    );
    // FIX: the below
    do_test(
        r#"print(print(print(print(print(print(print()))))));"#,
        expect![[r"
            print(print(print(print(print(print(print()))))));
        "]],
        expect![[r#""#]],
    );
}
