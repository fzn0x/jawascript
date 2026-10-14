//===- Pipeline.cpp - From the Jawa dialect to an object file -------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#include "jawa/Pipeline/Pipeline.h"

#include "jawa/Conversion/Passes.h"
#include "jawa/Dialect/Jawa/IR/Jawa.h"
#include "jawa/Dialect/Jawa/Transforms/Passes.h"

#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlow.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/Pass/PassRegistry.h"
#include "mlir/Target/LLVMIR/Dialect/Builtin/BuiltinToLLVMIRTranslation.h"
#include "mlir/Target/LLVMIR/Dialect/LLVMIR/LLVMToLLVMIRTranslation.h"
#include "mlir/Target/LLVMIR/Export.h"
#include "mlir/Transforms/Passes.h"

#include "llvm/IR/LegacyPassManager.h"
#include "llvm/MC/TargetRegistry.h"
#include "llvm/Passes/PassBuilder.h"
#include "llvm/Support/FileSystem.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Target/TargetMachine.h"
#include "llvm/TargetParser/Host.h"

using namespace mlir;
using namespace mlir::jawa;

void mlir::jawa::registerPipelineDialects(DialectRegistry &registry) {
  registry.insert<JawaDialect, arith::ArithDialect, func::FuncDialect,
                  cf::ControlFlowDialect, LLVM::LLVMDialect>();
  registerBuiltinDialectTranslation(registry);
  registerLLVMDialectTranslation(registry);
}

void mlir::jawa::buildJawaOptimizationPipeline(OpPassManager &pm,
                                               unsigned optLevel) {
  OpPassManager &functions = pm.nest<func::FuncOp>();
  if (optLevel > 0) {
    // Fold constants and merge string concatenation while the structure is
    // still visible.
    functions.addPass(createCanonicalizerPass());
    functions.addPass(createJawaFuseConcat());
    functions.addPass(createCSEPass());
  }
  functions.addPass(createJawaFlattenCFG());
  if (optLevel > 0) {
    // Local variables become SSA values on the flat control flow.
    functions.addPass(createMem2Reg());
    functions.addPass(createCanonicalizerPass());
    functions.addPass(createCSEPass());
    // Drop functions nothing calls; only `utama` is public.
    pm.addPass(createSymbolDCEPass());
  }
}

void mlir::jawa::buildJawaToLLVMPipeline(OpPassManager &pm, unsigned optLevel) {
  buildJawaOptimizationPipeline(pm, optLevel);
  pm.addPass(createConvertJawaToLLVM());
}

void mlir::jawa::registerJawaPipelines() {
  PassPipelineRegistration<>(
      "jawa-optimize",
      "Optimize Jawa IR and flatten its control flow, as `jawac -O2` does",
      [](OpPassManager &pm) { buildJawaOptimizationPipeline(pm, 2); });
  PassPipelineRegistration<>(
      "jawa-to-llvm",
      "Run the whole Jawa pipeline down to the LLVM dialect, as `jawac -O2` "
      "does",
      [](OpPassManager &pm) { buildJawaToLLVMPipeline(pm, 2); });
}

/// A target machine for the host, generic enough that the program runs on
/// any machine with the same operating system and architecture.
static std::unique_ptr<llvm::TargetMachine>
createHostTargetMachine(unsigned optLevel, std::string &error) {
  static const bool initialized = [] {
    llvm::InitializeNativeTarget();
    llvm::InitializeNativeTargetAsmPrinter();
    return true;
  }();
  (void)initialized;

  llvm::Triple triple(llvm::sys::getDefaultTargetTriple());
  const llvm::Target *target =
      llvm::TargetRegistry::lookupTarget(triple, error);
  if (!target)
    return nullptr;

  llvm::CodeGenOptLevel level = llvm::CodeGenOptLevel::Default;
  if (optLevel == 0)
    level = llvm::CodeGenOptLevel::None;
  else if (optLevel == 1)
    level = llvm::CodeGenOptLevel::Less;
  else if (optLevel >= 3)
    level = llvm::CodeGenOptLevel::Aggressive;

  llvm::TargetOptions options;
  return std::unique_ptr<llvm::TargetMachine>(target->createTargetMachine(
      triple, "generic", "", options, llvm::Reloc::PIC_, std::nullopt, level));
}

static void optimize(llvm::Module &module, llvm::TargetMachine &machine,
                     unsigned optLevel) {
  llvm::LoopAnalysisManager loops;
  llvm::FunctionAnalysisManager functions;
  llvm::CGSCCAnalysisManager sccs;
  llvm::ModuleAnalysisManager modules;
  llvm::PassBuilder builder(&machine);
  builder.registerModuleAnalyses(modules);
  builder.registerCGSCCAnalyses(sccs);
  builder.registerFunctionAnalyses(functions);
  builder.registerLoopAnalyses(loops);
  builder.crossRegisterProxies(loops, functions, sccs, modules);

  llvm::ModulePassManager passes;
  switch (optLevel) {
  case 0:
    passes = builder.buildO0DefaultPipeline(llvm::OptimizationLevel::O0);
    break;
  case 1:
    passes = builder.buildPerModuleDefaultPipeline(llvm::OptimizationLevel::O1);
    break;
  case 2:
    passes = builder.buildPerModuleDefaultPipeline(llvm::OptimizationLevel::O2);
    break;
  default:
    passes = builder.buildPerModuleDefaultPipeline(llvm::OptimizationLevel::O3);
    break;
  }
  passes.run(module, modules);
}

std::unique_ptr<llvm::Module>
mlir::jawa::translateToLLVMIR(ModuleOp module, llvm::LLVMContext &context,
                              unsigned optLevel, std::string &error) {
  std::unique_ptr<llvm::TargetMachine> machine =
      createHostTargetMachine(optLevel, error);
  if (!machine)
    return nullptr;
  std::unique_ptr<llvm::Module> llvmModule =
      translateModuleToLLVMIR(module, context, "jawa");
  if (!llvmModule) {
    error = "could not translate the module to LLVM IR";
    return nullptr;
  }
  llvmModule->setTargetTriple(machine->getTargetTriple());
  llvmModule->setDataLayout(machine->createDataLayout());
  optimize(*llvmModule, *machine, optLevel);
  return llvmModule;
}

LogicalResult mlir::jawa::writeObjectFile(llvm::Module &module, StringRef path,
                                          unsigned optLevel,
                                          std::string &error) {
  std::unique_ptr<llvm::TargetMachine> machine =
      createHostTargetMachine(optLevel, error);
  if (!machine)
    return failure();
  std::error_code code;
  llvm::raw_fd_ostream out(path, code, llvm::sys::fs::OF_None);
  if (code) {
    error = ("cannot write " + path + ": " + code.message()).str();
    return failure();
  }
  llvm::legacy::PassManager passes;
  if (machine->addPassesToEmitFile(passes, out, nullptr,
                                   llvm::CodeGenFileType::ObjectFile)) {
    error = "this target cannot write object files";
    return failure();
  }
  passes.run(module);
  out.flush();
  return success();
}
