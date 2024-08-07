// TODO: create custom tester
// use expect_test::expect;
use super::Reader;
use pretty_assertions::assert_eq;

use write::*;
mod write;

const PUNCT_SRC: &str = "}()[],.@#~?:$=!<>-&|+*/^%";

macro_rules! pos {
    () => {
        (file!(), line!(), column!())
    };
}

fn map_errs(s: &str) -> String {
    let mut errs = String::with_capacity(s.len());
    s.trim()
        .lines()
        .map(str::trim)
        .map(str::trim)
        .for_each(|s| {
            errs.push_str(s);
            errs.push('\n');
        });
    if errs.chars().last() == Some('\n') {
        errs.pop();
    }
    errs
}

macro_rules! do_test {
    ($src:expr, $expected_tokens:expr, $expected_errors:expr $(,)?) => {{
        let (module, errors) = Reader::new($src).module("test");
        // extra info here in case of failure
        println!("testing {:?} {module:#?}", pos!());
        let errs = map_errs($expected_errors);
        assert_eq!(
            $expected_tokens
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            write_module($src, &module)
        );
        assert_eq!(&errs, &write_errs($src, &errors));
    }};
}

#[test]
fn empty() {
    do_test!("", [], "");
}

#[test]
fn unexpected_punct() {
    do_test!(
        PUNCT_SRC,
        [],
        r#"unexpected 0,25 = "}()[],.@#~?:$=!<>-&|+*/^%""#,
    );
}

#[test]
fn unclosed_block_comment() {
    do_test!("/*/*/**/*/", [], r#"unclosed 0,10 = "/*/*/**/*/""#);
}

#[test]
fn let_punct_fail() {
    do_test!(
        &("let ".to_owned() + PUNCT_SRC),
        [],
        r#"
            unexpected 4,29 = "}()[],.@#~?:$=!<>-&|+*/^%"
            eof 29"#,
    );
}

#[test]
fn multi_err() {
    do_test!(
        "\
        let aa = // \n\
        /**/ ^@@ # !/*/*/**/*/",
        ["let", "aa"],
        r##"
            unexpected 18,21 = "^@@"
            unexpected 22,23 = "#"
            unexpected 24,25 = "!"
            unclosed 25,35 = "/*/*/**/*/"
            missing semi 35"##,
    );
}

#[test]
fn decl() {
    do_test!("let yeah = 3;", ["let", "yeah", "=", "3"], "");
    do_test!("const yeah = 3;", ["const", "yeah", "=", "3"], "");
}

#[test]
fn decl_with_type() {
    do_test!(
        "let string yeah = 3;",
        ["let", "string", "yeah", "=", "3"],
        ""
    );
    do_test!(
        "const string yeah = 3;",
        ["const", "string", "yeah", "=", "3"],
        ""
    );
}

#[test]
fn let_chain() {
    do_test!(
        &"let yeah = 3;".repeat(5),
        [
            "let", "yeah", "=", "3", "let", "yeah", "=", "3", "let", "yeah", "=", "3", "let",
            "yeah", "=", "3", "let", "yeah", "=", "3"
        ],
        "",
    )
}

#[test]
fn let_and_fn() {
    do_test!(
        "\
        let yeah = 3;\n\
        print(yeah);\
        ",
        ["let", "yeah", "=", "3", "print", "(", "yeah", ")",],
        "",
    );
}

#[test]
fn fn_call_str() {
    do_test!(r#"print("yeah");"#, ["print", "(", "\"yeah\"", ")"], "");
}

#[test]
fn fn_fail_single_param() {
    do_test!(r#"print("""#, [], "eof 8");
    do_test!(r#"print("#, [], "eof 6");
    do_test!(r#"print(print"#, [], "eof 11");
    do_test!(
        r#"print(print("#,
        [],
        r#"
        eof 12
        eof 12"#,
    );
    do_test!(r#"print"#, [], r#"unexpected 0,5 = "print""#);
}

#[test]
fn fn_fail_multi_param() {
    do_test!(r#"print("yeah","""#, [], "eof 15");
    do_test!(r#"print(one"#, [], "eof 9");
    do_test!(r#"print(yeah, yeah(), """#, [], r#"eof 22"#);
    do_test!(
        r#"print(print(), print("#,
        [],
        r#"
            eof 21
            eof 21"#,
    );
    do_test!(r#"print(yeah, 1, print"#, [], "eof 20");
}

#[test]
fn nested_fn() {
    do_test!(
        r#"print(print());"#,
        ["print", "(", "print", "(", ")", ")"],
        r#""#
    );
    do_test!(
        r#"print(print(), print());"#,
        ["print", "(", "print", "(", ")", ",", "print", "(", ")", ")"],
        "",
    );
    do_test!(
        r#"print(print(print(print(print(print(print()))))));"#,
        [
            "print", "(", "print", "(", "print", "(", "print", "(", "print", "(", "print", "(",
            "print", "(", ")", ")", ")", ")", ")", ")", ")"
        ],
        "",
    );
    do_test!(
        r#"print(print(print(), ""), print(print(one, two, three, yeah(five))));"#,
        [
            "print", "(", "print", "(", "print", "(", ")", ",", "\"\"", ")", ",", "print", "(",
            "print", "(", "one", ",", "two", ",", "three", ",", "yeah", "(", "five", ")", ")", ")",
            ")",
        ],
        "",
    );
}

#[test]
fn block() {
    do_test!("{}", ["{", "}"], "",);
    do_test!("{print(yeah);}", ["{", "print", "(", "yeah", ")", "}"], "",);
    do_test!("{{}}", ["{", "{", "}", "}"], "",);
    do_test!(
        "{{print(yeah);}}",
        ["{", "{", "print", "(", "yeah", ")", "}", "}"],
        "",
    );
    do_test!(
        r#"{
            {
                let string yeah = "";
                print(yeah);
            }
        }"#,
        ["{", "{", "let", "string", "yeah", "=", "\"\"", "print", "(", "yeah", ")", "}", "}"],
        "",
    );
}
