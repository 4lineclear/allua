use std::rc::Rc;

use crate::parse::iter::RcChars;

use super::{token::TokenKind, Token};

/// Peekable iterator over a char sequence.
///
/// Next characters can be peeked via `first` method,
/// and position can be shifted forward via `bump` method.
#[derive(Debug)]
pub struct Cursor {
    /// the current bye position
    pos: u32,
    /// the position of the start of the previous token
    pub(super) token_pos: u32,
    len_remaining: usize,
    /// Iterator over chars. Slightly faster than a &str.
    chars: RcChars,
    prev: char,
    prev_token: Token,
}

impl From<&str> for Cursor {
    fn from(value: &str) -> Self {
        Self::new(value.into())
    }
}

impl From<Rc<str>> for Cursor {
    fn from(value: Rc<str>) -> Self {
        Self::new(value)
    }
}

impl From<String> for Cursor {
    fn from(value: String) -> Self {
        Self::new(value.into())
    }
}

pub const EOF_CHAR: char = '\0';

impl Cursor {
    #[must_use]
    pub fn new(input: Rc<str>) -> Self {
        Cursor {
            pos: 0,
            token_pos: 0,
            len_remaining: input.len(),
            chars: RcChars::new(input),
            prev: EOF_CHAR,
            prev_token: Token::new(TokenKind::Eof, 0),
        }
    }

    #[must_use]
    pub const fn pos(&self) -> u32 {
        self.pos
    }

    /// the position of the start of the previous token
    #[must_use]
    pub const fn token_pos(&self) -> u32 {
        self.token_pos
    }

    #[inline]
    #[must_use]
    pub fn src(&self) -> &str {
        self.chars.src()
    }

    /// Returns the last eaten symbol (or `'\0'` in release builds).
    /// (For debug assertions only.)
    #[must_use]
    pub const fn prev(&self) -> char {
        self.prev
    }

    /// Returns the last eaten token
    /// (For debug assertions only.)
    #[must_use]
    pub const fn prev_token(&self) -> Token {
        self.prev_token
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    #[must_use]
    pub fn first(&self) -> char {
        self.chars.first()
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[must_use]
    pub fn second(&self) -> char {
        self.chars.second()
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[must_use]
    pub fn third(&self) -> char {
        self.chars.third()
    }

    /// Checks if there is nothing more to consume.
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.chars.is_eof()
    }

    /// Returns amount of already consumed symbols.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    #[allow(clippy::cast_possible_truncation)]
    pub fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.pos += c.len_utf8() as u32;

        #[cfg(debug_assertions)]
        {
            self.prev = c;
        }

        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // It was tried making optimized version of this for eg. line comments, but
        // LLVM can inline all of this and compile it down to fast iteration over bytes.
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }
}
