//===- JawaDialect.td - The Jawa dialect -------------------*- tablegen -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_DIALECT_JAWA_IR_JAWADIALECT_TD
#define JAWA_DIALECT_JAWA_IR_JAWADIALECT_TD

include "mlir/IR/OpBase.td"

def Jawa_Dialect : Dialect {
  let name = "jawa";
  let cppNamespace = "::mlir::jawa";
  let summary = "High-level IR for the Jawa language";
  let description = [{
    The `jawa` dialect keeps the shape of Jawa source: structured control
    flow (`jawa.if`, `jawa.while`, `jawa.for` with `jawa.break`,
    `jawa.continue` and `jawa.return` anywhere inside), garbage-collected
    strings, arrays and structs, and checked operations such as division and
    indexing. Like ClangIR, it is lowered in steps: `jawa-flatten-cfg` turns
    the structured control flow into `cf` branches, and
    `convert-jawa-to-llvm` lowers the rest to the LLVM dialect with calls into
    the runtime.
  }];

  let useDefaultTypePrinterParser = 1;
  let hasConstantMaterializer = 1;
}

class Jawa_Op<string mnemonic, list<Trait> traits = []>
    : Op<Jawa_Dialect, mnemonic, traits>;

#endif // JAWA_DIALECT_JAWA_IR_JAWADIALECT_TD
