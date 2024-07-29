pub mod token;
//
// pub fn ws_start<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     preceded(multispace0, inner)
// }
//
// pub fn ws_end<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     terminated(inner, multispace0)
// }
//
// /// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
// /// trailing whitespace, returning the output of `inner`.
// pub fn ws<'a, O, E: ParseError<&'a str>, F>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Parser<&'a str, O, E>,
// {
//     delimited(multispace0, inner, multispace0)
// }
//
// pub fn identifier(input: &str) -> IResult<&str, &str, Error<&str>> {
//     recognize(all_consuming(pair(
//         verify(anychar, |&ch| is_xid_start(ch) || ch == '_'),
//         many0_count(verify(anychar, |&ch| is_xid_continue(ch))),
//     )))
//     .parse(input)
// }
//
// /// parse top level fn
// ///
// /// eq a rust mod
// pub fn parse<'a>(name: &'a str, s: &'a str) -> IResult<&'a str, Fn<'a>, Error<&'a str>> {
//     let params = CommaList { tokens: Vec::new() };
//     let (s, tokens) = parse_tokens().parse(s)?;
//     Ok((
//         s,
//         Fn {
//             name,
//             params,
//             tokens,
//         },
//     ))
// }
//
// fn parse_tokens<'a>() -> impl nom::Parser<&'a str, TokenSet<'a>, Error<&'a str>> {
//     many0(parse_token())
// }
//
// fn parse_token<'a>() -> impl nom::Parser<&'a str, Token<'a>, Error<&'a str>> {
//     alt((parse_decl.map(Token::from), parse_expr.map(Token::from)))
// }
//
// fn parse_decl(s: &str) -> IResult<&str, Decl, Error<&str>> {
//     let (s, ty) = parse_decl_type(s)?;
//     let decl = Decl {
//         ty,
//         name: todo!(),
//         value: todo!(),
//     };
//     Ok((s, decl))
// }
//
// fn parse_decl_type(s: &str) -> IResult<&str, DeclType, Error<&str>> {
//     let (s, ty) = ws_start(identifier)(s)?;
//     match ty {
//         "let" => Ok((s, DeclType::Let)),
//         "const" => {
//             let (s, ty_name) = ws_start(identifier)(s)?;
//             Ok((s, DeclType::ConstType(ty_name)))
//         }
//         ty => Ok((s, DeclType::Type(ty))),
//     }
// }
//
// fn parse_expr<I>(s: &str) -> IResult<&str, Expr, Error<I>> {
//     let expr = Expr::FnCall {
//         name: todo!(),
//         params: todo!(),
//     };
//     Ok((s, expr))
// }
//
