//! The type checker: resolves names, infers and checks types, and lowers the
//! syntax tree to the typed [`hir`](crate::hir).
//!
//! Jawa is statically typed with local inference. A few conveniences are
//! resolved here so the code generator never sees them:
//!
//! - `ukara + x` (or `x + ukara`) converts a number or boolean `x` to text.
//! - An integer literal used where a `pecahan` is expected becomes a float,
//!   so `ayo x: pecahan = 1` and `x * 2` both work for a float `x`.
//! - `kosong` takes the struct type of whatever it is assigned to or
//!   compared with.

use crate::ast::{self, BinaryOp, ElseBranch, ForIter, TypeExprKind, UnaryOp};
use crate::diag::Diagnostic;
use crate::hir::*;
use crate::source::Span;
use std::collections::HashMap;

pub struct Checked {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn check(program: &ast::Program) -> Checked {
    let mut checker = Checker {
        structs: Vec::new(),
        struct_index: HashMap::new(),
        functions: HashMap::new(),
        diagnostics: Vec::new(),
        function_name: String::new(),
        locals: Vec::new(),
        scopes: Vec::new(),
        ret: Ty::Void,
        loop_depth: 0,
    };
    let program = checker.program(program);
    Checked {
        program,
        diagnostics: checker.diagnostics,
    }
}

struct Signature {
    params: Vec<Ty>,
    ret: Ty,
    span: Span,
}

struct Checker {
    structs: Vec<StructDef>,
    struct_index: HashMap<String, usize>,
    functions: HashMap<String, Signature>,
    diagnostics: Vec<Diagnostic>,

