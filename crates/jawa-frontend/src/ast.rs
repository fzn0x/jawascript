//! The untyped syntax tree produced by the parser.

use crate::source::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// `gawe name(param: type, …) -> type { … }`
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// `wujud Name { field: type, … }`
#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    pub name: Ident,
    pub ty: TypeExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TypeExprKind {
    Wilangan,
    Pecahan,
    Logika,
    Ukara,
    Suwung,
    /// `[T]`
    Array(Box<TypeExpr>),
    /// A struct name.
    Named(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StmtKind {
    /// `ayo name: type = init` or `tetep name = init`
    Let {
        name: Ident,
        mutable: bool,
        ty: Option<TypeExpr>,
        init: Expr,
    },
    /// `target = value`, or `target += value` when `op` is set.
    Assign {
        target: Expr,
        op: Option<BinaryOp>,
        value: Expr,
    },
    Expr(Expr),
    /// `yen cond { … } liyane { … }`
    If {
        cond: Expr,
        then_block: Block,
        else_branch: Option<Box<ElseBranch>>,
    },
    /// `sajrone cond { … }`
    While {
        cond: Expr,
        body: Block,
    },
    /// `kanggo var ing iter { … }`, or `kanggo (tetep var ing iter) { … }`
    For {
        var: Ident,
        iter: ForIter,
        body: Block,
    },
    /// The JavaScript-style loop `kanggo (init; cond; update) { … }`. Each
    /// part may be left out; without a condition the loop runs until
    /// `mandheg` or `bali`.
    ForC {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Box<Stmt>>,
        body: Block,
    },
    /// `bali value`
    Return(Option<Expr>),
    /// `mandheg`
    Break,
    /// `terusna`
    Continue,
    Block(Block),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ElseBranch {
    Block(Block),
    /// `liyane yen …`
    If(Stmt),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForIter {
    /// `start..end` or `start..=end`
    Range {
        start: Expr,
        end: Expr,
        inclusive: bool,
    },
    /// Every element of an array.
    Each(Expr),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    Int(u64),
    Float(f64),
    Str(String),
    /// `` `text ${expr} text` ``
    Template(Vec<TemplatePart>),
    Bool(bool),
    /// `kosong`
    Null,
    Name(String),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        callee: Ident,
        args: Vec<Expr>,
    },
    /// A type keyword used as a conversion, such as `ukara(42)`.
    Convert {
        to: TypeExpr,
        arg: Box<Expr>,
    },
    /// `[a, b, c]`
    Array(Vec<Expr>),
    /// `Name { field: value, … }`
    StructLit {
        name: Ident,
        fields: Vec<(Ident, Expr)>,
    },
    /// `base[index]`
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// `base.field`
    Field {
        base: Box<Expr>,
        field: Ident,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePart {
    Text(String),
    Expr(Expr),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-x`
    Neg,
    /// `ora x`
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// `lan`
    And,
    /// `utawa`
    Or,
}

impl BinaryOp {
    pub fn is_arithmetic(self) -> bool {
        matches!(
            self,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
        )
    }

    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        )
    }

    pub fn is_logical(self) -> bool {
        matches!(self, BinaryOp::And | BinaryOp::Or)
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "lan",
            BinaryOp::Or => "utawa",
        })
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnaryOp::Neg => "-",
            UnaryOp::Not => "ora",
        })
    }
}
