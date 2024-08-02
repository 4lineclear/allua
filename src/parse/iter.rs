use std::iter::FusedIterator;
use std::rc::Rc;
use std::str::Chars;

#[derive(Debug, Clone)]
pub struct RcChars {
    rc: Rc<str>,
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
        Self { rc, chars }
    }

    pub fn as_str(&self) -> &str {
        &self.rc
    }

    /// Creates a new iterator with a reset position
    ///
    /// [`Clone`] it to retain position
    pub fn clone_reset(&self) -> Self {
        Self::new(self.rc.clone())
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
