//===- jawa-opt.cpp - The Jawa optimizer driver ---------------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// An `mlir-opt` for the Jawa dialect: parses MLIR, runs passes and prints the
// result. The lit tests use it to check each pass in isolation.
//
//===----------------------------------------------------------------------===//

#include "jawa/Conversion/Passes.h"
#include "jawa/Dialect/Jawa/IR/Jawa.h"
#include "jawa/Dialect/Jawa/Transforms/Passes.h"

#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlow.h"
#include "mlir/Dialect/Func/Extensions/InlinerExtension.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/IR/DialectRegistry.h"
#include "mlir/Tools/mlir-opt/MlirOptMain.h"
#include "mlir/Transforms/Passes.h"

int main(int argc, char **argv) {
  mlir::DialectRegistry registry;
  registry.insert<mlir::jawa::JawaDialect, mlir::arith::ArithDialect,
                  mlir::func::FuncDialect, mlir::cf::ControlFlowDialect,
                  mlir::LLVM::LLVMDialect>();
  mlir::func::registerInlinerExtension(registry);
  mlir::registerTransformsPasses();
  mlir::jawa::registerJawaPasses();
  mlir::jawa::registerJawaConversionPasses();

  return mlir::asMainReturnCode(
      mlir::MlirOptMain(argc, argv, "Jawa optimizer driver\n", registry));
}
