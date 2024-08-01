use std::str::Chars;

use super::{token::TokenKind, Token};

/// Peekable iterator over a char sequence.
///
/// Next characters can be peeked via `first` method,
/// and position can be shifted forward via `bump` method.
#[derive(Debug)]
pub struct Cursor<'a> {
    /// the current bye position
    pos: u32,
    /// the position of the start of the previous token
    pub(super) token_pos: u32,
    len_remaining: usize,
    /// Iterator over chars. Slightly faster than a &str.
    chars: Chars<'a>,
    #[cfg(debug_assertions)]
    prev: char,
    #[cfg(debug_assertions)]
    prev_token: Token,
}

pub const EOF_CHAR: char = '\0';

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Cursor<'a> {
        Cursor {
            pos: 0,
            token_pos: 0,
            len_remaining: input.len(),
            chars: input.chars(),
            #[cfg(debug_assertions)]
            prev: EOF_CHAR,
            #[cfg(debug_assertions)]
            prev_token: Token::new(TokenKind::Eof, 0),
        }
    }

    pub fn pos(&self) -> u32 {
        self.pos
    }

    /// the position of the start of the previous token
    pub fn token_pos(&self) -> u32 {
        self.token_pos
    }

    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Returns the last eaten symbol
    /// (For debug assertions only.)
    #[cfg(debug_assertions)]
    pub fn prev(&self) -> char {
        self.prev
    }

    /// Returns the last eaten token
    /// (For debug assertions only.)
    #[cfg(debug_assertions)]
    pub fn prev_token(&self) -> Token {
        self.prev_token
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    pub fn first(&self) -> char {
        // `.next()` optimizes better than `.nth(0)`
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    pub fn second(&self) -> char {
        // `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    pub fn third(&self) -> char {
        // `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Checks if there is nothing more to consume.
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// Returns amount of already consumed symbols.
    pub fn pos_within_token(&self) -> u32 {
        (self.len_remaining - self.chars.as_str().len()) as u32
    }

    /// Resets the number of bytes consumed to 0.
    pub fn reset_pos_within_token(&mut self) {
        self.len_remaining = self.chars.as_str().len();
    }

    /// Moves to the next character.
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
