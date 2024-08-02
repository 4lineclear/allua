use std::iter::FusedIterator;
use std::rc::Rc;
use std::str::Chars;

use crate::lex::EOF_CHAR;

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