    // State for the function being checked.
    function_name: String,
    locals: Vec<Local>,
    scopes: Vec<HashMap<String, LocalId>>,
    ret: Ty,
    loop_depth: u32,
}

fn expr(kind: ExprKind, ty: Ty, span: Span) -> Expr {
    Expr { kind, ty, span }
}

fn error_expr(span: Span) -> Expr {
    expr(ExprKind::Int(0), Ty::Error, span)
}

fn assignable(target: &Ty, value: &Ty) -> bool {
    target == value
        || matches!(value, Ty::Error)
        || matches!(target, Ty::Error)
        || (matches!(value, Ty::Null) && matches!(target, Ty::Struct(_)))
}

impl Checker {
    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::error(span, message));
    }

    fn error_with_note(&mut self, span: Span, message: impl Into<String>, note: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::error(span, message).with_note(note));
    }

    // ---- Items ---------------------------------------------------------

    fn program(&mut self, program: &ast::Program) -> Program {
        let struct_decls: Vec<&ast::StructDecl> = program
            .items
            .iter()
            .filter_map(|item| match item {
                ast::Item::Struct(decl) => Some(decl),
                ast::Item::Function(_) => None,
            })
            .collect();
        let function_decls: Vec<&ast::FunctionDecl> = program
            .items
            .iter()
            .filter_map(|item| match item {
                ast::Item::Function(decl) => Some(decl),
                ast::Item::Struct(_) => None,
            })
            .collect();

        // Struct names first, so fields and signatures can refer to any of them.
        let mut declared_structs = Vec::new();
        for decl in &struct_decls {
            if self.struct_index.contains_key(&decl.name.name) {
                self.error(
                    decl.name.span,
                    format!("`{}` is already defined", decl.name.name),
                );
                continue;
            }
            self.struct_index
                .insert(decl.name.name.clone(), self.structs.len());
            self.structs.push(StructDef {
                name: decl.name.name.clone(),
                fields: Vec::new(),
                span: decl.span,
            });
            declared_structs.push(*decl);
        }
        for (index, decl) in declared_structs.iter().enumerate() {
            let mut fields: Vec<Field> = Vec::new();
            for field in &decl.fields {
                let ty = self.value_type(&field.ty);
                if fields.iter().any(|f| f.name == field.name.name) {
                    self.error(
                        field.name.span,
                        format!("field `{}` is declared twice", field.name.name),
                    );
                    continue;
                }
                fields.push(Field {
                    name: field.name.name.clone(),
                    ty,
                });
            }
            self.structs[index].fields = fields;
        }

        let mut declared_functions = Vec::new();
        for decl in &function_decls {
            let name = &decl.name.name;
            if Builtin::from_name(name).is_some() {
                self.error(
                    decl.name.span,
                    format!("`{name}` is a built-in function and cannot be redefined"),
                );
                continue;
            }
            if self.functions.contains_key(name) {
                self.error(decl.name.span, format!("`{name}` is already defined"));
                continue;
            }
            let params = decl.params.iter().map(|p| self.value_type(&p.ty)).collect();
            let ret = match &decl.ret {
                Some(ty) => self.resolve_type(ty),
                None => Ty::Void,
            };
            self.functions.insert(
                name.clone(),
                Signature {
                    params,
                    ret,
                    span: decl.name.span,
                },
            );
            declared_functions.push(*decl);
        }

        self.check_entry_point();

        let functions = declared_functions
            .into_iter()
            .map(|decl| self.function(decl))
            .collect();
        Program {
            structs: std::mem::take(&mut self.structs),
            functions,
        }
    }

    fn check_entry_point(&mut self) {
        match self.functions.get("utama") {
            None => self.error_with_note(
                Span::new(0, 0),
                "this program has no `utama` function",
                "programs start running at `gawe utama() { … }`",
            ),
            Some(signature) => {
                let span = signature.span;
                let has_params = !signature.params.is_empty();
                let returns = signature.ret != Ty::Void;
                if has_params {
                    self.error(span, "`utama` takes no parameters");
                }
                if returns {
                    self.error(span, "`utama` cannot return a value");
                }
            }
        }
    }

    fn resolve_type(&mut self, ty: &ast::TypeExpr) -> Ty {
        match &ty.kind {
            TypeExprKind::Wilangan => Ty::Int,
            TypeExprKind::Pecahan => Ty::Float,
            TypeExprKind::Logika => Ty::Bool,
            TypeExprKind::Ukara => Ty::Str,
            TypeExprKind::Suwung => Ty::Void,
            TypeExprKind::Array(element) => match self.value_type(element) {
                Ty::Error => Ty::Error,
                element => Ty::Array(Box::new(element)),
            },
            TypeExprKind::Named(name) => {
                if self.struct_index.contains_key(name) {
                    Ty::Struct(name.clone())
                } else {
                    let message = format!("unknown type `{name}`");
                    match suggest(name, self.struct_index.keys().map(String::as_str)) {
                        Some(similar) => self.error_with_note(
                            ty.span,
                            message,
                            format!("did you mean `{similar}`?"),
                        ),
                        None => self.error(ty.span, message),
                    }
                    Ty::Error
                }
            }
        }
    }

    /// A type for a variable, parameter, field or array element.
    fn value_type(&mut self, ty: &ast::TypeExpr) -> Ty {
        match self.resolve_type(ty) {
            Ty::Void => {
                self.error(ty.span, "`suwung` is not a type a value can have");
                Ty::Error
            }
            ty => ty,
        }
    }

    fn function(&mut self, decl: &ast::FunctionDecl) -> Function {
        let signature = &self.functions[&decl.name.name];
        let param_types = signature.params.clone();
        self.ret = signature.ret.clone();
        self.function_name = decl.name.name.clone();
        self.locals.clear();
        self.scopes = vec![HashMap::new()];
        self.loop_depth = 0;

        let mut params = Vec::new();
        for (param, ty) in decl.params.iter().zip(param_types) {
            if self.scopes[0].contains_key(&param.name.name) {
                self.error(
                    param.name.span,
                    format!("parameter `{}` is declared twice", param.name.name),
                );
            }
            params.push(self.declare(&param.name, ty, true));
        }

        let body = self.block(&decl.body);
        if self.ret != Ty::Void && !block_terminates(&body) {
            let end = Span::new(decl.body.span.end as usize - 1, decl.body.span.end as usize);
            self.error_with_note(
                end,
                format!(
                    "`{}` can reach its end without returning a `{}`",
                    decl.name.name, self.ret
                ),
                "add a `bali` with a value on every path",
            );
        }

        Function {
            name: decl.name.name.clone(),
            params,
            ret: self.ret.clone(),
            locals: std::mem::take(&mut self.locals),
            body,
            span: decl.span,
        }
    }

    // ---- Scopes --------------------------------------------------------

    fn declare(&mut self, name: &ast::Ident, ty: Ty, mutable: bool) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(Local {
            name: name.name.clone(),
            ty,
            mutable,
            span: name.span,
        });
        self.scopes
            .last_mut()
            .expect("a scope is open")
            .insert(name.name.clone(), id);
        id
    }

    fn lookup(&self, name: &str) -> Option<LocalId> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn visible_names(&self) -> Vec<String> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.keys().cloned())
            .collect()
    }

    // ---- Statements ----------------------------------------------------

    fn block(&mut self, block: &ast::Block) -> Block {
        self.scopes.push(HashMap::new());
        let mut stmts = Vec::new();
        let mut terminated = false;
        let mut warned = false;
        for stmt in &block.stmts {
            // Code after `bali` is still checked, but reported once.
            if terminated && !warned {
                self.diagnostics
                    .push(Diagnostic::warning(stmt.span, "unreachable code"));
                warned = true;
            }
            let checked = self.stmt(stmt);
            terminated |= stmt_terminates(&checked);
            stmts.push(checked);
        }
        self.scopes.pop();
        Block { stmts }
    }

    fn stmt(&mut self, stmt: &ast::Stmt) -> Stmt {
        let kind = match &stmt.kind {
            ast::StmtKind::Let {
                name,
                mutable,
                ty,
                init,
            } => self.let_stmt(name, *mutable, ty.as_ref(), init),
            ast::StmtKind::Assign { target, op, value } => self.assign(target, *op, value),
            ast::StmtKind::Expr(e) => StmtKind::Expr(self.expr(e, None)),
            ast::StmtKind::If {
                cond,
                then_block,
                else_branch,
            } => {
                let cond = self.check_against(cond, &Ty::Bool, "for the `yen` condition");
                let then_block = self.block(then_block);
                let else_block = else_branch.as_deref().map(|branch| match branch {
                    ElseBranch::Block(block) => self.block(block),
                    ElseBranch::If(stmt) => Block {
                        stmts: vec![self.stmt(stmt)],
                    },
                });
                StmtKind::If {
                    cond,
                    then_block,
                    else_block,
                }
            }
            ast::StmtKind::While { cond, body } => {
                let cond = self.check_against(cond, &Ty::Bool, "for the `sajrone` condition");
                let body = self.loop_body(body);
                StmtKind::While { cond, body }
            }
            ast::StmtKind::For { var, iter, body } => self.for_stmt(var, iter, body),
            ast::StmtKind::Return(value) => self.return_stmt(stmt.span, value.as_ref()),
            ast::StmtKind::Break | ast::StmtKind::Continue => {
                let is_break = matches!(stmt.kind, ast::StmtKind::Break);
                if self.loop_depth == 0 {
                    let word = if is_break { "mandheg" } else { "terusna" };
                    self.error(stmt.span, format!("`{word}` outside a loop"));
                }
                if is_break {
                    StmtKind::Break
                } else {
                    StmtKind::Continue
                }
            }
            ast::StmtKind::Block(block) => StmtKind::Block(self.block(block)),
        };
        Stmt {
            kind,
            span: stmt.span,
        }
    }

    fn loop_body(&mut self, body: &ast::Block) -> Block {
        self.loop_depth += 1;
        let body = self.block(body);
        self.loop_depth -= 1;
        body
    }

    fn let_stmt(
        &mut self,
        name: &ast::Ident,
        mutable: bool,
        ty: Option<&ast::TypeExpr>,
        init: &ast::Expr,
    ) -> StmtKind {
        let (init, ty) = match ty {
            Some(ty) => {
                let ty = self.value_type(ty);
                let init = self.check_against(init, &ty, &format!("for `{}`", name.name));
                (init, ty)
            }
            None => {
                let init = self.expr(init, None);
                let ty = match &init.ty {
                    Ty::Null => {
                        self.error_with_note(
                            init.span,
                            "cannot tell which struct this `kosong` is",
                            format!("add a type, such as `ayo {}: Simpul = kosong`", name.name),
                        );
                        Ty::Error
                    }
                    Ty::Void => {
                        self.error(init.span, "this call does not return a value");
                        Ty::Error
                    }
                    ty => ty.clone(),
                };
                (init, ty)
            }
        };
        let local = self.declare(name, ty, mutable);
        StmtKind::Let { local, init }
    }

    fn assign(&mut self, target: &ast::Expr, op: Option<BinaryOp>, value: &ast::Expr) -> StmtKind {
        let (place, place_ty) = self.place(target);
        let Some(op) = op else {
            let value = self.check_against(value, &place_ty, "for this assignment");
            return StmtKind::Assign {
                place,
                op: None,
                value,
            };
        };

        if place_ty == Ty::Str && op == BinaryOp::Add {
            let value = self.expr(value, None);
            let value = self.stringify(value, "+=");
            return StmtKind::Assign {
                place,
                op: Some(AssignOp::Concat),
                value,
            };
        }
        let value = self.expr(value, Some(&place_ty));
        let arith = arith_op(op).expect("compound assignment uses arithmetic");
        if !(place_ty.is_numeric() && value.ty == place_ty)
            && place_ty != Ty::Error
            && value.ty != Ty::Error
        {
            self.error(
                value.span,
                format!("cannot use `{op}=` with `{place_ty}` and `{}`", value.ty),
            );
        }
        StmtKind::Assign {
            place,
            op: Some(AssignOp::Arith(arith)),
            value,
        }
    }

    /// Checks an assignment target.
    fn place(&mut self, target: &ast::Expr) -> (Place, Ty) {
        match &target.kind {
            ast::ExprKind::Name(name) => match self.lookup(name) {
                Some(id) => {
                    let local = &self.locals[id.0 as usize];
                    let (ty, mutable) = (local.ty.clone(), local.mutable);
                    if !mutable {
                        self.error_with_note(
                            target.span,
                            format!("cannot assign to `{name}`, it does not change"),
                            "declare it with `ayo` instead of `tetep` to change it",
                        );
                    }
                    (Place::Local(id), ty)
                }
                None => {
                    self.unknown_variable(name, target.span);
                    (Place::Local(LocalId(u32::MAX)), Ty::Error)
                }
            },
            ast::ExprKind::Index { base, index } => {
                let (array, index, ty) = self.index_parts(base, index);
                (Place::Index { array, index }, ty)
            }
            ast::ExprKind::Field { base, field } => {
                let (object, field, ty) = self.field_parts(base, field);
                (Place::Field { object, field }, ty)
            }
            _ => unreachable!("the parser only accepts names, indexes and fields"),
        }
    }

    fn for_stmt(&mut self, var: &ast::Ident, iter: &ForIter, body: &ast::Block) -> StmtKind {
        match iter {
            ForIter::Range {
                start,
                end,
                inclusive,
            } => {
                let start = self.check_against(start, &Ty::Int, "for the loop range");
                let end = self.check_against(end, &Ty::Int, "for the loop range");
                self.scopes.push(HashMap::new());
                let var = self.declare(var, Ty::Int, false);
                let body = self.loop_body(body);
                self.scopes.pop();
                StmtKind::ForRange {
                    var,
                    start,
                    end,
                    inclusive: *inclusive,
                    body,
                }
            }
            ForIter::Each(array) => {
                let array = self.expr(array, None);
                let element = match &array.ty {
                    Ty::Array(element) => element.as_ref().clone(),
                    Ty::Error => Ty::Error,
                    other => {
                        let other = other.clone();
                        self.error_with_note(
                            array.span,
                            format!("cannot loop over `{other}`"),
                            "`kanggo` loops over a range such as `0..10`, or over an array",
                        );
                        Ty::Error
                    }
                };
                self.scopes.push(HashMap::new());
                let var = self.declare(var, element, false);
                let body = self.loop_body(body);
                self.scopes.pop();
                StmtKind::ForEach { var, array, body }
            }
        }
    }

    fn return_stmt(&mut self, span: Span, value: Option<&ast::Expr>) -> StmtKind {
        let ret = self.ret.clone();
        match (value, &ret) {
            (Some(value), Ty::Void) => {
                let checked = self.expr(value, None);
                self.error_with_note(
                    checked.span,
                    format!("`{}` does not return a value", self.function_name),
                    "declare a return type, such as `-> wilangan`, to return one",
                );
                StmtKind::Return(None)
            }
            (None, Ty::Void) => StmtKind::Return(None),
            (None, ret) => {
                self.error(span, format!("expected a `{ret}` value after `bali`"));
                StmtKind::Return(None)
            }
            (Some(value), ret) => {
                StmtKind::Return(Some(self.check_against(value, ret, "as the return value")))
            }
        }
    }

    // ---- Expressions ---------------------------------------------------

    /// Checks `e` and reports an error unless its type fits `target`.
    fn check_against(&mut self, e: &ast::Expr, target: &Ty, context: &str) -> Expr {
        let mut checked = self.expr(e, Some(target));
        if !assignable(target, &checked.ty) {
            self.error(
                checked.span,
                format!("expected `{target}` {context}, found `{}`", checked.ty),
            );
            checked.ty = Ty::Error;
        }
        checked
    }

    fn expr(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let mut checked = self.expr_inner(e, expected);
        if let Some(expected) = expected {
            coerce(&mut checked, expected);
        }
        checked
    }

    fn expr_inner(&mut self, e: &ast::Expr, expected: Option<&Ty>) -> Expr {
        let span = e.span;
        match &e.kind {
            ast::ExprKind::Int(value) => match i64::try_from(*value) {
                Ok(value) => expr(ExprKind::Int(value), Ty::Int, span),
                Err(_) => {
                    self.error(span, "number is too large for `wilangan`");
                    error_expr(span)
                }
            },
            ast::ExprKind::Float(value) => expr(ExprKind::Float(*value), Ty::Float, span),
            ast::ExprKind::Str(value) => expr(ExprKind::Str(value.clone()), Ty::Str, span),
            ast::ExprKind::Bool(value) => expr(ExprKind::Bool(*value), Ty::Bool, span),
            ast::ExprKind::Null => {
                let ty = match expected {
                    Some(ty @ Ty::Struct(_)) => ty.clone(),
                    _ => Ty::Null,
                };
                expr(ExprKind::Null, ty, span)
            }
            ast::ExprKind::Name(name) => match self.lookup(name) {
                Some(id) => {
                    let ty = self.locals[id.0 as usize].ty.clone();
                    expr(ExprKind::Local(id), ty, span)
                }
                None => {
                    if self.functions.contains_key(name) || Builtin::from_name(name).is_some() {
                        self.error_with_note(
                            span,
                            format!("`{name}` is a function, not a value"),
                            format!("call it: `{name}(…)`"),
                        );
                    } else {
                        self.unknown_variable(name, span);
                    }
                    error_expr(span)
                }
            },
            ast::ExprKind::Unary { op, operand } => self.unary(*op, operand, span),
            ast::ExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs, span),
            ast::ExprKind::Call { callee, args } => self.call(callee, args, span),
            ast::ExprKind::Convert { to, arg } => self.convert(to, arg, span),
            ast::ExprKind::Array(elements) => self.array(elements, expected, span),
            ast::ExprKind::StructLit { name, fields } => self.struct_lit(name, fields, span),
            ast::ExprKind::Index { base, index } => {
                let (array, index, ty) = self.index_parts(base, index);
                expr(
                    ExprKind::Index {
                        array: Box::new(array),
                        index: Box::new(index),
                    },
                    ty,
                    span,
                )
            }
            ast::ExprKind::Field { base, field } => {
                let (object, field, ty) = self.field_parts(base, field);
                expr(
                    ExprKind::Field {
                        object: Box::new(object),
                        field,
                    },
                    ty,
                    span,
                )
            }
        }
    }

    fn unknown_variable(&mut self, name: &str, span: Span) {
        let names = self.visible_names();
        let message = format!("unknown variable `{name}`");
        match suggest(name, names.iter().map(String::as_str)) {
            Some(similar) => {
                self.error_with_note(span, message, format!("did you mean `{similar}`?"))
            }
            None => self.error(span, message),
        }
    }

    fn unary(&mut self, op: UnaryOp, operand: &ast::Expr, span: Span) -> Expr {
        match op {
            UnaryOp::Neg => {
                // Fold negative literals, which also makes the most negative
                // `wilangan` writable: -9223372036854775808.
                match operand.kind {
                    ast::ExprKind::Int(value) if value <= 1 << 63 => {
                        return expr(ExprKind::Int((value as i64).wrapping_neg()), Ty::Int, span);
                    }
                    ast::ExprKind::Float(value) => {
                        return expr(ExprKind::Float(-value), Ty::Float, span);
                    }
                    _ => {}
                }
                let operand = self.expr(operand, None);
                if !operand.ty.is_numeric() && operand.ty != Ty::Error {
                    self.error(
                        operand.span,
                        format!("cannot negate `{}`, only numbers", operand.ty),
                    );
                    return error_expr(span);
                }
                let ty = operand.ty.clone();
                expr(ExprKind::Neg(Box::new(operand)), ty, span)
            }
            UnaryOp::Not => {
                let operand = self.check_against(operand, &Ty::Bool, "after `ora`");
                expr(ExprKind::Not(Box::new(operand)), Ty::Bool, span)
            }
        }
    }

    fn binary(&mut self, op: BinaryOp, lhs: &ast::Expr, rhs: &ast::Expr, span: Span) -> Expr {
        if op.is_logical() {
            let word = if op == BinaryOp::And { "lan" } else { "utawa" };
            let context = format!("for `{word}`");
            let lhs = self.check_against(lhs, &Ty::Bool, &context);
            let rhs = self.check_against(rhs, &Ty::Bool, &context);
            return expr(
                ExprKind::Logic {
                    and: op == BinaryOp::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                Ty::Bool,
                span,
            );
        }

        let mut lhs = self.expr(lhs, None);
        let mut rhs = self.expr(rhs, None);
        // Integer literals adapt to a float on the other side, and `kosong`
        // to a struct.
        coerce(&mut lhs, &rhs.ty.clone());
        coerce(&mut rhs, &lhs.ty.clone());
        let error = lhs.ty == Ty::Error || rhs.ty == Ty::Error;

        if op == BinaryOp::Add && (lhs.ty == Ty::Str || rhs.ty == Ty::Str) {
            let lhs = self.stringify(lhs, "+");
            let rhs = self.stringify(rhs, "+");
            return expr(
                ExprKind::Concat(Box::new(lhs), Box::new(rhs)),
                Ty::Str,
                span,
            );
        }

        if let Some(arith) = arith_op(op) {
            if error {
                return error_expr(span);
            }
            if lhs.ty.is_numeric() && lhs.ty == rhs.ty {
                let ty = lhs.ty.clone();
                return expr(
                    ExprKind::Arith {
                        op: arith,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    },
                    ty,
                    span,
                );
            }
            if lhs.ty.is_numeric() && rhs.ty.is_numeric() {
                self.error_with_note(
                    span,
                    format!("cannot use `{op}` on `{}` and `{}`", lhs.ty, rhs.ty),
                    "convert one side first, such as `pecahan(n)` or `wilangan(x)`",
                );
            } else {
                self.error(
                    span,
                    format!("cannot use `{op}` on `{}` and `{}`", lhs.ty, rhs.ty),
                );
            }
            return error_expr(span);
        }

        let cmp = cmp_op(op).expect("comparison operator");
        if error {
            return expr(ExprKind::Bool(false), Ty::Error, span);
        }
        let ordered = !matches!(cmp, CmpOp::Eq | CmpOp::Ne);
        let comparable = if ordered {
            lhs.ty == rhs.ty && matches!(lhs.ty, Ty::Int | Ty::Float | Ty::Str)
        } else {
            lhs.ty == rhs.ty && !matches!(lhs.ty, Ty::Void | Ty::Null)
        };
        if !comparable {
            if lhs.ty == Ty::Null && rhs.ty == Ty::Null {
                self.error(span, "comparing `kosong` with `kosong` is always the same");
            } else if lhs.ty == rhs.ty {
                self.error(span, format!("cannot use `{op}` on `{}`", lhs.ty));
            } else {
                self.error(
                    span,
                    format!("cannot compare `{}` with `{}`", lhs.ty, rhs.ty),
                );
            }
            return expr(ExprKind::Bool(false), Ty::Error, span);
        }
        expr(
            ExprKind::Compare {
                op: cmp,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            Ty::Bool,
            span,
        )
    }

    /// Converts a number or boolean to `ukara` for concatenation.
    fn stringify(&mut self, value: Expr, op: &str) -> Expr {
        match value.ty {
            Ty::Str | Ty::Error => value,
            Ty::Int | Ty::Float | Ty::Bool => {
                let span = value.span;
                expr(ExprKind::ToStr(Box::new(value)), Ty::Str, span)
            }
            ref other => {
                self.error(
                    value.span,
                    format!("cannot join `{other}` to text with `{op}`"),
                );
                error_expr(value.span)
            }
        }
    }

    fn call(&mut self, callee: &ast::Ident, args: &[ast::Expr], span: Span) -> Expr {
        let name = &callee.name;
        if let Some(signature) = self.functions.get(name) {
            let params = signature.params.clone();
            let ret = signature.ret.clone();
            if params.len() != args.len() {
                self.error(
                    span,
                    format!(
                        "`{name}` takes {} argument{}, but {} {} given",
                        params.len(),
                        if params.len() == 1 { "" } else { "s" },
                        args.len(),
                        if args.len() == 1 { "was" } else { "were" }
                    ),
                );
            }
            let args = args
                .iter()
                .enumerate()
                .map(|(i, arg)| match params.get(i) {
                    Some(param) => self.check_against(
                        arg,
                        param,
                        &format!("for argument {} of `{name}`", i + 1),
                    ),
                    None => self.expr(arg, None),
                })
                .collect();
            return expr(
                ExprKind::Call {
                    function: name.clone(),
                    args,
                },
                ret,
                span,
            );
        }
        if let Some(builtin) = Builtin::from_name(name) {
            return self.builtin(builtin, args, span);
        }
        if self.struct_index.contains_key(name) {
            self.error_with_note(
                callee.span,
                format!("`{name}` is a struct, not a function"),
                format!("build one with `{name} {{ field: value, … }}`"),
            );
        } else {
            let mut candidates: Vec<&str> = self.functions.keys().map(String::as_str).collect();
            candidates.extend(["tulis", "dawane", "tambah", "gagal"]);
            let message = format!("unknown function `{name}`");
            match suggest(name, candidates.into_iter()) {
                Some(similar) => {
                    self.error_with_note(callee.span, message, format!("did you mean `{similar}`?"))
                }
                None => self.error(callee.span, message),
            }
        }
        for arg in args {
            self.expr(arg, None);
        }
        error_expr(span)
    }

    fn builtin(&mut self, builtin: Builtin, args: &[ast::Expr], span: Span) -> Expr {
        let (name, arity) = match builtin {
            Builtin::Tulis => ("tulis", None),
            Builtin::Dawane => ("dawane", Some(1)),
            Builtin::Tambah => ("tambah", Some(2)),
            Builtin::Gagal => ("gagal", Some(1)),
        };
        if let Some(arity) = arity {
            if args.len() != arity {
                self.error(
                    span,
                    format!(
                        "`{name}` takes {arity} argument{}, but {} {} given",
                        if arity == 1 { "" } else { "s" },
                        args.len(),
                        if args.len() == 1 { "was" } else { "were" }
                    ),
                );
                for arg in args {
                    self.expr(arg, None);
                }
                return error_expr(span);
            }
        }

        let (args, ty) = match builtin {
            Builtin::Tulis => {
                let args: Vec<Expr> = args.iter().map(|arg| self.expr(arg, None)).collect();
                for arg in &args {
                    if !arg.ty.is_printable() {
                        let note = match &arg.ty {
                            Ty::Struct(_) => "print its fields instead, such as `tulis(p.x)`",
                            Ty::Array(_) => {
                                "only arrays of numbers, `logika` or `ukara` print directly"
                            }
                            _ => "`tulis` prints numbers, `logika`, `ukara` and arrays of them",
                        };
                        self.error_with_note(arg.span, format!("cannot print `{}`", arg.ty), note);
                    }
                }
                (args, Ty::Void)
            }
            Builtin::Dawane => {
                let arg = self.expr(&args[0], None);
                if !matches!(arg.ty, Ty::Str | Ty::Array(_) | Ty::Error) {
                    self.error(
                        arg.span,
                        format!("`dawane` measures `ukara` or arrays, not `{}`", arg.ty),
                    );
                }
                (vec![arg], Ty::Int)
            }
            Builtin::Tambah => {
                let array = self.expr(&args[0], None);
                let value = match &array.ty {
                    Ty::Array(element) => {
                        let element = element.as_ref().clone();
                        self.check_against(&args[1], &element, "for `tambah`")
                    }
                    Ty::Error => self.expr(&args[1], None),
                    other => {
                        let other = other.clone();
                        self.error(
                            array.span,
                            format!("`tambah` appends to an array, not `{other}`"),
                        );
                        self.expr(&args[1], None)
                    }
                };
                (vec![array, value], Ty::Void)
            }
            Builtin::Gagal => {
                let message = self.check_against(&args[0], &Ty::Str, "for `gagal`");
                (vec![message], Ty::Void)
            }
        };
        expr(ExprKind::Builtin { builtin, args }, ty, span)
    }

    fn convert(&mut self, to: &ast::TypeExpr, arg: &ast::Expr, span: Span) -> Expr {
        let arg = self.expr(arg, None);
        let target = self.resolve_type(to);
        if arg.ty == Ty::Error {
            return error_expr(span);
        }
        let cast = |kind: CastKind, operand: Expr, ty: Ty| {
            expr(
                ExprKind::Cast {
                    kind,
                    operand: Box::new(operand),
                },
                ty,
                span,
            )
        };
        match (&target, &arg.ty) {
            (target, ty) if target == ty => Expr { span, ..arg },
            (Ty::Int, Ty::Float) => cast(CastKind::FloatToInt, arg, Ty::Int),
            (Ty::Int, Ty::Bool) => cast(CastKind::BoolToInt, arg, Ty::Int),
            (Ty::Int, Ty::Str) => cast(CastKind::StrToInt, arg, Ty::Int),
            (Ty::Float, Ty::Int) => match arg.kind {
                ExprKind::Int(value) => expr(ExprKind::Float(value as f64), Ty::Float, span),
                _ => cast(CastKind::IntToFloat, arg, Ty::Float),
            },
            (Ty::Float, Ty::Str) => cast(CastKind::StrToFloat, arg, Ty::Float),
            (Ty::Str, Ty::Int | Ty::Float | Ty::Bool) => {
                expr(ExprKind::ToStr(Box::new(arg)), Ty::Str, span)
            }
            (Ty::Bool, Ty::Int) => {
                let zero = expr(ExprKind::Int(0), Ty::Int, arg.span);
                expr(
                    ExprKind::Compare {
                        op: CmpOp::Ne,
                        lhs: Box::new(arg),
                        rhs: Box::new(zero),
                    },
                    Ty::Bool,
                    span,
                )
            }
            (target, ty) => {
                self.error(span, format!("cannot convert `{ty}` to `{target}`"));
                error_expr(span)
            }
        }
    }

    fn array(&mut self, elements: &[ast::Expr], expected: Option<&Ty>, span: Span) -> Expr {
        let expected_element = match expected {
            Some(Ty::Array(element)) => Some(element.as_ref().clone()),
            _ => None,
        };
        let Some((first, rest)) = elements.split_first() else {
            return match expected_element {
                Some(element) => expr(
                    ExprKind::Array(Vec::new()),
                    Ty::Array(Box::new(element)),
                    span,
                ),
                None => {
                    self.error_with_note(
                        span,
                        "cannot tell the element type of an empty array",
                        "add a type, such as `ayo xs: [wilangan] = []`",
                    );
                    error_expr(span)
                }
            };
        };

        let first = match &expected_element {
            Some(element) => self.check_against(first, element, "for array elements"),
            None => self.expr(first, None),
        };
        let element = match expected_element {
            Some(element) => element,
            None => match &first.ty {
                Ty::Null => {
                    self.error_with_note(
                        first.span,
                        "cannot tell which struct this `kosong` is",
                        "add a type, such as `ayo xs: [Simpul] = [kosong]`",
                    );
                    Ty::Error
                }
                Ty::Void => {
                    self.error(first.span, "this call does not return a value");
                    Ty::Error
                }
                ty => ty.clone(),
            },
        };
        let mut checked = vec![first];
        for element_expr in rest {
            checked.push(self.check_against(element_expr, &element, "for array elements"));
        }
        if element == Ty::Error {
            return error_expr(span);
        }
        expr(ExprKind::Array(checked), Ty::Array(Box::new(element)), span)
    }

    fn struct_lit(
        &mut self,
        name: &ast::Ident,
        fields: &[(ast::Ident, ast::Expr)],
        span: Span,
    ) -> Expr {
        let Some(&index) = self.struct_index.get(&name.name) else {
            self.error(name.span, format!("unknown struct `{}`", name.name));
            for (_, value) in fields {
                self.expr(value, None);
            }
            return error_expr(span);
        };
        let def = self.structs[index].clone();
        let mut values: Vec<Option<Expr>> = vec![None; def.fields.len()];
        for (field, value) in fields {
            match def.fields.iter().position(|f| f.name == field.name) {
                Some(position) => {
                    let checked = self.check_against(
                        value,
                        &def.fields[position].ty,
                        &format!("for field `{}`", field.name),
                    );
                    if values[position].is_some() {
                        self.error(field.span, format!("field `{}` is set twice", field.name));
                    }
                    values[position] = Some(checked);
                }
                None => {
                    self.unknown_field(&def, field);
                    self.expr(value, None);
                }
            }
        }
        let missing: Vec<&str> = def
            .fields
            .iter()
            .zip(&values)
            .filter(|(_, value)| value.is_none())
            .map(|(field, _)| field.name.as_str())
            .collect();
        if !missing.is_empty() {
            let list = missing
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            self.error(
                span,
                format!(
                    "missing field{} {list} in `{}`",
                    if missing.len() == 1 { "" } else { "s" },
                    def.name
                ),
            );
            return error_expr(span);
        }
        expr(
            ExprKind::StructLit {
                name: def.name.clone(),
                fields: values.into_iter().map(Option::unwrap).collect(),
            },
            Ty::Struct(def.name),
            span,
        )
    }

    fn unknown_field(&mut self, def: &StructDef, field: &ast::Ident) {
        let message = format!("`{}` has no field `{}`", def.name, field.name);
        match suggest(&field.name, def.fields.iter().map(|f| f.name.as_str())) {
            Some(similar) => {
                self.error_with_note(field.span, message, format!("did you mean `{similar}`?"))
            }
            None => self.error(field.span, message),
        }
    }

    fn index_parts(&mut self, base: &ast::Expr, index: &ast::Expr) -> (Expr, Expr, Ty) {
        let array = self.expr(base, None);
        let index = self.check_against(index, &Ty::Int, "as an index");
        let ty = match &array.ty {
            Ty::Array(element) => element.as_ref().clone(),
            Ty::Error => Ty::Error,
            other => {
                let other = other.clone();
                self.error(array.span, format!("cannot index into `{other}`"));
                Ty::Error
            }
        };
        (array, index, ty)
    }

    fn field_parts(&mut self, base: &ast::Expr, field: &ast::Ident) -> (Expr, usize, Ty) {
        let object = self.expr(base, None);
        let def = match &object.ty {
            Ty::Struct(name) => self.structs[self.struct_index[name]].clone(),
            Ty::Error => return (object, 0, Ty::Error),
            other => {
                let other = other.clone();
                if matches!(other, Ty::Array(_) | Ty::Str) {
                    self.error_with_note(
                        field.span,
                        format!("`{other}` has no fields"),
                        "for the length, use `dawane(x)`",
                    );
                } else {
                    self.error(field.span, format!("`{other}` has no fields"));
                }
                return (object, 0, Ty::Error);
            }
        };
        match def.fields.iter().position(|f| f.name == field.name) {
            Some(index) => {
                let ty = def.fields[index].ty.clone();
                (object, index, ty)
            }
            None => {
                self.unknown_field(&def, field);
                (object, 0, Ty::Error)
            }
        }
    }
}

/// Adapts literals to the type they meet: an integer literal to `pecahan`,
/// and `kosong` to a struct.
fn coerce(value: &mut Expr, target: &Ty) {
    match (&value.kind, target) {
        (ExprKind::Int(n), Ty::Float) => {
            value.kind = ExprKind::Float(*n as f64);
            value.ty = Ty::Float;
        }
        (ExprKind::Null, Ty::Struct(_)) if value.ty == Ty::Null => {
            value.ty = target.clone();
        }
        _ => {}
    }
}

fn arith_op(op: BinaryOp) -> Option<ArithOp> {
    Some(match op {
        BinaryOp::Add => ArithOp::Add,
        BinaryOp::Sub => ArithOp::Sub,
        BinaryOp::Mul => ArithOp::Mul,
        BinaryOp::Div => ArithOp::Div,
        BinaryOp::Rem => ArithOp::Rem,
        _ => return None,
    })
}

fn cmp_op(op: BinaryOp) -> Option<CmpOp> {
    Some(match op {
        BinaryOp::Eq => CmpOp::Eq,
        BinaryOp::Ne => CmpOp::Ne,
        BinaryOp::Lt => CmpOp::Lt,
        BinaryOp::Le => CmpOp::Le,
        BinaryOp::Gt => CmpOp::Gt,
        BinaryOp::Ge => CmpOp::Ge,
        _ => return None,
    })
}

/// Whether control never continues past `stmt`: it returns, stops the
/// program, leaves the loop, or loops forever.
pub fn stmt_terminates(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) | StmtKind::Break | StmtKind::Continue => true,
        StmtKind::Expr(e) => matches!(
            e.kind,
            ExprKind::Builtin {
                builtin: Builtin::Gagal,
                ..
            }
        ),
        StmtKind::If {
            then_block,
            else_block: Some(else_block),
            ..
        } => block_terminates(then_block) && block_terminates(else_block),
        StmtKind::Block(block) => block_terminates(block),
        StmtKind::While { cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks_out(body)
        }
        _ => false,
    }
}

