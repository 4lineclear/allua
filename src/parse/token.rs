// NOTE:
// is run automatically when imported, unless `defer` is used

use std::marker::PhantomData;

use crate::util::Symbol;

/// a module of code
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module {
    /// First item must be a fn
    items: Vec<Token>,
}

impl Module {
    pub fn new(name: &str) -> Self {
        let function = Fn {
            name: name.into(),
            params: Span::default(),
            tokens: Span::default(),
        };
        Self {
            items: vec![function.into()],
        }
    }

    pub fn push(&mut self, token: impl Into<Token>) {
        self.items.push(token.into());
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
    ty: DeclType,
    name: Symbol,
    value: Option<Value>,
}

// TODO: add let style const decl

/// <type> | `let` | `const` | `const` <type>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclType {
    /// let <name>
    Let,
    /// <type> <name>
    Type(Symbol),
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
pub enum Value {
    // "..."
    String(Symbol),
}

/// <expr>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    /// <name>(<params>...)
    FnCall { name: Symbol, params: Span<Value> },
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

impl<'a, T> Span<T> {
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
