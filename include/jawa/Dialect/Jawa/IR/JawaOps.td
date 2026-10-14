//===- JawaOps.td - Jawa dialect operations ----------------*- tablegen -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_IR_JAWAOPS_TD
#define JAWA_DIALECT_JAWA_IR_JAWAOPS_TD

include "jawa/Dialect/Jawa/IR/JawaDialect.td"
include "jawa/Dialect/Jawa/IR/JawaTypes.td"
include "mlir/IR/BuiltinAttributes.td"
include "mlir/IR/CommonAttrConstraints.td"
include "mlir/IR/EnumAttr.td"
include "mlir/IR/SymbolInterfaces.td"
include "mlir/Interfaces/InferTypeOpInterface.td"
include "mlir/Interfaces/MemorySlotInterfaces.td"
include "mlir/Interfaces/SideEffectInterfaces.td"

//===----------------------------------------------------------------------===//
// Enums
//===----------------------------------------------------------------------===//

def Jawa_CmpPredicateAttr : I64EnumAttr<"CmpPredicate", "comparison predicate", [
  I64EnumAttrCase<"eq", 0>,
  I64EnumAttrCase<"ne", 1>,
  I64EnumAttrCase<"lt", 2>,
  I64EnumAttrCase<"le", 3>,
  I64EnumAttrCase<"gt", 4>,
  I64EnumAttrCase<"ge", 5>,
]> {
  let cppNamespace = "::mlir::jawa";
}

def Jawa_CastKindAttr : I64EnumAttr<"CastKind", "conversion kind", [
  I64EnumAttrCase<"int_to_float", 0>,
  I64EnumAttrCase<"float_to_int", 1>,
  I64EnumAttrCase<"bool_to_int", 2>,
  I64EnumAttrCase<"str_to_int", 3>,
  I64EnumAttrCase<"str_to_float", 4>,
]> {
  let cppNamespace = "::mlir::jawa";
}

//===----------------------------------------------------------------------===//
// Constants
//===----------------------------------------------------------------------===//

def Jawa_ConstantOp : Jawa_Op<"const",
    [ConstantLike, Pure, AllTypesMatch<["value", "result"]>]> {
  let summary = "an integer, float or boolean constant";
  let arguments = (ins TypedAttrInterface:$value);
  let results = (outs Jawa_ScalarType:$result);
  let assemblyFormat = "attr-dict $value";
  let hasFolder = 1;
  let hasVerifier = 1;
}

def Jawa_StrConstOp : Jawa_Op<"str_const", [ConstantLike, Pure]> {
  let summary = "a string literal";
  let arguments = (ins StrAttr:$value);
  let results = (outs Jawa_StrType:$result);
  let assemblyFormat = "$value attr-dict";
  let builders = [
    OpBuilder<(ins "::llvm::StringRef":$value), [{
      build($_builder, $_state, StrType::get($_builder.getContext()),
            $_builder.getStringAttr(value));
    }]>,
  ];
  let hasFolder = 1;
}

def Jawa_NullOp : Jawa_Op<"null", [Pure]> {
  let summary = "the null reference (`kosong`)";
  let results = (outs Jawa_RefType:$result);
  let assemblyFormat = "attr-dict `:` type($result)";
}

//===----------------------------------------------------------------------===//
// Arithmetic and logic
//===----------------------------------------------------------------------===//

class Jawa_BinaryOp<string mnemonic, list<Trait> traits = []>
    : Jawa_Op<mnemonic, !listconcat(traits, [SameOperandsAndResultType])> {
  let arguments = (ins Jawa_NumericType:$lhs, Jawa_NumericType:$rhs);
  let results = (outs Jawa_NumericType:$result);
  let assemblyFormat = "$lhs `,` $rhs attr-dict `:` type($result)";
  let hasFolder = 1;
}

