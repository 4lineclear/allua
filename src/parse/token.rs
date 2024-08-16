use crate::span::TSpan;
use crate::{lex, util::Symbol};

/// a module of code
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Module {
    name: Symbol,
    /// First item must be a fn
    pub(crate) items: Vec<Token>,
}

impl Module {
    #[must_use]
    pub fn new(name: &str, items: Vec<Token>) -> Self {
        Self {
            name: name.into(),
            items,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, token: impl Into<Token>) {
        self.items.push(token.into());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token {
    Flow(Flow),
    FnDef(FnDef),
    Decl(Decl),
    // NOTE: never have expr be under another token, instead refer to a span, etc
    Expr(Expr),
    Return,
    // Value(Value),
    Import(Import),
    Block(TSpan),
    FnDefParam(FnDefParam),
    /// A dummy token. should never appear in the final output.
    Dummy,
}

/// A user defined function
///
/// Acts as both as a module, datatype and function
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnDef {
    pub name: Symbol,
    pub type_name: Option<Symbol>,
    pub params: TSpan,
    pub tokens: TSpan,
}

/// <type > <name> ?(= <value>);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnDefParam {
    pub type_name: Symbol,
    pub name: Symbol,
    pub value: bool,
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

token_from!(FnDef, Decl, Expr, Import, FnDefParam, Flow);

impl From<ExprKind> for Token {
    fn from(value: ExprKind) -> Self {
        Self::Expr(Expr::from(value))
    }
}

/// Control flow
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Flow {
    If(TSpan, Option<TSpan>),
    // While(TSpan),
}

/// [`DeclKind`] <name> ?(= <value>);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decl {
    pub kind: DeclKind,
    pub type_name: Option<Symbol>,
    pub name: Symbol,
    pub value: bool,
}

/// <type> | `let` | `const` | `const` <type>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclKind {
    /// let <name>
    Let,
    /// const <name>
    Const,
}

/// ?(<defer>) `use` <name>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import {
    name: Symbol,
    defer: bool,
}

// NOTE: consider turnin span to just a usize denoting the end

/// <name>(<params>) | <var> | <value>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expr {
    /// The total end of the expression
    pub end: usize,
    pub kind: ExprKind,
}

impl From<ExprKind> for Expr {
    fn from(value: ExprKind) -> Self {
        Self {
            end: 0,
            kind: value,
        }
    }
}

/// <name>(<params>) | <var> | <value>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExprKind {
    /// <name>(<params>)
    FnCall(FnCall),
    /// <name>
    Var(Symbol),
    /// constant value
    Value(Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FnCall {
    pub name: Symbol,
    /// true if ended with comma
    pub comma: bool,
    // /// true if call has been closed
    // pub closed: bool,
}

macro_rules! impl_from {
    ($($ty:ident),*) => { $(
        impl From<$ty> for ExprKind {
            fn from(value: $ty) -> Self {
                Self::$ty(value)
            }
        }
        impl From<$ty> for Expr {
            fn from(value: $ty) -> Self {
                Self {
                    end: 0,
                    kind: ExprKind::from(value)
                }
            }
        }
        // impl From<$ty> for Token {
        //     fn from(value: $ty) -> Self {
        //         Token::from(Expr::from(value))
        //     }
        // }
    )*};
}

impl_from!(Value, FnCall);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Value {
    pub value: Symbol,
    pub kind: lex::LiteralKind,
    pub suffix_start: usize,
}

impl Value {
    #[must_use]
    pub const fn new(value: Symbol, kind: lex::LiteralKind, suffix_start: usize) -> Self {
        Self {
            value,
            kind,
            suffix_start,
        }
    }
}
