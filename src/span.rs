//! this module concerns spans

/// A byte span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSpan {
    pub from: usize,
    pub to: usize,
}

impl BSpan {
    #[must_use]
    pub const fn new(from: usize, to: usize) -> Self {
        Self { from, to }
    }
    #[must_use]
    pub const fn from_len(from: usize, len: usize) -> Self {
        Self::new(from, from + len)
    }
}

/// A token span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TSpan {
    pub from: usize,
    pub to: usize,
}

impl BSpan {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
}

impl TSpan {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.from == self.to
    }
}
