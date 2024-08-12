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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token {
    Flow(Flow),
    FnDef(FnDef),
    Decl(Decl),
    // NOTE: never have expr be under another token, instead refer to a span, etc
    Expr(Expr),
    Return,
    Value(Value),
    Import(Import),
    Block(TSpan),
    FnDefParam(FnDefParam),
    /// A dummy token. should never appear in the final output.
    Dummy,
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

token_from!(FnDef, Decl, Expr, Value, Import, FnDefParam, Flow);

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

/// <name>(<params>) | <var> | <value>
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    /// <name>(<params>)
    FnCall(Symbol, TSpan),
    /// <name>
    Var(Symbol),
    /// constant value
    Value(Value),
}

impl From<Value> for Expr {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}

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
