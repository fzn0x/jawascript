//! The typed program produced by the type checker.
//!
//! Every expression carries its type, names are resolved to locals or
//! functions, implicit conversions are explicit nodes, and operators are
//! split by what they operate on (arithmetic, comparison, string concat).
//! The code generator walks this tree without re-checking anything.

use crate::source::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    /// `wilangan`: 64-bit signed integer.
    Int,
    /// `pecahan`: 64-bit IEEE float.
    Float,
    /// `logika`: boolean.
    Bool,
    /// `ukara`: immutable string.
    Str,
    /// `suwung`: no value.
    Void,
    /// `[T]`: growable array, a reference type.
    Array(Box<Ty>),
    /// A `wujud`, by name. Structs are reference types and may be `kosong`.
    Struct(String),
    /// The type of the `kosong` literal before it meets a struct type.
    Null,
    /// The type of an expression that failed to check. It is compatible with
    /// everything so one mistake produces one error.
    Error,
}

impl Ty {
    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }

    /// Whether `tulis` can print a value of this type.
    pub fn is_printable(&self) -> bool {
        match self {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Error => true,
            Ty::Array(element) => matches!(
                element.as_ref(),
                Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Error
            ),
            _ => false,
        }
    }

    pub fn is_reference(&self) -> bool {
        matches!(self, Ty::Array(_) | Ty::Struct(_) | Ty::Str)
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Int => f.write_str("wilangan"),
            Ty::Float => f.write_str("pecahan"),
            Ty::Bool => f.write_str("logika"),
            Ty::Str => f.write_str("ukara"),
            Ty::Void => f.write_str("suwung"),
            Ty::Array(element) => write!(f, "[{element}]"),
            Ty::Struct(name) => f.write_str(name),
            Ty::Null => f.write_str("kosong"),
            Ty::Error => f.write_str("{error}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub functions: Vec<Function>,
}

impl Program {
    pub fn struct_def(&self, name: &str) -> Option<&StructDef> {
        self.structs.iter().find(|s| s.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Clone, Debug)]
pub struct Local {
    pub name: String,
    pub ty: Ty,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<LocalId>,
    pub ret: Ty,
    /// Every local in the function, parameters first, indexed by [`LocalId`].
    pub locals: Vec<Local>,
    pub body: Block,
    pub span: Span,
}

impl Function {
    pub fn local(&self, id: LocalId) -> &Local {
        &self.locals[id.0 as usize]
    }
}

#[derive(Clone, Debug, Default)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Let {
        local: LocalId,
        init: Expr,
    },
    /// `place = value`, or `place op= value` when `op` is set. The place's
    /// parts are evaluated once.
    Assign {
        place: Place,
        op: Option<AssignOp>,
        value: Expr,
    },
    Expr(Expr),
    If {
        cond: Expr,
        then_block: Block,
        else_block: Option<Block>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    /// `kanggo var ing start..end`; `end` is evaluated once, before the loop.
    ForRange {
        var: LocalId,
        start: Expr,
        end: Expr,
        inclusive: bool,
        body: Block,
    },
    /// `kanggo var ing array`; the length is re-read on every iteration.
    ForEach {
        var: LocalId,
        array: Expr,
        body: Block,
    },
    /// `kanggo (init; cond; update)`. Without a condition the loop runs until
    /// `mandheg` or `bali`; `terusna` runs the update.
    ForC {
        init: Option<Box<Stmt>>,
        cond: Option<Expr>,
        update: Option<Box<Stmt>>,
        body: Block,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Block(Block),
}

#[derive(Clone, Debug)]
pub enum Place {
    Local(LocalId),
    Index { array: Expr, index: Expr },
    Field { object: Expr, field: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Arith(ArithOp),
    /// `ukara += …`
    Concat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastKind {
    IntToFloat,
    FloatToInt,
    BoolToInt,
    StrToInt,
    StrToFloat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Builtin {
    /// `tulis(…)`: prints its arguments separated by spaces, then a newline.
    Tulis,
    /// `dawane(x)`: the length of a string (in bytes) or an array.
    Dawane,
    /// `tambah(xs, x)`: appends to an array.
    Tambah,
    /// `gagal(pesen)`: stops the program with an error message.
    Gagal,
}

impl Builtin {
    pub fn from_name(name: &str) -> Option<Builtin> {
        Some(match name {
            "tulis" => Builtin::Tulis,
            "dawane" => Builtin::Dawane,
            "tambah" => Builtin::Tambah,
            "gagal" => Builtin::Gagal,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    /// `kosong`, typed as the struct it is used as.
    Null,
    Local(LocalId),
    /// Integer or float arithmetic, by the expression's type.
    Arith {
        op: ArithOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// A comparison of two operands of the same type.
    Compare {
        op: CmpOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `lan` (`and: true`) or `utawa`, short-circuiting.
    Logic {
        and: bool,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Neg(Box<Expr>),
    Not(Box<Expr>),
    /// `ukara + ukara`
    Concat(Box<Expr>, Box<Expr>),
    /// A number or boolean turned into text, explicitly with `ukara(x)` or
    /// implicitly by `"…" + x`.
    ToStr(Box<Expr>),
    Cast {
        kind: CastKind,
        operand: Box<Expr>,
    },
    Call {
        function: String,
        args: Vec<Expr>,
    },
    Builtin {
        builtin: Builtin,
        args: Vec<Expr>,
    },
    Array(Vec<Expr>),
    /// Field values in declaration order.
    StructLit {
        name: String,
        fields: Vec<Expr>,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    Field {
        object: Box<Expr>,
        field: usize,
    },
}
