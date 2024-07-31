use expect_test::expect;

use super::Reader;

#[test]
fn unexpected_punct() {
    let src = ",.@#~?:$=!<>-&|+*/^%";
    let mut reader = Reader::new(src);
    let token = reader.fn_next();
    assert_eq!(token, None);

    let expected_errors = expect![["ErrorMulti { errors: [Lecical(UnexpectedPunct(',')), \
        Lecical(UnexpectedPunct('.')), Lecical(UnexpectedPunct('@')), \
        Lecical(UnexpectedPunct('#')), Lecical(UnexpectedPunct('~')), \
        Lecical(UnexpectedPunct('?')), Lecical(UnexpectedPunct(':')), \
        Lecical(UnexpectedPunct('$')), Lecical(UnexpectedPunct('=')), \
        Lecical(UnexpectedPunct('!')), Lecical(UnexpectedPunct('<')), \
        Lecical(UnexpectedPunct('>')), Lecical(UnexpectedPunct('-')), \
        Lecical(UnexpectedPunct('&')), Lecical(UnexpectedPunct('|')), \
        Lecical(UnexpectedPunct('+')), Lecical(UnexpectedPunct('*')), \
        Lecical(UnexpectedPunct('/')), Lecical(UnexpectedPunct('^')), \
        Lecical(UnexpectedPunct('%'))] }"]];
    expected_errors.assert_eq(&format!("{:?}", reader.errors));
}
