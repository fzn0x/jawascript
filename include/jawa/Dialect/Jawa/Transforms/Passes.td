//===- Passes.td - Jawa dialect passes ---------------------*- tablegen -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_TD
#define JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_TD

include "mlir/Pass/PassBase.td"

def JawaFlattenCFG : Pass<"jawa-flatten-cfg", "::mlir::func::FuncOp"> {
  let summary = "Lower structured Jawa control flow to cf branches";
  let description = [{
    Inlines the regions of `jawa.if`, `jawa.while` and `jawa.for` into the
    enclosing function and connects them with `cf.br` and `cf.cond_br`.
    `jawa.break` and `jawa.continue` become branches to the loop's exit and
    latch (the `step` region of a `jawa.for`), and `jawa.return` becomes
    `func.return`. Operations are flattened innermost first, so by the time a
    loop is flattened every `break` left in its body belongs to it.

    Afterwards each function is a plain control-flow graph, ready for
    `mem2reg` and the LLVM lowering.
  }];
  let dependentDialects = ["::mlir::cf::ControlFlowDialect"];
}

def JawaFuseConcat : Pass<"jawa-fuse-concat", "::mlir::func::FuncOp"> {
  let summary = "Merge chains of string concatenation into one allocation";
  let description = [{
    `a + b + c` on strings arrives as nested binary `jawa.concat` ops, and
    each would allocate an intermediate string. This pass rewrites trees of
    single-use concats into one n-ary `jawa.concat`, which the runtime builds
    with a single allocation, and joins neighbouring constant pieces at
    compile time.
  }];
}

#endif // JAWA_DIALECT_JAWA_TRANSFORMS_PASSES_TD
