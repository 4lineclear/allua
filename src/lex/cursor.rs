use std::str::Chars;

use super::{token::LexKind, Lexeme};

// PERF: Chars is slightly faster than &str

/// Peekable iterator over a char sequence.
///
/// Next characters can be peeked via `first` method,
/// and position can be shifted forward via `bump` method.
#[derive(Debug)]
pub struct Cursor<'a> {
    pub(super) token_pos: usize,
    len_remaining: usize,
    src: &'a str,
    chars: Chars<'a>,
    prev: char,
    prev_token: Lexeme,
}

impl<'a> Cursor<'a> {
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            token_pos: 0,
            len_remaining: input.len(),
            src: input,
            chars: input.chars(),
            prev: EOF_CHAR,
            prev_token: Lexeme::new(LexKind::Eof, 0),
        }
    }
}

pub const EOF_CHAR: char = '\0';

impl Cursor<'_> {
    /// The current subslice as a string

    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.chars.as_str()
    }

    #[must_use]
    // #[allow(clippy::cast_possible_truncation)]
    pub fn pos(&self) -> usize {
        self.src.len() - self.chars.as_str().len()
    }

    /// the position of the start of the previous token
    #[must_use]
    pub const fn token_pos(&self) -> usize {
        self.token_pos
    }

    #[inline]
    #[must_use]
    pub const fn src(&self) -> &str {
        self.src
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
    pub const fn prev_token(&self) -> Lexeme {
        self.prev_token
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    #[must_use]
    pub fn first(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(0)`
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[must_use]
    pub fn second(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[must_use]
    pub fn third(&self) -> char {
        // PERF: `.next()` optimizes better than `.nth(2)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns amount of already consumed symbols.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn pos_within_token(&self) -> usize {
        self.len_remaining - self.chars.as_str().len()
    }

    /// Resets the number of bytes consumed to 0.
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
    #[allow(clippy::cast_possible_truncation)]
    pub fn bump(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.prev = c;
        Some(c)
    }

    /// Eats symbols while predicate returns true or until the end of file is reached.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        // PERF: It was tried making optimized version of this for eg. line comments, but
        // LLVM can inline all of this and compile it down to fast iteration over bytes.
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }
}