pub fn block_terminates(block: &Block) -> bool {
    block.stmts.iter().any(stmt_terminates)
}

/// Whether `body` contains a `mandheg` for the loop that owns it.
fn breaks_out(body: &Block) -> bool {
    body.stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break => true,
        StmtKind::If {
            then_block,
            else_block,
            ..
        } => breaks_out(then_block) || else_block.as_ref().is_some_and(breaks_out),
        StmtKind::Block(block) => breaks_out(block),
        _ => false,
    })
}

/// The closest candidate to a misspelled `name`, if any is close enough.
fn suggest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (name.chars().count() / 3).max(1);
    candidates
        .filter(|candidate| *candidate != name)
        .map(|candidate| (edit_distance(name, candidate), candidate))
        .filter(|(distance, _)| *distance <= limit)
        .min()
        .map(|(_, candidate)| candidate)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let current = row[j + 1];
            row[j + 1] = if ca == cb {
                previous
            } else {
                1 + previous.min(row[j]).min(row[j + 1])
            };
            previous = current;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn check_text(text: &str) -> Checked {
        let parsed = parse(text);
        assert!(
            parsed.diagnostics.is_empty(),
            "parse errors: {:#?}",
            parsed.diagnostics
        );
        check(&parsed.program)
    }

    fn messages(text: &str) -> Vec<String> {
        check_text(text)
            .diagnostics
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    fn ok(text: &str) -> Program {
        let checked = check_text(text);
        let errors: Vec<_> = checked
            .diagnostics
            .iter()
            .filter(|d| d.is_error())
            .collect();
        assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
        checked.program
    }

    /// Wraps statements in an entry point.
    fn in_utama(body: &str) -> String {
        format!("gawe utama() {{\n{body}\n}}\n")
    }

    #[test]
    fn checks_a_complete_program() {
        let program = ok("
            wujud Titik { x: wilangan, y: wilangan }
            gawe fib(n: wilangan) -> wilangan {
                yen n < 2 { bali n }
                bali fib(n - 1) + fib(n - 2)
            }
            gawe utama() {
                ayo xs = [1, 2, 3]
                tambah(xs, fib(10))
                ayo p = Titik { y: 2, x: 1 }
                p.x += 5
                kanggo i ing 0..dawane(xs) { tulis(xs[i], p.x) }
                kanggo x ing xs { yen x == 2 { terusna } }
            }
        ");
        assert_eq!(program.functions.len(), 2);
        let utama = &program.functions[1];
        assert_eq!(utama.local(LocalId(0)).ty, Ty::Array(Box::new(Ty::Int)));
        assert_eq!(utama.local(LocalId(1)).ty, Ty::Struct("Titik".into()));
    }

    #[test]
    fn struct_literal_fields_follow_declaration_order() {
        let program = ok(&format!(
            "wujud Titik {{ x: wilangan, y: ukara }}\n{}",
            in_utama("ayo p = Titik { y: \"loro\", x: 1 }")
        ));
        let StmtKind::Let { init, .. } = &program.functions[0].body.stmts[0].kind else {
            panic!()
        };
        let ExprKind::StructLit { fields, .. } = &init.kind else {
            panic!()
        };
        assert!(matches!(fields[0].kind, ExprKind::Int(1)));
        assert!(matches!(fields[1].kind, ExprKind::Str(_)));
    }

    #[test]
    fn text_plus_value_converts_the_value() {
        let program = ok(&in_utama("ayo s = \"umur: \" + 20 + \", bener: \" + bener"));
        let StmtKind::Let { init, .. } = &program.functions[0].body.stmts[0].kind else {
            panic!()
        };
        assert_eq!(init.ty, Ty::Str);
        let ExprKind::Concat(lhs, rhs) = &init.kind else {
            panic!()
        };
        assert!(matches!(rhs.kind, ExprKind::ToStr(_)));
        assert!(matches!(lhs.kind, ExprKind::Concat(..)));
    }

    #[test]
    fn integer_literals_adapt_to_floats() {
        let program = ok(&in_utama(
            "ayo x: pecahan = 1\nayo y = x * 2\nayo zs: [pecahan] = [1, 2.5]",
        ));
        let stmts = &program.functions[0].body.stmts;
        let StmtKind::Let { init, .. } = &stmts[1].kind else {
            panic!()
        };
        assert_eq!(init.ty, Ty::Float);
        let ExprKind::Arith { rhs, .. } = &init.kind else {
            panic!()
        };
        assert!(matches!(rhs.kind, ExprKind::Float(v) if v == 2.0));
    }

    #[test]
    fn mixing_number_types_needs_a_conversion() {
        let checked = check_text(&in_utama("ayo a = 1\nayo b = 2.5\ntulis(a + b)"));
        assert_eq!(
            checked.diagnostics[0].message,
            "cannot use `+` on `wilangan` and `pecahan`"
        );
        assert_eq!(
            checked.diagnostics[0].notes,
            ["convert one side first, such as `pecahan(n)` or `wilangan(x)`"]
        );
    }

    #[test]
    fn kosong_takes_the_struct_type() {
        ok("
            wujud Simpul { regane: wilangan, sabanjure: Simpul }
            gawe utama() {
                ayo n = Simpul { regane: 1, sabanjure: kosong }
                ayo m: Simpul = kosong
                yen n.sabanjure == kosong { m = n }
            }
        ");
    }

    #[test]
    fn reports_type_mismatches() {
        assert_eq!(
            messages(&in_utama("ayo x: wilangan = \"siji\"")),
            ["expected `wilangan` for `x`, found `ukara`"]
        );
        assert_eq!(
            messages(&in_utama("yen 1 { }")),
            ["expected `logika` for the `yen` condition, found `wilangan`"]
        );
        assert_eq!(
            messages("gawe f(n: wilangan) { }\ngawe utama() { f(bener) }"),
            ["expected `wilangan` for argument 1 of `f`, found `logika`"]
        );
    }

    #[test]
    fn suggests_similar_names() {
        let checked = check_text(&in_utama("ayo jeneng = \"Jawa\"\ntulis(jenneg)"));
        assert_eq!(checked.diagnostics[0].message, "unknown variable `jenneg`");
        assert_eq!(checked.diagnostics[0].notes, ["did you mean `jeneng`?"]);
        let checked = check_text(&in_utama("tuliss(1)"));
        assert_eq!(checked.diagnostics[0].notes, ["did you mean `tulis`?"]);
    }

    #[test]
    fn tetep_and_loop_variables_do_not_change() {
        assert_eq!(
            messages(&in_utama("tetep x = 1\nx = 2")),
            ["cannot assign to `x`, it does not change"]
        );
        assert_eq!(
            messages(&in_utama("kanggo i ing 0..3 { i = 5 }")),
            ["cannot assign to `i`, it does not change"]
        );
    }

    #[test]
    fn missing_return_is_an_error() {
        assert_eq!(
            messages("gawe f(n: wilangan) -> wilangan { yen n > 0 { bali 1 } }\ngawe utama() {}"),
            ["`f` can reach its end without returning a `wilangan`"]
        );
        // Both branches returning, or an endless loop, is fine.
        ok(
            "gawe f(n: wilangan) -> wilangan { yen n > 0 { bali 1 } liyane { bali 2 } }
            gawe g() -> wilangan { sajrone bener { bali 1 } }
            gawe h() -> wilangan { gagal(\"durung\") }
            gawe utama() {}",
        );
        // A loop that can break can fall through.
        assert_eq!(
            messages("gawe g() -> wilangan { sajrone bener { mandheg } }\ngawe utama() {}"),
            ["`g` can reach its end without returning a `wilangan`"]
        );
    }

    #[test]
    fn break_outside_a_loop() {
        assert_eq!(messages(&in_utama("mandheg")), ["`mandheg` outside a loop"]);
    }

    #[test]
    fn empty_arrays_need_a_type() {
        assert_eq!(
            messages(&in_utama("ayo xs = []")),
            ["cannot tell the element type of an empty array"]
        );
        ok(&in_utama("ayo xs: [wilangan] = []\ntambah(xs, 1)"));
    }

    #[test]
    fn requires_an_entry_point() {
        assert_eq!(
            messages("gawe f() {}"),
            ["this program has no `utama` function"]
        );
        assert_eq!(
            messages("gawe utama(x: wilangan) -> wilangan { bali x }"),
            [
                "`utama` takes no parameters",
                "`utama` cannot return a value"
            ]
        );
    }

    #[test]
    fn struct_literals_need_every_field() {
        assert_eq!(
            messages(&format!(
                "wujud Titik {{ x: wilangan, y: wilangan }}\n{}",
                in_utama("ayo p = Titik { x: 1, z: 2 }")
            )),
            ["`Titik` has no field `z`", "missing field `y` in `Titik`"]
        );
    }

    #[test]
    fn cannot_print_structs() {
        assert_eq!(
            messages(&format!(
                "wujud Titik {{ x: wilangan }}\n{}",
                in_utama("tulis(Titik { x: 1 })")
            )),
            ["cannot print `Titik`"]
        );
    }

    #[test]
    fn conversions() {
        ok(&in_utama(
            "ayo a = wilangan(2.9)\nayo b = pecahan(a)\nayo c = ukara(b)\nayo d = wilangan(\"42\")\nayo e = logika(a)",
        ));
        assert_eq!(
            messages(&in_utama("ayo xs = [1]\nayo s = ukara(xs)")),
            ["cannot convert `[wilangan]` to `ukara`"]
        );
    }

    #[test]
    fn warns_about_unreachable_code() {
        let checked = check_text(&in_utama("bali\ntulis(1)"));
        assert_eq!(checked.diagnostics.len(), 1);
        assert!(!checked.diagnostics[0].is_error());
        assert_eq!(checked.diagnostics[0].message, "unreachable code");
    }

    #[test]
    fn most_negative_integer_literal() {
        let program = ok(&in_utama("ayo x = -9223372036854775808"));
        let StmtKind::Let { init, .. } = &program.functions[0].body.stmts[0].kind else {
            panic!()
        };
        assert!(matches!(init.kind, ExprKind::Int(i64::MIN)));
    }

    #[test]
    fn edit_distance_suggestions() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(
            suggest("tuls", ["tulis", "tambah"].into_iter()),
            Some("tulis")
        );
        assert_eq!(suggest("x", ["tulis"].into_iter()), None);
    }
}
