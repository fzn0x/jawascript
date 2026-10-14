//===- Passes.h - Jawa dialect passes ---------------------------*- C++ -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_H
#define JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_H

#include "mlir/Dialect/ControlFlow/IR/ControlFlow.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlowOps.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Pass/Pass.h"

namespace mlir::jawa {

#define GEN_PASS_DECL
#include "jawa/Dialect/Jawa/Transforms/Passes.h.inc"

#define GEN_PASS_REGISTRATION
#include "jawa/Dialect/Jawa/Transforms/Passes.h.inc"

} // namespace mlir::jawa

#endif // JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_H