def Jawa_AddOp : Jawa_BinaryOp<"add", [Pure, Commutative]> {
  let summary = "addition; integers wrap on overflow";
}
def Jawa_SubOp : Jawa_BinaryOp<"sub", [Pure]> {
  let summary = "subtraction; integers wrap on overflow";
}
def Jawa_MulOp : Jawa_BinaryOp<"mul", [Pure, Commutative]> {
  let summary = "multiplication; integers wrap on overflow";
}
def Jawa_DivOp : Jawa_BinaryOp<"div",
    [DeclareOpInterfaceMethods<MemoryEffectsOpInterface>]> {
  let summary = "division, truncating toward zero for integers";
  let description = [{
    Integer division by zero stops the program with an error, so unless the
    divisor is a non-zero constant the operation is treated as having a side
    effect and is never removed. `MIN / -1` wraps to `MIN` instead of being
    undefined.
  }];
}
def Jawa_RemOp : Jawa_BinaryOp<"rem",
    [DeclareOpInterfaceMethods<MemoryEffectsOpInterface>]> {
  let summary = "remainder, with the sign of the dividend";
  let description = [{
    Like `jawa.div`, integer remainder by zero stops the program.
  }];
}

def Jawa_NegOp : Jawa_Op<"neg", [Pure, SameOperandsAndResultType]> {
  let summary = "numeric negation";
  let arguments = (ins Jawa_NumericType:$operand);
  let results = (outs Jawa_NumericType:$result);
  let assemblyFormat = "$operand attr-dict `:` type($result)";
  let hasFolder = 1;
}

def Jawa_NotOp : Jawa_Op<"not", [Pure, SameOperandsAndResultType]> {
  let summary = "logical negation (`ora`)";
  let arguments = (ins I1:$operand);
  let results = (outs I1:$result);
  let assemblyFormat = "$operand attr-dict";
  let hasFolder = 1;
}

def Jawa_CmpOp : Jawa_Op<"cmp", [Pure, SameTypeOperands]> {
  let summary = "comparison";
  let description = [{
    Numbers and strings support every predicate (strings compare by bytes).
    Booleans, arrays and structs support `eq` and `ne`; for arrays and
    structs that is identity.
  }];
  let arguments = (ins Jawa_CmpPredicateAttr:$predicate,
                       Jawa_ValueType:$lhs, Jawa_ValueType:$rhs);
  let results = (outs I1:$result);
  let assemblyFormat = "$predicate `,` $lhs `,` $rhs attr-dict `:` type($lhs)";
  let hasFolder = 1;
  let hasVerifier = 1;
}

//===----------------------------------------------------------------------===//
// Strings and conversions
//===----------------------------------------------------------------------===//

def Jawa_ConcatOp : Jawa_Op<"concat", [Pure]> {
  let summary = "joins strings";
  let description = [{
    Joins any number of strings into a new one with a single allocation.
    The frontend emits it with two operands; `jawa-fuse-concat` merges
    chains such as `a + b + c` into one n-ary concat.
  }];
  let arguments = (ins Variadic<Jawa_StrType>:$parts);
  let results = (outs Jawa_StrType:$result);
  let assemblyFormat = "$parts attr-dict";
  let builders = [
    OpBuilder<(ins "::mlir::ValueRange":$parts), [{
      build($_builder, $_state, StrType::get($_builder.getContext()), parts);
    }]>,
  ];
  let hasFolder = 1;
  let hasVerifier = 1;
}

def Jawa_ToStrOp : Jawa_Op<"to_str", [Pure]> {
  let summary = "formats a number or boolean as a string";
  let arguments = (ins Jawa_ScalarType:$value);
  let results = (outs Jawa_StrType:$result);
  let assemblyFormat = "$value attr-dict `:` type($value)";
  let builders = [
    OpBuilder<(ins "::mlir::Value":$value), [{
      build($_builder, $_state, StrType::get($_builder.getContext()), value);
    }]>,
  ];
  let hasFolder = 1;
}

