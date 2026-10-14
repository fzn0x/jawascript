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
#include "jawa/Pipeline/Pipeline.h"

#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlow.h"
#include "mlir/Dialect/Func/Extensions/InlinerExtension.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/IR/DialectRegistry.h"
#include "mlir/Tools/mlir-opt/MlirOptMain.h"
#include "mlir/Transforms/Passes.h"
#include "llvm/ADT/StringRef.h"

#include <vector>

int main(int argc, char **argv) {
#ifdef __MINGW32__
  // MSYS2's MLIR libraries crash when two threads create the same attribute
  // at once (stock mlir-opt does too), so run passes on one thread there.
  std::vector<char *> args(argv, argv + argc);
  static char disableThreading[] = "--mlir-disable-threading";
  bool alreadyDisabled = false;
  for (char *arg : args)
    alreadyDisabled |= llvm::StringRef(arg) == disableThreading;
  if (!alreadyDisabled)
    args.push_back(disableThreading);
  argc = static_cast<int>(args.size());
  args.push_back(nullptr);
  argv = args.data();
#endif

  mlir::DialectRegistry registry;
  registry.insert<mlir::jawa::JawaDialect, mlir::arith::ArithDialect,
                  mlir::func::FuncDialect, mlir::cf::ControlFlowDialect,
                  mlir::LLVM::LLVMDialect>();
  mlir::func::registerInlinerExtension(registry);
  mlir::registerTransformsPasses();
  mlir::jawa::registerJawaPasses();
  mlir::jawa::registerJawaConversionPasses();
  mlir::jawa::registerJawaPipelines();

  return mlir::asMainReturnCode(
      mlir::MlirOptMain(argc, argv, "Jawa optimizer driver\n", registry));
}
