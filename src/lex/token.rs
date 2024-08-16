//! primitive lexing

/// Parsed token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lexeme {
    pub kind: LexKind,
    pub len: usize,
}

impl Lexeme {
    #[must_use]
    pub const fn new(kind: LexKind, len: usize) -> Self {
        Self { kind, len }
    }
}

/// Enum representing common lexeme types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LexKind {
    // Multi-char tokens:
    /// "// comment"
    LineComment { doc_style: Option<DocStyle> },

    /// `/* block comment */`
    ///
    /// Block comments can be recursive, so a sequence like `/* /* */`
    /// will not be considered terminated and will result in a parsing error.
    BlockComment {
        doc_style: Option<DocStyle>,
        terminated: bool,
    },

    /// Any whitespace character sequence.
    Whitespace,

    /// "ident" or "continue"
    ///
    /// At this step, keywords are also considered identifiers.
    Ident,

    /// "r#ident"
    RawIdent,

    /// Like the above, but containing invalid unicode codepoints.
    InvalidIdent,

    /// invalid string prefix, for emojis
    InvalidPrefix,

    /// Examples: `12u8`, `1.0e-40`, `b"123"`. Note that `_` is an invalid
    /// suffix, but may be present here on string and float literals. Users of
    /// this type will need to check for and reject that case.
    ///
    /// See [`LiteralKind`] for more details.
    Literal {
        kind: LiteralKind,
        suffix_start: usize,
    },

    // One-char tokens:
    /// ";"
    Semi,
    /// ","
    Comma,
    /// "."
    Dot,
    /// "("
    OpenParen,
    /// ")"
    CloseParen,
    /// "{"
    OpenBrace,
    /// "}"
    CloseBrace,
    /// "["
    OpenBracket,
    /// "]"
    CloseBracket,
    /// "@"
    At,
    /// "#"
    Pound,
    /// "~"
    Tilde,
    /// "?"
    Question,
    /// ":"
    Colon,
    /// "$"
    Dollar,
    /// "="
    Eq,
    /// "!"
    Bang,
    /// "<"
    Lt,
    /// ">"
    Gt,
    /// "-"
    Minus,
    /// "&"
    And,
    /// "|"
    Or,
    /// "+"
    Plus,
    /// "*"
    Star,
    /// "/"
    Slash,
    /// "^"
    Caret,
    /// "%"
    Percent,

    /// Unknown token, not expected by the lexer, e.g. "№"
    Unknown,

    /// End of input.
    Eof,
}

// /// the id of a lexkind
// ///
// /// considers alls literals to be the same
// #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
// pub struct LexKindId(u64);
//
// impl Deref for LexKindId {
//     type Target = u64;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// impl BitOr for LexKind {
//     type Output = LexKindId;
//
//     fn bitor(self, rhs: Self) -> Self::Output {
//         self.id() | rhs.id()
//     }
// }
//
// impl BitOr for LexKindId {
//     type Output = LexKindId;
//
//     fn bitor(self, rhs: Self) -> Self::Output {
//         Self(self.0 + rhs.0)
//     }
// }
//
impl LexKind {
    // NOTE: consider eliminating possible ids to be up to 31
    // #[must_use]
    // pub const fn id(self) -> LexKindId {
    //     use LexKind::*;
    //     let shift = match self {
    //         LineComment { .. } => 1,
    //         BlockComment { .. } => 2,
    //         Whitespace => 3,
    //         Ident => 4,
    //         RawIdent => 5,
    //         InvalidIdent => 6,
    //         InvalidPrefix => 7,
    //         Literal { .. } => 8,
    //         Semi => 9,
    //         Comma => 10,
    //         Dot => 11,
    //         OpenParen => 12,
    //         CloseParen => 13,
    //         OpenBrace => 14,
    //         CloseBrace => 15,
    //         OpenBracket => 16,
    //         CloseBracket => 17,
    //         At => 18,
    //         Pound => 19,
    //         Tilde => 20,
    //         Question => 21,
    //         Colon => 22,
    //         Dollar => 23,
    //         Eq => 24,
    //         Bang => 25,
    //         Lt => 26,
    //         Gt => 27,
    //         Minus => 28,
    //         And => 29,
    //         Or => 30,
    //         Plus => 31,
    //         Star => 32,
    //         Slash => 33,
    //         Caret => 34,
    //         Percent => 35,
    //         Unknown => 36,
    //         Eof => 0,
    //     };
    //     LexKindId(1 << shift)
    // }