def Jawa_CastOp : Jawa_Op<"cast",
    [DeclareOpInterfaceMethods<MemoryEffectsOpInterface>]> {
  let summary = "converts between numbers, booleans and strings";
  let description = [{
    `float_to_int` truncates and saturates (NaN becomes 0). Parsing a string
    (`str_to_int`, `str_to_float`) stops the program if the text is not a
    number, so those kinds have a side effect.
  }];
  let arguments = (ins Jawa_CastKindAttr:$kind, Jawa_ValueType:$value);
  let results = (outs Jawa_ScalarType:$result);
  let assemblyFormat =
      "$kind $value attr-dict `:` type($value) `->` type($result)";
  let hasFolder = 1;
  let hasVerifier = 1;
}

def Jawa_StrLenOp : Jawa_Op<"str_len", [Pure]> {
  let summary = "the length of a string, in bytes";
  let arguments = (ins Jawa_StrType:$str);
  let results = (outs I64:$result);
  let assemblyFormat = "$str attr-dict";
  let hasFolder = 1;
}

//===----------------------------------------------------------------------===//
// Arrays
//===----------------------------------------------------------------------===//

def Jawa_ArrayNewOp : Jawa_Op<"array_new", [MemoryEffects<[MemAlloc]>]> {
  let summary = "allocates an array holding the given elements";
  let arguments = (ins Variadic<Jawa_ValueType>:$elements);
  let results = (outs Jawa_ArrayType:$result);
  let assemblyFormat =
      "`(` $elements `)` attr-dict `:` functional-type($elements, $result)";
  let hasVerifier = 1;
}

def Jawa_ArrayLenOp : Jawa_Op<"array_len", [MemoryEffects<[MemRead]>]> {
  let summary = "the number of elements in an array";
  let arguments = (ins Jawa_ArrayType:$array);
  let results = (outs I64:$result);
  let assemblyFormat = "$array attr-dict `:` qualified(type($array))";
}

def Jawa_ArrayGetOp : Jawa_Op<"array_get",
    [MemoryEffects<[MemRead, MemWrite]>]> {
  let summary = "reads an element, stopping the program if out of bounds";
  let arguments = (ins Jawa_ArrayType:$array, I64:$index);
  let results = (outs Jawa_ValueType:$result);
  let assemblyFormat =
      "$array `[` $index `]` attr-dict `:` qualified(type($array)) `->` type($result)";
  let hasVerifier = 1;
}

def Jawa_ArraySetOp : Jawa_Op<"array_set",
    [MemoryEffects<[MemRead, MemWrite]>]> {
  let summary = "writes an element, stopping the program if out of bounds";
  let arguments = (ins Jawa_ArrayType:$array, I64:$index,
                       Jawa_ValueType:$value);
  let assemblyFormat =
      "$array `[` $index `]` `,` $value attr-dict `:` qualified(type($array)) `,` "
      "type($value)";
  let hasVerifier = 1;
}

def Jawa_ArrayPushOp : Jawa_Op<"array_push",
    [MemoryEffects<[MemRead, MemWrite]>]> {
  let summary = "appends an element (`tambah`)";
  let arguments = (ins Jawa_ArrayType:$array, Jawa_ValueType:$value);
  let assemblyFormat =
      "$array `,` $value attr-dict `:` qualified(type($array)) `,` type($value)";
  let hasVerifier = 1;
}

//===----------------------------------------------------------------------===//
// Structs
//===----------------------------------------------------------------------===//

def Jawa_StructDefOp : Jawa_Op<"struct",
    [Symbol, HasParent<"::mlir::ModuleOp">]> {
  let summary = "declares the fields of a struct";
  let description = [{
    ```mlir
    jawa.struct @Titik {x: i64, y: i64}
    ```
  }];
  let arguments = (ins SymbolNameAttr:$sym_name, StrArrayAttr:$field_names,
                       TypeArrayAttr:$field_types);
  let hasCustomAssemblyFormat = 1;
  let hasVerifier = 1;
  let extraClassDeclaration = [{
    /// The type of the field at `index`.
    ::mlir::Type getFieldType(unsigned index) {
      return ::llvm::cast<::mlir::TypeAttr>(getFieldTypes()[index]).getValue();
    }
    unsigned getNumFields() { return getFieldTypes().size(); }
  }];
}

