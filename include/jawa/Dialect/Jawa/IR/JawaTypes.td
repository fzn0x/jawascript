//===- JawaTypes.td - Jawa dialect types -------------------*- tablegen -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_IR_JAWATYPES_TD
#define JAWA_DIALECT_JAWA_IR_JAWATYPES_TD

include "jawa/Dialect/Jawa/IR/JawaDialect.td"
include "mlir/IR/AttrTypeBase.td"

class Jawa_Type<string name, string typeMnemonic, list<Trait> traits = []>
    : TypeDef<Jawa_Dialect, name, traits> {
  let mnemonic = typeMnemonic;
}

def Jawa_StrType : Jawa_Type<"Str", "str"> {
  let summary = "an immutable, garbage-collected string (`ukara`)";
}

def Jawa_ArrayType : Jawa_Type<"Array", "array"> {
  let summary = "a growable, garbage-collected array (`[T]`)";
  let parameters = (ins "::mlir::Type":$elementType);
  let assemblyFormat = "`<` $elementType `>`";
}

def Jawa_StructType : Jawa_Type<"Struct", "struct"> {
  let summary = "a reference to a garbage-collected struct (`wujud`)";
  let description = [{
    Structs are nominal: the type names a `jawa.struct` definition in the
    enclosing module, which holds the field layout. Naming the definition
    rather than embedding the fields lets structs refer to themselves.
  }];
  let parameters = (ins StringRefParameter<"the struct name">:$name);
  let assemblyFormat = "`<` $name `>`";
}

def Jawa_PtrType : Jawa_Type<"Ptr", "ptr"> {
  let summary = "the address of a local variable slot";
  let parameters = (ins "::mlir::Type":$pointee);
  let assemblyFormat = "`<` $pointee `>`";
}

// Type constraints.
def Jawa_RefType : AnyTypeOf<[Jawa_StrType, Jawa_ArrayType, Jawa_StructType],
                             "a Jawa reference type">;
def Jawa_NumericType : AnyTypeOf<[I64, F64], "`wilangan` or `pecahan`">;
def Jawa_ScalarType : AnyTypeOf<[I64, F64, I1],
                                "`wilangan`, `pecahan` or `logika`">;
def Jawa_ValueType : AnyTypeOf<[I64, F64, I1, Jawa_StrType, Jawa_ArrayType,
                                Jawa_StructType], "a Jawa value type">;

#endif // JAWA_DIALECT_JAWA_IR_JAWATYPES_TD
