//! A safe, typed wrapper around the C++ `ModuleBuilder`.
//!
//! The bridge passes plain `u32` handles; here they become [`Value`],
//! [`Slot`] and [`TypeId`], so a type can never be passed where a value is
//! expected.

use crate::bridge::ffi;
pub use crate::bridge::ffi::{BinaryOp, CastKind, Emit, Predicate};
use cxx::UniquePtr;

/// The C++ side's handle for "no value".
const NONE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Value(u32);

/// The address of a local variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeId(u32);

pub struct Builder {
    inner: UniquePtr<ffi::ModuleBuilder>,
}

macro_rules! value {
    ($call:expr) => {
        Value($call)
    };
}

impl Builder {
    pub fn new(file_name: &str) -> Builder {
        Builder {
            inner: ffi::new_module_builder(file_name),
        }
    }

    fn b(&mut self) -> std::pin::Pin<&mut ffi::ModuleBuilder> {
        self.inner.pin_mut()
    }

    fn handles<const N: usize>(values: [Value; N]) -> [u32; N] {
        values.map(|v| v.0)
    }

    fn list(values: &[Value]) -> Vec<u32> {
        values.iter().map(|v| v.0).collect()
    }

    // Types.
    pub fn int_type(&mut self) -> TypeId {
        TypeId(self.b().int_type())
    }
    pub fn float_type(&mut self) -> TypeId {
        TypeId(self.b().float_type())
    }
    pub fn bool_type(&mut self) -> TypeId {
        TypeId(self.b().bool_type())
    }
    pub fn str_type(&mut self) -> TypeId {
        TypeId(self.b().str_type())
    }
    pub fn array_type(&mut self, element: TypeId) -> TypeId {
        TypeId(self.b().array_type(element.0))
    }
    pub fn struct_type(&mut self, name: &str) -> TypeId {
        TypeId(self.b().struct_type(name))
    }

    // Structs and functions.
    pub fn declare_struct(&mut self, name: &str, fields: &[(&str, TypeId)]) {
        self.b().begin_struct(name);
        for (field, ty) in fields {
            self.b().add_field(field, ty.0);
        }
        self.b().end_struct();
    }
    pub fn declare_function(&mut self, name: &str, params: &[TypeId], result: Option<TypeId>) {
        let params: Vec<u32> = params.iter().map(|t| t.0).collect();
        self.b()
            .declare_function(name, &params, result.map_or(NONE, |t| t.0));
    }
    pub fn begin_function(&mut self, name: &str) {
        self.b().begin_function(name);
    }
    pub fn param(&mut self, index: usize) -> Value {
        value!(self.b().param(index as u32))
    }
    pub fn end_function(&mut self) {
        self.b().end_function();
    }
    pub fn set_location(&mut self, line: u32, column: u32) {
        self.b().set_location(line, column);
    }

    // Values.
    pub fn const_int(&mut self, value: i64) -> Value {
        value!(self.b().const_int(value))
    }
    pub fn const_float(&mut self, value: f64) -> Value {
        value!(self.b().const_float(value))
    }
    pub fn const_bool(&mut self, value: bool) -> Value {
        value!(self.b().const_bool(value))
    }
    pub fn const_str(&mut self, value: &str) -> Value {
        value!(self.b().const_str(value))
    }
    pub fn null(&mut self, ty: TypeId) -> Value {
        value!(self.b().null_value(ty.0))
    }
    pub fn binary(&mut self, op: BinaryOp, lhs: Value, rhs: Value) -> Value {
        let [lhs, rhs] = Self::handles([lhs, rhs]);
        value!(self.b().binary(op, lhs, rhs))
    }
    pub fn compare(&mut self, predicate: Predicate, lhs: Value, rhs: Value) -> Value {
        let [lhs, rhs] = Self::handles([lhs, rhs]);
        value!(self.b().compare(predicate, lhs, rhs))
    }
    pub fn negate(&mut self, value: Value) -> Value {
        value!(self.b().negate(value.0))
    }
    pub fn not(&mut self, value: Value) -> Value {
        value!(self.b().logical_not(value.0))
    }
    pub fn concat(&mut self, lhs: Value, rhs: Value) -> Value {
        value!(self.b().concat(lhs.0, rhs.0))
    }
    /// Formats a number or boolean as `ukara`.
    pub fn stringify(&mut self, value: Value) -> Value {
        value!(self.b().to_str(value.0))
    }
    pub fn cast(&mut self, kind: CastKind, value: Value) -> Value {
        value!(self.b().cast(kind, value.0))
    }
    pub fn str_len(&mut self, value: Value) -> Value {
        value!(self.b().str_len(value.0))
    }