    #[must_use]
    pub const fn name(self) -> &'static str {
        use LexKind::*;
        match self {
            LineComment { .. } => "line comment",
            BlockComment { .. } => "block comment",
            Whitespace => "whitespace",
            Ident => "ident",
            RawIdent => "r#ident",
            InvalidIdent => "invalid ident",
            InvalidPrefix => "invalid prefix",
            Literal { .. } => "literal",
            Semi => "semicolon",
            Comma => "comma",
            Dot => "dot",
            OpenParen => "open parenthesis",
            CloseParen => "close parenthesis",
            OpenBrace => "open brace",
            CloseBrace => "close brace",
            OpenBracket => "open bracket",
            CloseBracket => "close bracket",
            At => "@",
            Pound => "#",
            Tilde => "~",
            Question => "?",
            Colon => ":",
            Dollar => "$",
            Eq => "=",
            Bang => "!",
            Lt => "<",
            Gt => ">",
            Minus => "-",
            And => "&",
            Or => "|",
            Plus => "+",
            Star => "*",
            Slash => "/",
            Caret => "^",
            Percent => "%",
            Unknown => "unknown",
            Eof => "end of file",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocStyle {
    Outer,
    Inner,
}

/// Enum representing the literal types supported by the lexer.
///
/// Note that the suffix is *not* considered when deciding the `LiteralKind` in
/// this type. This means that float literals like `1f32` are classified by this
/// type as `Int`. (Compare against `rustc_ast::token::LitKind` and
/// `rustc_ast::ast::LitKind`).
#[allow(clippy::doc_markdown)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralKind {
    /// "12_u8", "0o100", "0b120i99", "1f32".
    Int { base: Base, empty_int: bool },
    /// "12.34f32", "1e3", but not "1f32".
    Float { base: Base, empty_exponent: bool },
    /// "'a'", "'\\'", "'''", "';"
    Char { terminated: bool },
    /// "b'a'", "b'\\'", "b'''", "b';"
    Byte { terminated: bool },
    /// ""abc"", ""abc"
    Str { terminated: bool },
    /// "b"abc"", "b"abc"
    ByteStr { terminated: bool },
    /// `c"abc"`, `c"abc`
    CStr { terminated: bool },
    /// "r"abc"", "r#"abc"#", "r####"ab"###"c"####", "r#"a". `None` indicates
    /// an invalid literal.
    RawStr { n_hashes: Option<u8> },
    /// "br"abc"", "br#"abc"#", "br####"ab"###"c"####", "br#"a". `None`
    /// indicates an invalid literal.
    RawByteStr { n_hashes: Option<u8> },
    /// `cr"abc"`, "cr#"abc"#", `cr#"a`. `None` indicates an invalid literal.
    RawCStr { n_hashes: Option<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawStrError {
    /// Non `#` characters exist between `r` and `"`, e.g. `r##~"abcde"##`
    InvalidStarter { bad_char: char },
    /// The string was not terminated, e.g. `r###"abcde"##`.
    /// `possible_terminator_offset` is the number of characters after `r` or
    /// `br` where they may have intended to terminate it.
    NoTerminator {
        expected: usize,
        found: usize,
        possible_terminator_offset: Option<usize>,
    },
    /// More than 255 `#`s exist.
    TooManyDelimiters { found: usize },
}

/// Base of numeric literal encoding according to its prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Base {
    /// Literal starts with "0b".
    Binary = 2,
    /// Literal starts with "0o".
    Octal = 8,
    /// Literal doesn't contain a prefix.
    Decimal = 10,
    /// Literal starts with "0x".
    Hexadecimal = 16,
}
