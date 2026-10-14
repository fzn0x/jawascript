//! Lowers the typed program to the `jawa` MLIR dialect through the bridge.
//!
//! Every local variable gets a stack slot (`jawa.alloca`) and is read and
//! written with `jawa.load`/`jawa.store`; mem2reg turns them into SSA values
//! later. Control flow keeps its structure (`jawa.if`, `jawa.while`,
//! `jawa.for`) for the backend to flatten.

use crate::builder::{BinaryOp, Builder, CastKind, Predicate, Slot, TypeId, Value};
use jawa_frontend::hir::{
    ArithOp, AssignOp, Block, Builtin, CmpOp, Expr, ExprKind, Function, LocalId, Place, Program,
    Stmt, StmtKind, Ty,
};
use jawa_frontend::source::{SourceFile, Span};
use std::collections::HashMap;

pub fn generate(program: &Program, file: &SourceFile) -> Builder {
    let mut builder = Builder::new(file.name());
    let mut types = Types::default();

    for def in &program.structs {
        let fields: Vec<(&str, TypeId)> = def
            .fields
            .iter()
            .map(|field| (field.name.as_str(), types.get(&mut builder, &field.ty)))
            .collect();
        builder.declare_struct(&def.name, &fields);
    }
    for function in &program.functions {
        let params: Vec<TypeId> = function
            .params
            .iter()
            .map(|&param| types.get(&mut builder, &function.local(param).ty))
            .collect();
        let result = (function.ret != Ty::Void).then(|| types.get(&mut builder, &function.ret));
        builder.declare_function(&function.name, &params, result);
    }
    for function in &program.functions {
        FunctionGen {
            b: &mut builder,
            types: &mut types,
            file,
            function,
            slots: HashMap::new(),
        }
        .emit();
    }
    builder
}

/// Caches the bridge's handle for each type.
#[derive(Default)]
struct Types {
    cache: HashMap<Ty, TypeId>,
}

impl Types {
    fn get(&mut self, b: &mut Builder, ty: &Ty) -> TypeId {
        if let Some(&id) = self.cache.get(ty) {
            return id;
        }
        let id = match ty {
            Ty::Int => b.int_type(),
            Ty::Float => b.float_type(),
            Ty::Bool => b.bool_type(),
            Ty::Str => b.str_type(),
            Ty::Array(element) => {
                let element = self.get(b, element);
                b.array_type(element)
            }
            Ty::Struct(name) => b.struct_type(name),
            Ty::Void | Ty::Null | Ty::Error => {
                unreachable!("the type checker never lets `{ty}` reach code generation")
            }
        };
        self.cache.insert(ty.clone(), id);
        id
    }
}

struct FunctionGen<'a> {
    b: &'a mut Builder,
    types: &'a mut Types,
    file: &'a SourceFile,
    function: &'a Function,
    slots: HashMap<LocalId, Slot>,
}