    // Local variables.
    pub fn local(&mut self, ty: TypeId, name: &str) -> Slot {
        Slot(self.b().local(ty.0, name))
    }
    pub fn load(&mut self, slot: Slot) -> Value {
        value!(self.b().load(slot.0))
    }
    pub fn store(&mut self, value: Value, slot: Slot) {
        self.b().store(value.0, slot.0);
    }

    // Calls and built-in functions.
    /// Calls `callee`; `None` when it returns `suwung`.
    pub fn call(&mut self, callee: &str, args: &[Value]) -> Option<Value> {
        let result = self.b().call(callee, &Self::list(args));
        (result != NONE).then_some(Value(result))
    }
    pub fn print(&mut self, values: &[Value]) {
        self.b().print(&Self::list(values));
    }
    pub fn panic(&mut self, message: Value) {
        self.b().panic(message.0);
    }

    // Arrays and structs.
    pub fn array_new(&mut self, ty: TypeId, elements: &[Value]) -> Value {
        value!(self.b().array_new(ty.0, &Self::list(elements)))
    }
    pub fn array_len(&mut self, array: Value) -> Value {
        value!(self.b().array_len(array.0))
    }
    pub fn array_get(&mut self, array: Value, index: Value) -> Value {
        value!(self.b().array_get(array.0, index.0))
    }
    pub fn array_set(&mut self, array: Value, index: Value, value: Value) {
        self.b().array_set(array.0, index.0, value.0);
    }
    pub fn array_push(&mut self, array: Value, value: Value) {
        self.b().array_push(array.0, value.0);
    }
    pub fn struct_new(&mut self, ty: TypeId, fields: &[Value]) -> Value {
        value!(self.b().struct_new(ty.0, &Self::list(fields)))
    }
    pub fn get_field(&mut self, object: Value, index: usize) -> Value {
        value!(self.b().get_field(object.0, index as u32))
    }
    pub fn set_field(&mut self, object: Value, index: usize, value: Value) {
        self.b().set_field(object.0, index as u32, value.0);
    }

    // Structured control flow.
    pub fn begin_if(&mut self, condition: Value) {
        self.b().begin_if(condition.0);
    }
    pub fn begin_else(&mut self) {
        self.b().begin_else();
    }
    pub fn end_if(&mut self) {
        self.b().end_if();
    }
    pub fn begin_while(&mut self) {
        self.b().begin_while();
    }
    pub fn begin_for(&mut self) {
        self.b().begin_for();
    }
    pub fn begin_loop_body(&mut self, condition: Value) {
        self.b().begin_loop_body(condition.0);
    }
    pub fn begin_for_step(&mut self) {
        self.b().begin_for_step();
    }
    pub fn end_while(&mut self) {
        self.b().end_while();
    }
    pub fn end_for(&mut self) {
        self.b().end_for();
    }
    pub fn emit_break(&mut self) {
        self.b().emit_break();
    }
    pub fn emit_continue(&mut self) {
        self.b().emit_continue();
    }
    pub fn emit_return(&mut self, value: Option<Value>) {
        self.b().emit_return(value.map_or(NONE, |v| v.0));
    }
    pub fn is_terminated(&self) -> bool {
        self.inner.is_terminated()
    }

    // Output.
    pub fn verify(&mut self) -> Result<(), String> {
        if self.b().verify() {
            Ok(())
        } else {
            Err(self.inner.error_message())
        }
    }

    /// Writes `kind` to `path` (`-` for standard output).
    pub fn emit(&mut self, kind: Emit, opt_level: u32, path: &str) -> Result<(), String> {
        if self.b().emit(kind, opt_level, path) {
            Ok(())
        } else {
            Err(self.inner.error_message())
        }
    }
}