def Jawa_StructNewOp : Jawa_Op<"struct_new", [
    MemoryEffects<[MemAlloc]>,
    DeclareOpInterfaceMethods<SymbolUserOpInterface>]> {
  let summary = "allocates a struct with the given field values";
  let arguments = (ins Variadic<Jawa_ValueType>:$fields);
  let results = (outs Jawa_StructType:$result);
  let assemblyFormat =
      "`(` $fields `)` attr-dict `:` functional-type($fields, $result)";
}

def Jawa_GetFieldOp : Jawa_Op<"get_field", [
    MemoryEffects<[MemRead, MemWrite]>,
    DeclareOpInterfaceMethods<SymbolUserOpInterface>]> {
  let summary = "reads a field, stopping the program if the struct is null";
  let arguments = (ins Jawa_StructType:$object, I64Attr:$index);
  let results = (outs Jawa_ValueType:$result);
  let assemblyFormat =
      "$object `[` $index `]` attr-dict `:` qualified(type($object)) `->` type($result)";
}

def Jawa_SetFieldOp : Jawa_Op<"set_field", [
    MemoryEffects<[MemRead, MemWrite]>,
    DeclareOpInterfaceMethods<SymbolUserOpInterface>]> {
  let summary = "writes a field, stopping the program if the struct is null";
  let arguments = (ins Jawa_StructType:$object, I64Attr:$index,
                       Jawa_ValueType:$value);
  let assemblyFormat =
      "$object `[` $index `]` `,` $value attr-dict `:` qualified(type($object)) `,` "
      "type($value)";
}

//===----------------------------------------------------------------------===//
// Local variables
//===----------------------------------------------------------------------===//

def Jawa_AllocaOp : Jawa_Op<"alloca", [
    DeclareOpInterfaceMethods<PromotableAllocationOpInterface>,
    MemoryEffects<[MemAlloc<AutomaticAllocationScopeResource>]>]> {
  let summary = "a stack slot for a local variable";
  let description = [{
    The frontend gives every variable a slot and reads and writes it with
    `jawa.load` and `jawa.store`. The ops implement the memory slot
    interfaces, so MLIR's `mem2reg` pass promotes the slots to SSA values.
  }];
  let arguments = (ins StrAttr:$name);
  let results = (outs Jawa_PtrType:$addr);
  let assemblyFormat = "$name attr-dict `:` qualified(type($addr))";
}

def Jawa_LoadOp : Jawa_Op<"load", [
    DeclareOpInterfaceMethods<PromotableMemOpInterface>,
    TypesMatchWith<"the result matches the slot's type", "addr", "result",
                   "::llvm::cast<PtrType>($_self).getPointee()">]> {
  let summary = "reads a local variable";
  let arguments = (ins Arg<Jawa_PtrType, "the slot", [MemRead]>:$addr);
  let results = (outs Jawa_ValueType:$result);
  let assemblyFormat = "$addr attr-dict `:` qualified(type($addr))";
}

def Jawa_StoreOp : Jawa_Op<"store", [
    DeclareOpInterfaceMethods<PromotableMemOpInterface>,
    TypesMatchWith<"the value matches the slot's type", "addr", "value",
                   "::llvm::cast<PtrType>($_self).getPointee()">]> {
  let summary = "writes a local variable";
  let arguments = (ins Jawa_ValueType:$value,
                       Arg<Jawa_PtrType, "the slot", [MemWrite]>:$addr);
  let assemblyFormat = "$value `,` $addr attr-dict `:` qualified(type($addr))";
}

//===----------------------------------------------------------------------===//
// Built-in functions
//===----------------------------------------------------------------------===//

def Jawa_PrintOp : Jawa_Op<"print", [MemoryEffects<[MemWrite]>]> {
  let summary = "prints values separated by spaces, then a newline (`tulis`)";
  let arguments = (ins Variadic<Jawa_ValueType>:$values);
  let assemblyFormat = "($values^ `:` type($values))? attr-dict";
  let hasVerifier = 1;
}

