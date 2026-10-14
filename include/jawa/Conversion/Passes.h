//===- Passes.h - Jawa conversion passes ------------------------*- C++ -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_CONVERSION_PASSES_H
#define JAWA_CONVERSION_PASSES_H

#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/IR/BuiltinOps.h"
#include "mlir/Pass/Pass.h"

namespace mlir::jawa {

#define GEN_PASS_DECL
#include "jawa/Conversion/Passes.h.inc"

#define GEN_PASS_REGISTRATION
#include "jawa/Conversion/Passes.h.inc"

} // namespace mlir::jawa

#endif // JAWA_CONVERSION_PASSES_H
