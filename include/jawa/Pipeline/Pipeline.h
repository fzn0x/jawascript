//===- Pipeline.h - From the Jawa dialect to an object file -----*- C++ -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// The compilation pipeline after the frontend has built a module in the
// `jawa` dialect:
//
//   jawa (structured)  --optimize-->  jawa (flat CFG, SSA)  --lower-->  llvm
//   --translate-->  LLVM IR  --LLVM -O<n>-->  object file
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_PIPELINE_PIPELINE_H
#define JAWA_PIPELINE_PIPELINE_H

#include "mlir/IR/BuiltinOps.h"
#include "mlir/Pass/PassManager.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/Module.h"

#include <memory>

namespace mlir::jawa {

/// Registers the dialects, translations and passes the pipeline needs.
void registerPipelineDialects(DialectRegistry &registry);

/// Adds the Jawa-level passes: optimizations (when `optLevel > 0`) and
/// `jawa-flatten-cfg`, leaving each function a flat CFG in the jawa dialect.
void buildJawaOptimizationPipeline(OpPassManager &pm, unsigned optLevel);

/// Adds `buildJawaOptimizationPipeline` followed by the lowering to the LLVM
/// dialect.
void buildJawaToLLVMPipeline(OpPassManager &pm, unsigned optLevel);

/// Registers `-jawa-optimize` and `-jawa-to-llvm` pipelines for jawa-opt.
void registerJawaPipelines();

/// Translates a module in the LLVM dialect to LLVM IR for the host, with the
/// host's triple and data layout, and runs LLVM's -O<optLevel> pipeline.
std::unique_ptr<llvm::Module> translateToLLVMIR(ModuleOp module,
                                                llvm::LLVMContext &context,
                                                unsigned optLevel,
                                                std::string &error);

/// Writes `module` as a native object file at `path`.
LogicalResult writeObjectFile(llvm::Module &module, StringRef path,
                              unsigned optLevel, std::string &error);

} // namespace mlir::jawa

#endif // JAWA_PIPELINE_PIPELINE_H
