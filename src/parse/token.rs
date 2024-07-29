// NOTE:
// is run automatically when imported, unless `defer` is used

use std::marker::PhantomData;

/// a module of code
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module<'a> {
    /// First item must be a fn
    items: Vec<Token<'a>>,
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
pub struct Fn<'a> {
    name: &'a str,
    params: Span<Decl<'a>>,
    tokens: Span<Token<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token<'a> {
    Fn(Fn<'a>),
    Decl(Decl<'a>),
    Expr(Expr<'a>),
    Value(Value<'a>),
    Import(Import<'a>),
}

macro_rules! token_from {
    ($($name:ident),*) => {$(
        impl<'a> From<$name<'a>> for Token<'a> {
            fn from(value: $name<'a>) -> Self {
                Self::$name(value)
            }
        }
    )*};
}

token_from!(Fn, Decl, Expr, Value, Import);

/// [`DeclType`] <name> ?(= <value>);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decl<'a> {
    /// if none try implicit
    ty: DeclType<'a>,
    name: &'a str,
    value: Option<Value<'a>>,
}

// TODO: add let style const decl

/// <type> | `let` | `const` | `const` <type>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclType<'a> {
    /// let <name>
    Let,
    /// <type> <name>
    Type(&'a str),
    /// const <type> <name>
    ConstType(&'a str),
}

/// ?(<defer>) `use` <name>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import<'a> {
    name: &'a str,
    defer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value<'a> {
    // "..."
    String(&'a str),
}

/// <expr>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr<'a> {
    /// <name>(<params>...)
    FnCall {
        name: &'a str,
        params: Span<Value<'a>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span<T> {
    start: usize,
    end: usize,
    kind: PhantomData<T>,
}

impl<'a, T> Span<T> {
    #[must_use]
    pub fn push(self, token: T, set: &mut Vec<Token<'a>>) -> Self
    where
        T: Into<Token<'a>>,
    {
        set.push(token.into());
        Self {
            start: self.start,
            end: self.end + 1,
            kind: self.kind,
        }
    }
}
