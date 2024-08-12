//! pre-lexing
//!
//! At this level, error handling and keywords don't exist
#![allow(clippy::unnested_or_patterns)]

use unicode_properties::UnicodeEmoji;

use super::util::{is_id_continue, is_id_start, is_whitespace};

#[cfg(test)]
pub mod test;

pub mod cursor;
pub mod token;
pub mod unescape;

pub use cursor::{Cursor, EOF_CHAR};
pub use token::{
    Base, DocStyle,
    LexKind::{self, *},
    Lexeme,
    LiteralKind::{self, *},
    RawStrError,
};

pub fn tokenize(input: &str) -> impl Iterator<Item = Lexeme> + '_ {
    Cursor::new(input)
}

#[must_use]
pub fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|start| is_id_start(start) && chars.all(is_id_continue))
}

/// Validates a raw string literal. Used for getting more information about a
/// problem with a `RawStr`/`RawByteStr` with a `None` field.
///
/// # Errors
///
/// see [`RawStrError`]
///
/// # Panics
///
/// Panics if the given `prefix_len` is incorrect
#[inline]
pub fn validate_raw_str(input: &str, prefix_len: usize) -> Result<(), RawStrError> {
    debug_assert!(!input.is_empty());
    let mut cursor = Cursor::new(input);
    // Move past the leading `r` or `br`.
    for _ in 0..prefix_len {
        cursor.bump().unwrap();
    }
    cursor.raw_double_quoted_string(prefix_len).map(|_| ())
}

