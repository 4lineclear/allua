//! this module concerns spans

/// A byte span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSpan {
    pub from: u32,
    pub to: u32,
}

impl BSpan {
    #[must_use]
    pub const fn new(from: u32, to: u32) -> Self {
        Self { from, to }
    }
    #[must_use]
    pub const fn from_len(from: u32, len: u32) -> Self {
        Self::new(from, from + len)
    }
}

/// A token span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TSpan {
    pub from: u32,
    pub to: u32,
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
