//! this module concerns spans

/// A byte span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BSpan {
    pub from: u32,
    pub to: u32,
}

/// A token span
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TSpan {
    pub from: u32,
    pub to: u32,
}

impl BSpan {
    pub fn is_empty(self) -> bool {
        self.from == self.to
    }
}

impl TSpan {
    pub fn is_empty(self) -> bool {
        self.from == self.to
    }
}