use std::{iter::FusedIterator, rc::Rc, str::Chars};

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
        Self {
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

// TODO: move everything to cursor
#[derive(Debug, Clone)]
pub struct RcChars {
    src: Rc<str>,
    chars: Chars<'static>,
}

impl RcChars {
    /// Creates a new char iter
    ///
    /// # SAFETY
    ///
    /// points to inner rc
    #[must_use]
    pub fn new(rc: Rc<str>) -> Self {
        #[allow(unsafe_code)]
        let chars = unsafe { std::mem::transmute::<Chars, Chars<'static>>(rc.chars()) };
        Self { src: rc, chars }
    }

    /// The current chars as a string
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.chars.as_str()
    }

    /// Set the original source
    #[inline]
    #[must_use]
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Creates a new iterator with a reset position
    ///
    /// [`Clone`] it to retain position
    #[must_use]
    pub fn clone_reset(&self) -> Self {
        Self::new(self.src.clone())
    }

    /// Peeks the next symbol from the input stream without consuming it.
    /// If requested position doesn't exist, `EOF_CHAR` is returned.
    /// However, getting `EOF_CHAR` doesn't always mean actual end of file,
    /// it should be checked with `is_eof` method.
    #[must_use]
    pub fn first(&self) -> char {
        // `.next()` optimizes better than `.nth(0)`
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the second symbol from the input stream without consuming it.
    #[must_use]
    pub fn second(&self) -> char {
        // `.next()` optimizes better than `.nth(1)`
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    /// Peeks the third symbol from the input stream without consuming it.
    #[must_use]
    pub fn third(&self) -> char {
        // `.next()` optimizes better than `.nth(1)`
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
}

impl From<&str> for RcChars {
    fn from(value: &str) -> Self {
        Self::new(value.into())
    }
}

impl From<Rc<str>> for RcChars {
    fn from(value: Rc<str>) -> Self {
        Self::new(value)
    }
}

impl From<String> for RcChars {
    fn from(value: String) -> Self {
        Self::new(value.into())
    }
}

// NOTE: should use char's functions whenever possible
impl Iterator for RcChars {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.chars.next()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.chars.count()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.chars.size_hint()
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.chars.last()
    }
}

impl DoubleEndedIterator for RcChars {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.chars.next_back()
    }
}

impl FusedIterator for RcChars {}

#[cfg(test)]
mod test {
    use super::RcChars;

    #[test]
    fn basic() {
        let mut chars = RcChars::from("one");
        assert_eq!(chars.next(), Some('o'));
        assert_eq!(chars.next(), Some('n'));
        assert_eq!(chars.next(), Some('e'));

        let mut chars2 = chars.clone();
        assert_eq!(chars2.next(), None);

        let mut chars3 = chars.clone_reset();
        assert_eq!(chars3.next(), Some('o'));
        assert_eq!(chars3.next(), Some('n'));
        assert_eq!(chars3.next(), Some('e'));

        let mut chars4 = chars.clone_reset();

        drop(chars);
        drop(chars2);
        drop(chars3);

        assert_eq!(chars4.next(), Some('o'));
        assert_eq!(chars4.next(), Some('n'));
        assert_eq!(chars4.next(), Some('e'));
    }
}