def Jawa_PanicOp : Jawa_Op<"panic", [MemoryEffects<[MemWrite]>]> {
  let summary = "stops the program with a message (`gagal`)";
  let arguments = (ins Jawa_StrType:$message);
  let assemblyFormat = "$message attr-dict";
}

//===----------------------------------------------------------------------===//
// Structured control flow
//===----------------------------------------------------------------------===//

def Jawa_IfOp : Jawa_Op<"if", [RecursiveMemoryEffects, NoRegionArguments]> {
  let summary = "`yen` with an optional `liyane`";
  let description = [{
    Each region ends in `jawa.yield` to continue after the `if`, or in
    `jawa.break`, `jawa.continue`, `jawa.return` or `jawa.unreachable`.

    ```mlir
    jawa.if %cond {
      jawa.yield
    } else {
      jawa.return
    }
    ```
  }];
  let arguments = (ins I1:$condition);
  let regions = (region MaxSizedRegion<1>:$thenRegion,
                        MaxSizedRegion<1>:$elseRegion);
  let assemblyFormat = "$condition $thenRegion (`else` $elseRegion^)? attr-dict";
  let hasVerifier = 1;
}

def Jawa_WhileOp : Jawa_Op<"while", [RecursiveMemoryEffects, NoRegionArguments]> {
  let summary = "a `sajrone` loop";
  let description = [{
    The `cond` region computes the condition and ends in `jawa.condition`;
    the body runs while it is true. `jawa.continue` re-evaluates the
    condition and `jawa.break` leaves the loop.

    ```mlir
    jawa.while {
      %c = ...
      jawa.condition(%c)
    } do {
      jawa.yield
    }
    ```
  }];
  let regions = (region SizedRegion<1>:$cond, MaxSizedRegion<1>:$body);
  let assemblyFormat = "$cond `do` $body attr-dict";
  let hasVerifier = 1;
}

def Jawa_ForOp : Jawa_Op<"for", [RecursiveMemoryEffects, NoRegionArguments]> {
  let summary = "a `kanggo` loop";
  let description = [{
    Like `jawa.while`, plus a `step` region that runs after each iteration,
    including one ended by `jawa.continue`.
  }];
  let regions = (region SizedRegion<1>:$cond, MaxSizedRegion<1>:$body,
                        SizedRegion<1>:$step);
  let assemblyFormat = "$cond `do` $body `step` $step attr-dict";
  let hasVerifier = 1;
}

def Jawa_ConditionOp : Jawa_Op<"condition",
    [Terminator, ParentOneOf<["WhileOp", "ForOp"]>]> {
  let summary = "ends a loop condition region";
  let arguments = (ins I1:$condition);
  let assemblyFormat = "`(` $condition `)` attr-dict";
}

def Jawa_YieldOp : Jawa_Op<"yield",
    [Pure, Terminator, ParentOneOf<["IfOp", "WhileOp", "ForOp"]>]> {
  let summary = "ends a region and continues normally";
  let assemblyFormat = "attr-dict";
}

def Jawa_BreakOp : Jawa_Op<"break", [Terminator]> {
  let summary = "leaves the innermost loop (`mandheg`)";
  let assemblyFormat = "attr-dict";
  let hasVerifier = 1;
}

def Jawa_ContinueOp : Jawa_Op<"continue", [Terminator]> {
  let summary = "starts the next iteration of the innermost loop (`terusna`)";
  let assemblyFormat = "attr-dict";
  let hasVerifier = 1;
}

def Jawa_ReturnOp : Jawa_Op<"return", [Terminator]> {
  let summary = "returns from the function, from any depth (`bali`)";
  let arguments = (ins Optional<Jawa_ValueType>:$value);
  let assemblyFormat = "($value^ `:` type($value))? attr-dict";
  let hasVerifier = 1;
}

def Jawa_UnreachableOp : Jawa_Op<"unreachable", [Terminator]> {
  let summary = "marks code that control never reaches";
  let assemblyFormat = "attr-dict";
}

#endif // JAWA_DIALECT_JAWA_IR_JAWAOPS_TD
