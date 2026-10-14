//===- Jawa.h - The Jawa dialect --------------------------------*- C++ -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// The `jawa` dialect: types, attributes and operations.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_IR_JAWA_H
#define JAWA_DIALECT_JAWA_IR_JAWA_H

#include "mlir/Bytecode/BytecodeOpInterface.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/IR/BuiltinTypes.h"
#include "mlir/IR/Dialect.h"
#include "mlir/IR/OpDefinition.h"
#include "mlir/IR/OpImplementation.h"
#include "mlir/IR/SymbolTable.h"
#include "mlir/Interfaces/InferTypeOpInterface.h"
#include "mlir/Interfaces/MemorySlotInterfaces.h"
#include "mlir/Interfaces/SideEffectInterfaces.h"

#include "jawa/Dialect/Jawa/IR/JawaOpsDialect.h.inc"
#include "jawa/Dialect/Jawa/IR/JawaOpsEnums.h.inc"

#define GET_TYPEDEF_CLASSES
#include "jawa/Dialect/Jawa/IR/JawaOpsTypes.h.inc"

#define GET_OP_CLASSES
#include "jawa/Dialect/Jawa/IR/JawaOps.h.inc"

namespace mlir::jawa {

/// Whether values of `type` are references to the garbage-collected heap.
inline bool isReferenceType(Type type) {
  return llvm::isa<StrType, ArrayType, StructType>(type);
}

/// Whether `type` is `i64`, `f64` or `i1`.
inline bool isScalarType(Type type) {
  return type.isInteger(64) || type.isF64() || type.isInteger(1);
}

/// Whether `type` is a Jawa value type: a scalar or a reference.
inline bool isValueType(Type type) {
  return isScalarType(type) || isReferenceType(type);
}

} // namespace mlir::jawa

#endif // JAWA_DIALECT_JAWA_IR_JAWA_H
