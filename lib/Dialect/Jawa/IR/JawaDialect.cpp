//===- JawaDialect.cpp - The Jawa dialect ---------------------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#include "jawa/Dialect/Jawa/IR/Jawa.h"

#include "mlir/IR/Builders.h"
#include "mlir/IR/DialectImplementation.h"
#include "mlir/Transforms/InliningUtils.h"
#include "llvm/ADT/TypeSwitch.h"

using namespace mlir;
using namespace mlir::jawa;

#include "jawa/Dialect/Jawa/IR/JawaOpsDialect.cpp.inc"
#include "jawa/Dialect/Jawa/IR/JawaOpsEnums.cpp.inc"

#define GET_TYPEDEF_CLASSES
#include "jawa/Dialect/Jawa/IR/JawaOpsTypes.cpp.inc"

namespace {
/// Lets MLIR's inliner move Jawa operations into callers. `jawa.return` is
/// the exception: before `jawa-flatten-cfg` turns it into `func.return` it
/// may sit deep inside a region, where inlining would make it return from
/// the caller.
struct JawaInlinerInterface : public DialectInlinerInterface {
  using DialectInlinerInterface::DialectInlinerInterface;

  bool isLegalToInline(Operation *op, Region *, bool, IRMapping &) const final {
    return !isa<ReturnOp>(op);
  }

  bool isLegalToInline(Region *, Region *, bool, IRMapping &) const final {
    return true;
  }
};
} // namespace

void JawaDialect::initialize() {
  addTypes<
#define GET_TYPEDEF_LIST
#include "jawa/Dialect/Jawa/IR/JawaOpsTypes.cpp.inc"
      >();
  addOperations<
#define GET_OP_LIST
#include "jawa/Dialect/Jawa/IR/JawaOps.cpp.inc"
      >();
  addInterfaces<JawaInlinerInterface>();
}

Operation *JawaDialect::materializeConstant(OpBuilder &builder,
                                            Attribute value, Type type,
                                            Location loc) {
  if (auto text = dyn_cast<StringAttr>(value)) {
    if (isa<StrType>(type))
      return StrConstOp::create(builder, loc, type, text);
    return nullptr;
  }
  auto typed = dyn_cast<TypedAttr>(value);
  if (typed && typed.getType() == type && isScalarType(type) &&
      isa<IntegerAttr, FloatAttr>(value))
    return ConstantOp::create(builder, loc, type, typed);
  return nullptr;
}