impl Cursor<'_> {
    /// Parses a token from the input string.
    pub fn advance_token(&mut self) -> Lexeme {
        self.token_pos = self.pos();
        let Some(first_char) = self.bump() else {
            return Lexeme::new(LexKind::Eof, 0);
        };
        let token_kind = match first_char {
            // Slash, comment or block comment.
            '/' => match self.first() {
                '/' => self.line_comment(),
                '*' => self.block_comment(),
                _ => Slash,
            },

            // Whitespace sequence.
            c if is_whitespace(c) => self.whitespace(),

            // Raw identifier, raw string literal or identifier.
            'r' => match (self.first(), self.second()) {
                ('#', c1) if is_id_start(c1) => self.raw_ident(),
                ('#', _) | ('"', _) => {
                    let res = self.raw_double_quoted_string(1);
                    let suffix_start = self.pos_within_token();
                    if res.is_ok() {
                        self.eat_literal_suffix();
                    }
                    let kind = RawStr { n_hashes: res.ok() };
                    Literal { kind, suffix_start }
                }
                _ => self.ident_or_unknown_prefix(),
            },

            // Byte literal, byte string literal, raw byte string literal or identifier.
            'b' => self.c_or_byte_string(
                |terminated| ByteStr { terminated },
                |n_hashes| RawByteStr { n_hashes },
                Some(|terminated| Byte { terminated }),
            ),

            // c-string literal, raw c-string literal or identifier.
            'c' => self.c_or_byte_string(
                |terminated| CStr { terminated },
                |n_hashes| RawCStr { n_hashes },
                None,
            ),

            // Identifier (this should be checked after other variant that can
            // start as identifier).
            c if is_id_start(c) => self.ident_or_unknown_prefix(),

            // Numeric literal.
            c @ '0'..='9' => {
                let literal_kind = self.number(c);
                let suffix_start = self.pos_within_token();
                self.eat_literal_suffix();
                LexKind::Literal {
                    kind: literal_kind,
                    suffix_start,
                }
            }

            // One-symbol tokens.
            ';' => Semi,
            ',' => Comma,
            '.' => Dot,
            '(' => OpenParen,
            ')' => CloseParen,
            '{' => OpenBrace,
            '}' => CloseBrace,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '@' => At,
            '#' => Pound,
            '~' => Tilde,
            '?' => Question,
            ':' => Colon,
            '$' => Dollar,
            '=' => Eq,
            '!' => Bang,
            '<' => Lt,
            '>' => Gt,
            '-' => Minus,
            '&' => And,
            '|' => Or,
            '+' => Plus,
            '*' => Star,
            '^' => Caret,
            '%' => Percent,

            // Character literal.
            '\'' => self.char(),

            // String literal.
            '"' => self.string(),
            // Identifier starting with an emoji. Only lexed for graceful error recovery.
            c if !c.is_ascii() && c.is_emoji_char() => self.fake_ident_or_unknown_prefix(),
            _ => Unknown,
        };
        let res = Lexeme::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        res
    }

    fn line_comment(&mut self) -> LexKind {
        debug_assert!(self.prev() == '/' && self.first() == '/');
        self.bump();

        let doc_style = match self.first() {
            // `//!` is an inner line doc comment.
            '!' => Some(DocStyle::Inner),
            // `////` (more than 3 slashes) is not considered a doc comment.
            '/' if self.second() != '/' => Some(DocStyle::Outer),
            _ => None,
        };

        self.eat_while(|c| c != '\n');
        LineComment { doc_style }
    }

    fn block_comment(&mut self) -> LexKind {
        debug_assert!(self.prev() == '/' && self.first() == '*');
        self.bump();

        let doc_style = match self.first() {
            // `/*!` is an inner block doc comment.
            '!' => Some(DocStyle::Inner),
            // `/***` (more than 2 stars) is not considered a doc comment.
            // `/**/` is not considered a doc comment.
            '*' if !matches!(self.second(), '*' | '/') => Some(DocStyle::Outer),
            _ => None,
        };

        let mut depth = 1usize;
        while let Some(c) = self.bump() {
            match c {
                '/' if self.first() == '*' => {
                    self.bump();
                    depth += 1;
                }
                '*' if self.first() == '/' => {
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        // This block comment is closed, so for a construction like "/* */ */"
                        // there will be a successfully parsed block comment "/* */"
                        // and " */" will be processed separately.
                        break;
                    }
                }
                _ => (),
            }
        }

        BlockComment {
            doc_style,
            terminated: depth == 0,
        }
    }

    fn whitespace(&mut self) -> LexKind {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        Whitespace
    }

    fn raw_ident(&mut self) -> LexKind {
        debug_assert!(self.prev() == 'r' && self.first() == '#' && is_id_start(self.second()));
        // Eat "#" symbol.
        self.bump();
        // Eat the identifier part of RawIdent.
        self.eat_identifier();
        RawIdent
    }

    fn ident_or_unknown_prefix(&mut self) -> LexKind {
        debug_assert!(is_id_start(self.prev()));
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(is_id_continue);
        // Known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix.
        match self.first() {
            '#' | '"' | '\'' => InvalidPrefix,
            c if !c.is_ascii() && c.is_emoji_char() => self.fake_ident_or_unknown_prefix(),
            _ => Ident,
        }
    }

    fn fake_ident_or_unknown_prefix(&mut self) -> LexKind {
        // Start is already eaten, eat the rest of identifier.
        self.eat_while(|c| {
            unicode_ident::is_xid_continue(c)
                || (!c.is_ascii() && c.is_emoji_char())
                || c == '\u{200d}'
        });

        // Known prefixes must have been handled earlier. So if
        // we see a prefix here, it is definitely an unknown prefix.
        match self.first() {
            '#' | '"' | '\'' => InvalidPrefix,
            _ => InvalidIdent,
        }
    }

    pub fn string(&mut self) -> LexKind {
        let terminated = self.double_quoted_string();
        let suffix_start = self.pos_within_token();
        if terminated {
            self.eat_literal_suffix();
        }
        let kind = Str { terminated };
        Literal { kind, suffix_start }
    }

    fn c_or_byte_string(
        &mut self,
        mk_kind: impl FnOnce(bool) -> LiteralKind,
        mk_kind_raw: impl FnOnce(Option<u8>) -> LiteralKind,
        single_quoted: Option<fn(bool) -> LiteralKind>,
    ) -> LexKind {
        match (self.first(), self.second(), single_quoted) {
            ('\'', _, Some(mk_kind)) => {
                self.bump();
                let terminated = self.single_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind(terminated);
                Literal { kind, suffix_start }
            }
            ('"', _, _) => {
                self.bump();
                let terminated = self.double_quoted_string();
                let suffix_start = self.pos_within_token();
                if terminated {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind(terminated);
                Literal { kind, suffix_start }
            }
            ('r', '"', _) | ('r', '#', _) => {
                self.bump();
                let res = self.raw_double_quoted_string(2);
                let suffix_start = self.pos_within_token();
                if res.is_ok() {
                    self.eat_literal_suffix();
                }
                let kind = mk_kind_raw(res.ok());
                Literal { kind, suffix_start }
            }
            _ => self.ident_or_unknown_prefix(),
        }
    }

    fn number(&mut self, first_digit: char) -> LiteralKind {
        debug_assert!('0' <= self.prev() && self.prev() <= '9');
        let mut base = Base::Decimal;
        if first_digit == '0' {
            // Attempt to parse encoding base.
            match self.first() {
                'b' => {
                    base = Base::Binary;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'o' => {
                    base = Base::Octal;
                    self.bump();
                    if !self.eat_decimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                'x' => {
                    base = Base::Hexadecimal;
                    self.bump();
                    if !self.eat_hexadecimal_digits() {
                        return Int {
                            base,
                            empty_int: true,
                        };
                    }
                }
                // Not a base prefix; consume additional digits.
                '0'..='9' | '_' => {
                    self.eat_decimal_digits();
                }

                // Also not a base prefix; nothing more to do here.
                '.' | 'e' | 'E' => {}

                // Just a 0.
                _ => {
                    return Int {
                        base,
                        empty_int: false,
                    }
                }
            }
        } else {
            // No base prefix, parse number in the usual way.
            self.eat_decimal_digits();
        };

        match self.first() {
            // Don't be greedy if this is actually an
            // integer literal followed by field/method access or a range pattern
            // (`0..2` and `12.foo()`)
            '.' if self.second() != '.' && !is_id_start(self.second()) => {
                // might have stuff after the ., and if it does, it needs to start
                // with a number
                self.bump();
                let mut empty_exponent = false;
                if self.first().is_ascii_digit() {
                    self.eat_decimal_digits();
                    match self.first() {
                        'e' | 'E' => {
                            self.bump();
                            empty_exponent = !self.eat_float_exponent();
                        }
                        _ => (),
                    }
                }
                Float {
                    base,
                    empty_exponent,
                }
            }
            'e' | 'E' => {
                self.bump();
                let empty_exponent = !self.eat_float_exponent();
                Float {
                    base,
                    empty_exponent,
                }
            }
            _ => Int {
                base,
                empty_int: false,
            },
        }
    }

    fn char(&mut self) -> LexKind {
        debug_assert!(self.prev() == '\'');
        let terminated = self.single_quoted_string();
        let suffix_start = self.pos_within_token();
        if terminated {
            self.eat_literal_suffix();
        }
        let kind = Char { terminated };
        Literal { kind, suffix_start }
    }

    fn single_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '\'');
        // Check if it's a one-symbol literal.
        if self.second() == '\'' && self.first() != '\\' {
            self.bump();
            self.bump();
            return true;
        }

        // Literal has more than one symbol.

        // Parse until either quotes are terminated or error is detected.
        loop {
            match self.first() {
                // Quotes are terminated, finish parsing.
                '\'' => {
                    self.bump();
                    return true;
                }
                // Probably beginning of the comment, which we don't want to include
                // to the error report.
                '/' => break,
                // Newline without following '\'' means unclosed quote, stop parsing.
                '\n' if self.second() != '\'' => break,
                // End of file, stop parsing.
                EOF_CHAR if self.is_eof() => break,
                // Escaped slash is considered one character, so bump twice.
                '\\' => {
                    self.bump();
                    self.bump();
                }
                // Skip the character.
                _ => {
                    self.bump();
                }
            }
        }
        // String was not terminated.
        false
    }

    /// Eats double-quoted string and returns true
    /// if string is terminated.
    fn double_quoted_string(&mut self) -> bool {
        debug_assert!(self.prev() == '"');
        while let Some(c) = self.bump() {
            match c {
                '"' => {
                    return true;
                }
                '\\' if self.first() == '\\' || self.first() == '"' => {
                    // Bump again to skip escaped character.
                    self.bump();
                }
                _ => (),
            }
        }
        // End of file reached.
        false
    }

    /// Eats the double-quoted string and returns `n_hashes` and an error if encountered.
    fn raw_double_quoted_string(&mut self, prefix_len: usize) -> Result<u8, RawStrError> {
        // Wrap the actual function to handle the error with too many hashes.
        // This way, it eats the whole raw string.
        let n_hashes = self.raw_string_unvalidated(prefix_len)?;
        // Only up to 255 `#`s are allowed in raw strings
        u8::try_from(n_hashes).map_err(|_| RawStrError::TooManyDelimiters { found: n_hashes })
    }

    fn raw_string_unvalidated(&mut self, prefix_len: usize) -> Result<usize, RawStrError> {
        debug_assert!(self.prev() == 'r');
        let start_pos = self.pos_within_token();
        let mut possible_terminator_offset = None;
        let mut max_hashes = 0;

        // Count opening '#' symbols.
        let mut eaten = 0;
        while self.first() == '#' {
            eaten += 1;
            self.bump();
        }
        let n_start_hashes = eaten;

        // Check that string is started.
        match self.bump() {
            Some('"') => (),
            c => {
                let c = c.unwrap_or(EOF_CHAR);
                return Err(RawStrError::InvalidStarter { bad_char: c });
            }
        }

        // Skip the string contents and on each '#' character met, check if this is
        // a raw string termination.
        loop {
            self.eat_while(|c| c != '"');

            if self.is_eof() {
                return Err(RawStrError::NoTerminator {
                    expected: n_start_hashes,
                    found: max_hashes,
                    possible_terminator_offset,
                });
            }

            // Eat closing double quote.
            self.bump();

            // Check that amount of closing '#' symbols
            // is equal to the amount of opening ones.
            // Note that this will not consume extra trailing `#` characters:
            // `r###"abcde"####` is lexed as a `RawStr { n_hashes: 3 }`
            // followed by a `#` token.
            let mut n_end_hashes = 0;
            while self.first() == '#' && n_end_hashes < n_start_hashes {
                n_end_hashes += 1;
                self.bump();
            }

            if n_end_hashes == n_start_hashes {
                return Ok(n_start_hashes);
            } else if n_end_hashes > max_hashes {
                // Keep track of possible terminators to give a hint about
                // where there might be a missing terminator
                possible_terminator_offset =
                    Some(self.pos_within_token() - start_pos - n_end_hashes + prefix_len);
                max_hashes = n_end_hashes;
            }
        }
    }

    fn eat_decimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    fn eat_hexadecimal_digits(&mut self) -> bool {
        let mut has_digits = false;
        loop {
            match self.first() {
                '_' => {
                    self.bump();
                }
                '0'..='9' | 'a'..='f' | 'A'..='F' => {
                    has_digits = true;
                    self.bump();
                }
                _ => break,
            }
        }
        has_digits
    }

    /// Eats the float exponent. Returns true if at least one digit was met,
    /// and returns false otherwise.
    fn eat_float_exponent(&mut self) -> bool {
        debug_assert!(self.prev() == 'e' || self.prev() == 'E');
        if self.first() == '-' || self.first() == '+' {
            self.bump();
        }
        self.eat_decimal_digits()
    }

    // Eats the suffix of the literal, e.g. "u8".
    fn eat_literal_suffix(&mut self) {
        self.eat_identifier();
    }

    // Eats the identifier. Note: succeeds on `_`, which isn't a valid
    // identifier.
    fn eat_identifier(&mut self) {
        if !is_id_start(self.first()) {
            return;
        }
        self.bump();

        self.eat_while(is_id_continue);
    }
}

impl Iterator for Cursor<'_> {
    type Item = Lexeme;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.advance_token();
        (token.kind != LexKind::Eof).then_some(token)
    }
}
