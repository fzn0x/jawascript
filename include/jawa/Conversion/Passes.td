//===- Passes.td - Jawa conversion passes ------------------*- tablegen -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_CONVERSION_PASSES_TD
#define JAWA_CONVERSION_PASSES_TD

include "mlir/Pass/PassBase.td"

def ConvertJawaToLLVM : Pass<"convert-jawa-to-llvm", "::mlir::ModuleOp"> {
  let summary = "Lower the Jawa dialect, func and cf to the LLVM dialect";
  let description = [{
    Expects flat control flow (run `jawa-flatten-cfg` first). Strings,
    arrays, structs and variable slots become `!llvm.ptr`; string literals
    become private constant globals laid out like the runtime's `jawa_str`;
    heap operations become calls into `jawa_rt`. Checked operations get
    their checks inline: an out-of-bounds index, a field of `kosong` or an
    integer division by zero branches to a cold block that calls the
    runtime's panic function.

    Defined functions are renamed with a `jw_` prefix so they cannot clash
    with C symbols, and every function except `jw_utama`, which the runtime
    calls, gets internal linkage.
  }];
  let dependentDialects = ["::mlir::LLVM::LLVMDialect"];
}

#endif // JAWA_CONVERSION_PASSES_TD
