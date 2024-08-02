// NOTE:
// is run automatically when imported, unless `defer` is used

use std::marker::PhantomData;

use crate::{lex, util::Symbol};

/// a module of code
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module {
    name: Symbol,
    tokens: Span<Token>,
    /// First item must be a fn
    items: Vec<Token>,
}

impl Module {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            tokens: Span {
                from: 1,
                to: 1,
                kind: PhantomData,
            },
            items: Vec::new(),
        }
    }

    #[must_use]
    pub const fn span(&self) -> Span<Token> {
        self.tokens
    }

    pub fn push(&mut self, token: impl Into<Token>) {
        self.items.push(token.into());
        self.tokens.to += 1;
    }
}

/// A user defined function
///
/// Acts as both as a module, datatype and function
///
///
/// ```ignore
/// fn ::name(::params) then
///   ::tokens
/// end
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fn {
    name: Symbol,
    params: Span<Decl>,
    tokens: Span<Token>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token {
    Fn(Fn),
    Decl(Decl),
    Expr(Expr),
    Value(Value),
    Import(Import),
}

macro_rules! token_from {
    ($($name:ident),*) => {$(
        impl<'a> From<$name> for Token {
            fn from(value: $name) -> Self {
                Self::$name(value)
            }
        }
    )*};
}

token_from!(Fn, Decl, Expr, Value, Import);

/// [`DeclType`] <name> ?(= <value>);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decl {
    /// if none try implicit
    kind: DeclKind,
    name: Symbol,
    value: Option<Expr>,
}

impl Decl {
    #[must_use]
    pub const fn new(kind: DeclKind, name: Symbol, value: Option<Expr>) -> Self {
        Self { kind, name, value }
    }
}

/// <type> | `let` | `const` | `const` <type>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclKind {
    /// let <name>
    Let,
    /// <type> <name>
    Type(Symbol),
    /// const <name>
    Const,
    /// const <type> <name>
    ConstType(Symbol),
}

/// ?(<defer>) `use` <name>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    name: Symbol,
    defer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value {
    value: Symbol,
    kind: lex::LiteralKind,
    suffix_start: u32,
}

impl Value {
    #[must_use]
    pub const fn new(value: Symbol, kind: lex::LiteralKind, suffix_start: u32) -> Self {
        Self {
            value,
            kind,
            suffix_start,
        }
    }
}

/// <expr>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    /// <name>(<params>...)
    FnCall(Symbol, Span<Expr>),
    Value(Value),
}

impl From<Value> for Expr {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

/// A span of tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span<T> {
    from: u32,
    to: u32,
    kind: PhantomData<T>,
}

impl<T> Default for Span<T> {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            kind: PhantomData,
        }
    }
}

impl<T> Span<T> {
    #[must_use]
    pub fn push(self, token: T, set: &mut Vec<Token>) -> Self
    where
        T: Into<Token>,
    {
        set.push(token.into());
        Self {
            from: self.from,
            to: self.to + 1,
            kind: self.kind,
        }
    }
}