impl FunctionGen<'_> {
    fn emit(mut self) {
        let function = self.function;
        self.b.begin_function(&function.name);
        self.locate(function.span);
        for (index, &param) in function.params.iter().enumerate() {
            let slot = self.slot(param);
            let value = self.b.param(index);
            self.b.store(value, slot);
        }
        self.block(&function.body);
        self.b.end_function();
    }

    fn locate(&mut self, span: Span) {
        let position = self.file.line_col(span.start);
        self.b.set_location(position.line, position.col);
    }

    fn ty(&mut self, ty: &Ty) -> TypeId {
        self.types.get(self.b, ty)
    }

    /// The stack slot of a local, made on first use.
    fn slot(&mut self, id: LocalId) -> Slot {
        if let Some(&slot) = self.slots.get(&id) {
            return slot;
        }
        let function = self.function;
        let local = function.local(id);
        let ty = self.ty(&local.ty);
        let slot = self.b.local(ty, &local.name);
        self.slots.insert(id, slot);
        slot
    }

    /// A slot for a value the source never names, such as a loop index.
    fn hidden_slot(&mut self, ty: &Ty, name: &str) -> Slot {
        let ty = self.ty(ty);
        self.b.local(ty, name)
    }

    // ---- Statements ----------------------------------------------------

    fn block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            // Code after `bali`, `mandheg` or `terusna` never runs.
            if self.b.is_terminated() {
                break;
            }
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt) {
        self.locate(stmt.span);
        match &stmt.kind {
            StmtKind::Let { local, init } => {
                let value = self.value(init);
                let slot = self.slot(*local);
                self.b.store(value, slot);
            }
            StmtKind::Assign { place, op, value } => self.assign(place, *op, value),
            StmtKind::Expr(expr) => {
                self.expr(expr);
            }
            StmtKind::If {
                cond,
                then_block,
                else_block,
            } => {
                let cond = self.value(cond);
                self.b.begin_if(cond);
                self.block(then_block);
                if let Some(else_block) = else_block {
                    self.b.begin_else();
                    self.block(else_block);
                }
                self.b.end_if();
            }
            StmtKind::While { cond, body } => {
                self.b.begin_while();
                let cond = self.value(cond);
                self.b.begin_loop_body(cond);
                self.block(body);
                self.b.end_while();
            }
            StmtKind::ForRange {
                var,
                start,
                end,
                inclusive,
                body,
            } => {
                let start = self.value(start);
                let end = self.value(end);
                let slot = self.slot(*var);
                self.b.store(start, slot);

                self.b.begin_for();
                let current = self.b.load(slot);
                let predicate = if *inclusive {
                    Predicate::Le
                } else {
                    Predicate::Lt
                };
                let cond = self.b.compare(predicate, current, end);
                self.b.begin_loop_body(cond);
                self.block(body);
                self.b.begin_for_step();
                self.increment(slot);
                self.b.end_for();
            }
            StmtKind::ForEach { var, array, body } => {
                let array = self.value(array);
                let index = self.hidden_slot(&Ty::Int, "indeks");
                let zero = self.b.const_int(0);
                self.b.store(zero, index);

                self.b.begin_for();
                let current = self.b.load(index);
                // Re-read the length: the body may append to the array.
                let len = self.b.array_len(array);
                let cond = self.b.compare(Predicate::Lt, current, len);
                self.b.begin_loop_body(cond);
                let current = self.b.load(index);
                let element = self.b.array_get(array, current);
                let slot = self.slot(*var);
                self.b.store(element, slot);
                self.block(body);
                self.b.begin_for_step();
                self.increment(index);
                self.b.end_for();
            }
            StmtKind::Return(value) => {
                let value = value.as_ref().map(|value| self.value(value));
                self.b.emit_return(value);
            }
            StmtKind::Break => self.b.emit_break(),
            StmtKind::Continue => self.b.emit_continue(),
            StmtKind::Block(block) => self.block(block),
        }
    }

    fn increment(&mut self, slot: Slot) {
        let current = self.b.load(slot);
        let one = self.b.const_int(1);
        let next = self.b.binary(BinaryOp::Add, current, one);
        self.b.store(next, slot);
    }

    fn assign(&mut self, place: &Place, op: Option<AssignOp>, value: &Expr) {
        match place {
            Place::Local(id) => {
                let slot = self.slot(*id);
                let new = match op {
                    None => self.value(value),
                    Some(op) => {
                        let current = self.b.load(slot);
                        let rhs = self.value(value);
                        self.combine(op, current, rhs)
                    }
                };
                self.b.store(new, slot);
            }
            Place::Index { array, index } => {
                let array = self.value(array);
                let index = self.value(index);
                let new = match op {
                    None => self.value(value),
                    Some(op) => {
                        let current = self.b.array_get(array, index);
                        let rhs = self.value(value);
                        self.combine(op, current, rhs)
                    }
                };
                self.b.array_set(array, index, new);
            }
            Place::Field { object, field } => {
                let object = self.value(object);
                let new = match op {
                    None => self.value(value),
                    Some(op) => {
                        let current = self.b.get_field(object, *field);
                        let rhs = self.value(value);
                        self.combine(op, current, rhs)
                    }
                };
                self.b.set_field(object, *field, new);
            }
        }
    }

    fn combine(&mut self, op: AssignOp, current: Value, rhs: Value) -> Value {
        match op {
            AssignOp::Arith(op) => self.b.binary(binary_op(op), current, rhs),
            AssignOp::Concat => self.b.concat(current, rhs),
        }
    }

    // ---- Expressions ---------------------------------------------------

    /// An expression that produces a value.
    fn value(&mut self, expr: &Expr) -> Value {
        self.expr(expr)
            .expect("the type checker only allows valued expressions here")
    }

    /// Any expression; `None` for calls that return `suwung`.
    fn expr(&mut self, expr: &Expr) -> Option<Value> {
        self.locate(expr.span);
        let value = match &expr.kind {
            ExprKind::Int(value) => self.b.const_int(*value),
            ExprKind::Float(value) => self.b.const_float(*value),
            ExprKind::Bool(value) => self.b.const_bool(*value),
            ExprKind::Str(value) => self.b.const_str(value),
            ExprKind::Null => {
                let ty = self.ty(&expr.ty);
                self.b.null(ty)
            }
            ExprKind::Local(id) => {
                let slot = self.slot(*id);
                self.b.load(slot)
            }
            ExprKind::Arith { op, lhs, rhs } => {
                let lhs = self.value(lhs);
                let rhs = self.value(rhs);
                self.b.binary(binary_op(*op), lhs, rhs)
            }
            ExprKind::Compare { op, lhs, rhs } => {
                let lhs = self.value(lhs);
                let rhs = self.value(rhs);
                self.b.compare(predicate(*op), lhs, rhs)
            }
            ExprKind::Logic { and, lhs, rhs } => self.short_circuit(*and, lhs, rhs),
            ExprKind::Neg(operand) => {
                let operand = self.value(operand);
                self.b.negate(operand)
            }
            ExprKind::Not(operand) => {
                let operand = self.value(operand);
                self.b.not(operand)
            }
            ExprKind::Concat(lhs, rhs) => {
                let lhs = self.value(lhs);
                let rhs = self.value(rhs);
                self.b.concat(lhs, rhs)
            }
            ExprKind::ToStr(operand) => {
                let operand = self.value(operand);
                self.b.stringify(operand)
            }
            ExprKind::Cast { kind, operand } => {
                let operand = self.value(operand);
                self.b.cast(cast_kind(*kind), operand)
            }
            ExprKind::Call { function, args } => {
                let args: Vec<Value> = args.iter().map(|arg| self.value(arg)).collect();
                return self.b.call(function, &args);
            }
            ExprKind::Builtin { builtin, args } => return self.builtin(*builtin, args),
            ExprKind::Array(elements) => {
                let elements: Vec<Value> = elements.iter().map(|e| self.value(e)).collect();
                let ty = self.ty(&expr.ty);
                self.b.array_new(ty, &elements)
            }
            ExprKind::StructLit { fields, .. } => {
                let fields: Vec<Value> = fields.iter().map(|f| self.value(f)).collect();
                let ty = self.ty(&expr.ty);
                self.b.struct_new(ty, &fields)
            }
            ExprKind::Index { array, index } => {
                let array = self.value(array);
                let index = self.value(index);
                self.b.array_get(array, index)
            }
            ExprKind::Field { object, field } => {
                let object = self.value(object);
                self.b.get_field(object, *field)
            }
        };
        Some(value)
    }

    /// `a lan b` and `a utawa b` evaluate `b` only when needed, through a
    /// temporary slot that mem2reg later turns into a block argument.
    fn short_circuit(&mut self, and: bool, lhs: &Expr, rhs: &Expr) -> Value {
        let result = self.hidden_slot(&Ty::Bool, if and { "lan" } else { "utawa" });
        let lhs = self.value(lhs);
        self.b.store(lhs, result);
        let need_rhs = if and { lhs } else { self.b.not(lhs) };
        self.b.begin_if(need_rhs);
        let rhs = self.value(rhs);
        self.b.store(rhs, result);
        self.b.end_if();
        self.b.load(result)
    }

    fn builtin(&mut self, builtin: Builtin, args: &[Expr]) -> Option<Value> {
        match builtin {
            Builtin::Tulis => {
                let values: Vec<Value> = args.iter().map(|arg| self.value(arg)).collect();
                self.b.print(&values);
                None
            }
            Builtin::Dawane => {
                let value = self.value(&args[0]);
                Some(if args[0].ty == Ty::Str {
                    self.b.str_len(value)
                } else {
                    self.b.array_len(value)
                })
            }
            Builtin::Tambah => {
                let array = self.value(&args[0]);
                let value = self.value(&args[1]);
                self.b.array_push(array, value);
                None
            }
            Builtin::Gagal => {
                let message = self.value(&args[0]);
                self.b.panic(message);
                None
            }
        }
    }
}

fn binary_op(op: ArithOp) -> BinaryOp {
    match op {
        ArithOp::Add => BinaryOp::Add,
        ArithOp::Sub => BinaryOp::Sub,
        ArithOp::Mul => BinaryOp::Mul,
        ArithOp::Div => BinaryOp::Div,
        ArithOp::Rem => BinaryOp::Rem,
    }
}

fn predicate(op: CmpOp) -> Predicate {
    match op {
        CmpOp::Eq => Predicate::Eq,
        CmpOp::Ne => Predicate::Ne,
        CmpOp::Lt => Predicate::Lt,
        CmpOp::Le => Predicate::Le,
        CmpOp::Gt => Predicate::Gt,
        CmpOp::Ge => Predicate::Ge,
    }
}

fn cast_kind(kind: jawa_frontend::hir::CastKind) -> CastKind {
    use jawa_frontend::hir::CastKind as K;
    match kind {
        K::IntToFloat => CastKind::IntToFloat,
        K::FloatToInt => CastKind::FloatToInt,
        K::BoolToInt => CastKind::BoolToInt,
        K::StrToInt => CastKind::StrToInt,
        K::StrToFloat => CastKind::StrToFloat,
    }
}
