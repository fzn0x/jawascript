//===- JawaOps.cpp - Jawa dialect operations ------------------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#include "jawa/Dialect/Jawa/IR/Jawa.h"

#include "mlir/IR/Builders.h"
#include "mlir/IR/Matchers.h"
#include "mlir/Interfaces/FunctionInterfaces.h"
#include "llvm/ADT/STLExtras.h"
#include "llvm/ADT/SmallSet.h"

#include <cmath>
#include <limits>
#include <string>

using namespace mlir;
using namespace mlir::jawa;

#define GET_OP_CLASSES
#include "jawa/Dialect/Jawa/IR/JawaOps.cpp.inc"

//===----------------------------------------------------------------------===//
// Folding helpers
//===----------------------------------------------------------------------===//

namespace {
using IntFold =
    llvm::function_ref<std::optional<APInt>(const APInt &, const APInt &)>;
using FloatFold =
    llvm::function_ref<std::optional<APFloat>(const APFloat &, const APFloat &)>;
} // namespace

/// Folds a binary operation whose operands are both constants.
static Attribute foldBinary(Type type, Attribute lhs, Attribute rhs,
                            IntFold intFold, FloatFold floatFold) {
  auto lhsInt = dyn_cast_or_null<IntegerAttr>(lhs);
  auto rhsInt = dyn_cast_or_null<IntegerAttr>(rhs);
  if (lhsInt && rhsInt) {
    if (std::optional<APInt> result = intFold(lhsInt.getValue(), rhsInt.getValue()))
      return IntegerAttr::get(type, *result);
    return {};
  }
  auto lhsFloat = dyn_cast_or_null<FloatAttr>(lhs);
  auto rhsFloat = dyn_cast_or_null<FloatAttr>(rhs);
  if (lhsFloat && rhsFloat) {
    if (std::optional<APFloat> result =
            floatFold(lhsFloat.getValue(), rhsFloat.getValue()))
      return FloatAttr::get(type, *result);
  }
  return {};
}

static bool isIntConstant(Attribute attr, int64_t value) {
  auto integer = dyn_cast_or_null<IntegerAttr>(attr);
  return integer && integer.getValue().getSExtValue() == value;
}

//===----------------------------------------------------------------------===//
// Constants
//===----------------------------------------------------------------------===//

OpFoldResult ConstantOp::fold(FoldAdaptor) { return getValue(); }

LogicalResult ConstantOp::verify() {
  if (!isa<IntegerAttr, FloatAttr>(getValue()))
    return emitOpError("expects an integer or float value");
  return success();
}

OpFoldResult StrConstOp::fold(FoldAdaptor) { return getValueAttr(); }

//===----------------------------------------------------------------------===//
// Arithmetic
//===----------------------------------------------------------------------===//

OpFoldResult AddOp::fold(FoldAdaptor adaptor) {
  if (isa<IntegerType>(getType()) && isIntConstant(adaptor.getRhs(), 0))
    return getLhs();
  return foldBinary(
      getType(), adaptor.getLhs(), adaptor.getRhs(),
      [](const APInt &a, const APInt &b) -> std::optional<APInt> {
        return a + b;
      },
      [](const APFloat &a, const APFloat &b) -> std::optional<APFloat> {
        return a + b;
      });
}

OpFoldResult SubOp::fold(FoldAdaptor adaptor) {
  if (isa<IntegerType>(getType())) {
    if (isIntConstant(adaptor.getRhs(), 0))
      return getLhs();
    if (getLhs() == getRhs())
      return IntegerAttr::get(getType(), 0);
  }
  return foldBinary(
      getType(), adaptor.getLhs(), adaptor.getRhs(),
      [](const APInt &a, const APInt &b) -> std::optional<APInt> {
        return a - b;
      },
      [](const APFloat &a, const APFloat &b) -> std::optional<APFloat> {
        return a - b;
      });
}

OpFoldResult MulOp::fold(FoldAdaptor adaptor) {
  if (isa<IntegerType>(getType())) {
    if (isIntConstant(adaptor.getRhs(), 1))
      return getLhs();
    if (isIntConstant(adaptor.getRhs(), 0))
      return adaptor.getRhs();
  }
  return foldBinary(
      getType(), adaptor.getLhs(), adaptor.getRhs(),
      [](const APInt &a, const APInt &b) -> std::optional<APInt> {
        return a * b;
      },
      [](const APFloat &a, const APFloat &b) -> std::optional<APFloat> {
        return a * b;
      });
}

OpFoldResult DivOp::fold(FoldAdaptor adaptor) {
  if (isa<IntegerType>(getType()) && isIntConstant(adaptor.getRhs(), 1))
    return getLhs();
  return foldBinary(
      getType(), adaptor.getLhs(), adaptor.getRhs(),
      [](const APInt &a, const APInt &b) -> std::optional<APInt> {
        // Division by zero is a runtime error; leave it for the program.
        if (b.isZero())
          return std::nullopt;
        // MIN / -1 wraps, like the generated code.
        if (b.isAllOnes())
          return -a;
        return a.sdiv(b);
      },
      [](const APFloat &a, const APFloat &b) -> std::optional<APFloat> {
        return a / b;
      });
}

OpFoldResult RemOp::fold(FoldAdaptor adaptor) {
  return foldBinary(
      getType(), adaptor.getLhs(), adaptor.getRhs(),
      [](const APInt &a, const APInt &b) -> std::optional<APInt> {
        if (b.isZero())
          return std::nullopt;
        if (b.isAllOnes())
          return APInt(a.getBitWidth(), 0);
        return a.srem(b);
      },
      [](const APFloat &a, const APFloat &b) -> std::optional<APFloat> {
        APFloat result = a;
        result.mod(b);
        return result;
      });
}

/// Integer division and remainder stop the program when the divisor is zero,
/// so they keep a side effect unless the divisor is a known non-zero
/// constant. Float division never traps.
static void getTrappingDivEffects(
    Operation *op, Value rhs,
    SmallVectorImpl<SideEffects::EffectInstance<MemoryEffects::Effect>>
        &effects) {
  if (!isa<IntegerType>(op->getResult(0).getType()))
    return;
  APInt divisor;
  if (matchPattern(rhs, m_ConstantInt(&divisor)) && !divisor.isZero())
    return;
  effects.emplace_back(MemoryEffects::Write::get());
}

void DivOp::getEffects(
    SmallVectorImpl<SideEffects::EffectInstance<MemoryEffects::Effect>>
        &effects) {
  getTrappingDivEffects(getOperation(), getRhs(), effects);
}

void RemOp::getEffects(
    SmallVectorImpl<SideEffects::EffectInstance<MemoryEffects::Effect>>
        &effects) {
  getTrappingDivEffects(getOperation(), getRhs(), effects);
}

OpFoldResult NegOp::fold(FoldAdaptor adaptor) {
  if (auto inner = getOperand().getDefiningOp<NegOp>())
    return inner.getOperand();
  if (auto integer = dyn_cast_or_null<IntegerAttr>(adaptor.getOperand()))
    return IntegerAttr::get(getType(), -integer.getValue());
  if (auto real = dyn_cast_or_null<FloatAttr>(adaptor.getOperand()))
    return FloatAttr::get(getType(), llvm::neg(real.getValue()));
  return {};
}

OpFoldResult NotOp::fold(FoldAdaptor adaptor) {
  if (auto inner = getOperand().getDefiningOp<NotOp>())
    return inner.getOperand();
  if (auto boolean = dyn_cast_or_null<IntegerAttr>(adaptor.getOperand()))
    return BoolAttr::get(getContext(), boolean.getValue().isZero());
  return {};
}

//===----------------------------------------------------------------------===//
// CmpOp
//===----------------------------------------------------------------------===//

static bool isOrderingPredicate(CmpPredicate predicate) {
  return predicate != CmpPredicate::eq && predicate != CmpPredicate::ne;
}

LogicalResult CmpOp::verify() {
  Type type = getLhs().getType();
  if (isOrderingPredicate(getPredicate()) &&
      !(type.isInteger(64) || type.isF64() || isa<StrType>(type)))
    return emitOpError("predicate '")
           << stringifyCmpPredicate(getPredicate())
           << "' needs i64, f64 or !jawa.str operands, got " << type;
  return success();
}

/// Applies `predicate` to the result of a three-way comparison.
static bool applyPredicate(CmpPredicate predicate, int order) {
  switch (predicate) {
  case CmpPredicate::eq:
    return order == 0;
  case CmpPredicate::ne:
    return order != 0;
  case CmpPredicate::lt:
    return order < 0;
  case CmpPredicate::le:
    return order <= 0;
  case CmpPredicate::gt:
    return order > 0;
  case CmpPredicate::ge:
    return order >= 0;
  }
  llvm_unreachable("unknown predicate");
}

OpFoldResult CmpOp::fold(FoldAdaptor adaptor) {
  MLIRContext *context = getContext();
  CmpPredicate predicate = getPredicate();

  // A value always equals itself, except a float NaN.
  if (getLhs() == getRhs() && !getLhs().getType().isF64())
    return BoolAttr::get(context, applyPredicate(predicate, 0));

  Attribute lhs = adaptor.getLhs();
  Attribute rhs = adaptor.getRhs();
  if (!lhs || !rhs)
    return {};

  if (auto a = dyn_cast<IntegerAttr>(lhs)) {
    auto b = cast<IntegerAttr>(rhs);
    const APInt &x = a.getValue();
    const APInt &y = b.getValue();
    int order = x.slt(y) ? -1 : (x == y ? 0 : 1);
    return BoolAttr::get(context, applyPredicate(predicate, order));
  }
  if (auto a = dyn_cast<FloatAttr>(lhs)) {
    APFloat::cmpResult result = a.getValue().compare(cast<FloatAttr>(rhs).getValue());
    if (result == APFloat::cmpUnordered)
      return BoolAttr::get(context, predicate == CmpPredicate::ne);
    int order = result == APFloat::cmpLessThan
                    ? -1
                    : (result == APFloat::cmpEqual ? 0 : 1);
    return BoolAttr::get(context, applyPredicate(predicate, order));
  }
  if (auto a = dyn_cast<StringAttr>(lhs)) {
    int order = a.getValue().compare(cast<StringAttr>(rhs).getValue());
    return BoolAttr::get(context, applyPredicate(predicate, order));
  }
  return {};
}

//===----------------------------------------------------------------------===//
// Strings and conversions
//===----------------------------------------------------------------------===//

LogicalResult ConcatOp::verify() {
  if (getParts().empty())
    return emitOpError("needs at least one string");
  return success();
}

OpFoldResult ConcatOp::fold(FoldAdaptor adaptor) {
  if (getParts().size() == 1)
    return getParts().front();
  std::string joined;
  for (Attribute part : adaptor.getParts()) {
    auto text = dyn_cast_or_null<StringAttr>(part);
    if (!text)
      return {};
    joined += text.getValue();
  }
  return StringAttr::get(getContext(), joined);
}

OpFoldResult ToStrOp::fold(FoldAdaptor adaptor) {
  // Floats are formatted by the runtime; folding them here could disagree
  // with it in the last digit, so only integers and booleans fold.
  auto integer = dyn_cast_or_null<IntegerAttr>(adaptor.getValue());
  if (!integer)
    return {};
  if (getValue().getType().isInteger(1))
    return StringAttr::get(getContext(),
                           integer.getValue().isZero() ? "salah" : "bener");
  return StringAttr::get(getContext(),
                         std::to_string(integer.getValue().getSExtValue()));
}

LogicalResult CastOp::verify() {
  Type from = getValue().getType();
  Type to = getType();
  auto expect = [&](bool fromOk, bool toOk, StringRef shape) -> LogicalResult {
    if (fromOk && toOk)
      return success();
    return emitOpError("'") << stringifyCastKind(getKind()) << "' converts "
                            << shape << ", got " << from << " -> " << to;
  };
  switch (getKind()) {
  case CastKind::int_to_float:
    return expect(from.isInteger(64), to.isF64(), "i64 -> f64");
  case CastKind::float_to_int:
    return expect(from.isF64(), to.isInteger(64), "f64 -> i64");
  case CastKind::bool_to_int:
    return expect(from.isInteger(1), to.isInteger(64), "i1 -> i64");
  case CastKind::str_to_int:
    return expect(isa<StrType>(from), to.isInteger(64), "!jawa.str -> i64");
  case CastKind::str_to_float:
    return expect(isa<StrType>(from), to.isF64(), "!jawa.str -> f64");
  }
  llvm_unreachable("unknown cast kind");
}

OpFoldResult CastOp::fold(FoldAdaptor adaptor) {
  Attribute value = adaptor.getValue();
  if (!value)
    return {};
  switch (getKind()) {
  case CastKind::int_to_float: {
    APFloat result(APFloat::IEEEdouble());
    result.convertFromAPInt(cast<IntegerAttr>(value).getValue(),
                            /*IsSigned=*/true, APFloat::rmNearestTiesToEven);
    return FloatAttr::get(getType(), result);
  }
  case CastKind::float_to_int: {
    // Truncate toward zero and saturate; NaN becomes 0.
    double real = cast<FloatAttr>(value).getValueAsDouble();
    int64_t result;
    if (std::isnan(real))
      result = 0;
    else if (real >= 9223372036854775808.0)
      result = std::numeric_limits<int64_t>::max();
    else if (real < -9223372036854775808.0)
      result = std::numeric_limits<int64_t>::min();
    else
      result = static_cast<int64_t>(real);
    return IntegerAttr::get(getType(), result);
  }
  case CastKind::bool_to_int:
    return IntegerAttr::get(getType(),
                            cast<IntegerAttr>(value).getValue().isZero() ? 0
                                                                         : 1);
  case CastKind::str_to_int:
  case CastKind::str_to_float:
    // Parsing can fail at runtime; leave it to the program.
    return {};
  }
  llvm_unreachable("unknown cast kind");
}

void CastOp::getEffects(
    SmallVectorImpl<SideEffects::EffectInstance<MemoryEffects::Effect>>
        &effects) {
  if (getKind() == CastKind::str_to_int || getKind() == CastKind::str_to_float)
    effects.emplace_back(MemoryEffects::Write::get());
}

OpFoldResult StrLenOp::fold(FoldAdaptor adaptor) {
  if (auto text = dyn_cast_or_null<StringAttr>(adaptor.getStr()))
    return IntegerAttr::get(getType(), text.getValue().size());
  return {};
}

//===----------------------------------------------------------------------===//
// Arrays
//===----------------------------------------------------------------------===//

LogicalResult ArrayNewOp::verify() {
  Type element = getType().getElementType();
  for (auto [index, value] : llvm::enumerate(getElements()))
    if (value.getType() != element)
      return emitOpError("element ")
             << index << " has type " << value.getType() << ", expected "
             << element;
  return success();
}

LogicalResult ArrayGetOp::verify() {
  Type element = getArray().getType().getElementType();
  if (getType() != element)
    return emitOpError("result type ")
           << getType() << " does not match the element type " << element;
  return success();
}

LogicalResult ArraySetOp::verify() {
  Type element = getArray().getType().getElementType();
  if (getValue().getType() != element)
    return emitOpError("value type ")
           << getValue().getType() << " does not match the element type "
           << element;
  return success();
}

LogicalResult ArrayPushOp::verify() {
  Type element = getArray().getType().getElementType();
  if (getValue().getType() != element)
    return emitOpError("value type ")
           << getValue().getType() << " does not match the element type "
           << element;
  return success();
}

//===----------------------------------------------------------------------===//
// Structs
//===----------------------------------------------------------------------===//

void StructDefOp::print(OpAsmPrinter &printer) {
  printer << ' ';
  printer.printSymbolName(getSymName());
  printer << " {";
  llvm::interleaveComma(
      llvm::zip(getFieldNames(), getFieldTypes()), printer, [&](auto field) {
        printer.printKeywordOrString(
            cast<StringAttr>(std::get<0>(field)).getValue());
        printer << ": ";
        printer.printType(cast<TypeAttr>(std::get<1>(field)).getValue());
      });
  printer << '}';
  printer.printOptionalAttrDict((*this)->getAttrs(),
                                {getSymNameAttrName(), getFieldNamesAttrName(),
                                 getFieldTypesAttrName()});
}

ParseResult StructDefOp::parse(OpAsmParser &parser, OperationState &result) {
  StringAttr name;
  if (parser.parseSymbolName(name))
    return failure();
  SmallVector<Attribute> names;
  SmallVector<Attribute> types;
  if (parser.parseLBrace())
    return failure();
  if (failed(parser.parseOptionalRBrace())) {
    do {
      std::string field;
      Type type;
      if (parser.parseKeywordOrString(&field) || parser.parseColon() ||
          parser.parseType(type))
        return failure();
      names.push_back(parser.getBuilder().getStringAttr(field));
      types.push_back(TypeAttr::get(type));
    } while (succeeded(parser.parseOptionalComma()));
    if (parser.parseRBrace())
      return failure();
  }
  if (parser.parseOptionalAttrDict(result.attributes))
    return failure();
  Builder &builder = parser.getBuilder();
  result.addAttribute(getSymNameAttrName(result.name), name);
  result.addAttribute(getFieldNamesAttrName(result.name),
                      builder.getArrayAttr(names));
  result.addAttribute(getFieldTypesAttrName(result.name),
                      builder.getArrayAttr(types));
  return success();
}

LogicalResult StructDefOp::verify() {
  if (getFieldNames().size() != getFieldTypes().size())
    return emitOpError("has ") << getFieldNames().size() << " field names but "
                               << getFieldTypes().size() << " field types";
  llvm::SmallSet<StringRef, 8> seen;
  for (auto [name, type] : llvm::zip(getFieldNames(), getFieldTypes())) {
    StringRef field = cast<StringAttr>(name).getValue();
    if (!seen.insert(field).second)
      return emitOpError("declares field '") << field << "' twice";
    Type fieldType = cast<TypeAttr>(type).getValue();
    if (!isValueType(fieldType))
      return emitOpError("field '")
             << field << "' has type " << fieldType
             << ", which is not a Jawa value type";
  }
  return success();
}

/// Finds the definition of `type`, reporting an error on `op` if it is missing.
static StructDefOp lookupStruct(Operation *op, StructType type,
                                SymbolTableCollection &symbolTable) {
  auto def = symbolTable.lookupNearestSymbolFrom<StructDefOp>(
      op, StringAttr::get(op->getContext(), type.getName()));
  if (!def)
    op->emitOpError("refers to undefined struct '") << type.getName() << "'";
  return def;
}

LogicalResult
StructNewOp::verifySymbolUses(SymbolTableCollection &symbolTable) {
  StructDefOp def = lookupStruct(*this, getType(), symbolTable);
  if (!def)
    return failure();
  if (def.getNumFields() != getFields().size())
    return emitOpError("expects ")
           << def.getNumFields() << " field values, got "
           << getFields().size();
  for (auto [index, value] : llvm::enumerate(getFields()))
    if (value.getType() != def.getFieldType(index))
      return emitOpError("field ")
             << index << " has type " << value.getType() << ", expected "
             << def.getFieldType(index);
  return success();
}

/// Checks a field index against the struct definition and returns the
/// field's type, or null after reporting an error.
static Type verifyFieldAccess(Operation *op, StructType type, uint64_t index,
                              SymbolTableCollection &symbolTable) {
  StructDefOp def = lookupStruct(op, type, symbolTable);
  if (!def)
    return {};
  if (index >= def.getNumFields()) {
    op->emitOpError("field index ")
        << index << " is out of range for '" << type.getName() << "'";
    return {};
  }
  return def.getFieldType(index);
}

LogicalResult
GetFieldOp::verifySymbolUses(SymbolTableCollection &symbolTable) {
  Type field = verifyFieldAccess(*this, getObject().getType(), getIndex(),
                                 symbolTable);
  if (!field)
    return failure();
  if (getType() != field)
    return emitOpError("result type ")
           << getType() << " does not match the field type " << field;
  return success();
}

LogicalResult
SetFieldOp::verifySymbolUses(SymbolTableCollection &symbolTable) {
  Type field = verifyFieldAccess(*this, getObject().getType(), getIndex(),
                                 symbolTable);
  if (!field)
    return failure();
  if (getValue().getType() != field)
    return emitOpError("value type ")
           << getValue().getType() << " does not match the field type "
           << field;
  return success();
}

//===----------------------------------------------------------------------===//
// Built-in functions
//===----------------------------------------------------------------------===//

LogicalResult PrintOp::verify() {
  for (Value value : getValues()) {
    Type type = value.getType();
    if (auto array = dyn_cast<ArrayType>(type))
      type = array.getElementType();
    if (!isScalarType(type) && !isa<StrType>(type))
      return emitOpError("cannot print a value of type ") << value.getType();
  }
  return success();
}

//===----------------------------------------------------------------------===//
// Structured control flow
//===----------------------------------------------------------------------===//

/// Checks that a non-empty region ends in a terminator that leaves it the
/// normal way (`jawa.yield`) or jumps out (`break`, `continue`, `return`).
static LogicalResult verifyExitRegion(Operation *op, Region &region,
                                      StringRef name) {
  if (region.empty())
    return success();
  Block &block = region.front();
  if (block.empty() || !block.back().hasTrait<OpTrait::IsTerminator>())
    return op->emitOpError("'") << name << "' region must end in a terminator";
  Operation &terminator = block.back();
  if (!isa<YieldOp, BreakOp, ContinueOp, ReturnOp, UnreachableOp>(terminator))
    return op->emitOpError("'")
           << name << "' region cannot end in '" << terminator.getName()
           << "'";
  return success();
}

LogicalResult IfOp::verify() {
  if (getThenRegion().empty())
    return emitOpError("needs a 'then' region");
  if (failed(verifyExitRegion(*this, getThenRegion(), "then")))
    return failure();
  return verifyExitRegion(*this, getElseRegion(), "else");
}

static LogicalResult verifyLoop(Operation *op, Region &cond, Region &body) {
  Block &condBlock = cond.front();
  if (condBlock.empty() || !isa<ConditionOp>(condBlock.back()))
    return op->emitOpError("condition region must end in 'jawa.condition'");
  if (body.empty())
    return op->emitOpError("needs a body region");
  return verifyExitRegion(op, body, "body");
}

LogicalResult WhileOp::verify() {
  return verifyLoop(*this, getCond(), getBody());
}

LogicalResult ForOp::verify() {
  if (failed(verifyLoop(*this, getCond(), getBody())))
    return failure();
  Block &step = getStep().front();
  if (step.empty() || !isa<YieldOp>(step.back()))
    return emitOpError("step region must end in 'jawa.yield'");
  return success();
}

/// `break` and `continue` must sit in the body of an enclosing loop, possibly
/// nested inside `jawa.if`.
static LogicalResult verifyInLoopBody(Operation *op) {
  Operation *current = op;
  while (Operation *parent = current->getParentOp()) {
    if (auto loop = dyn_cast<WhileOp>(parent)) {
      if (current->getParentRegion() == &loop.getBody())
        return success();
      break;
    }
    if (auto loop = dyn_cast<ForOp>(parent)) {
      if (current->getParentRegion() == &loop.getBody())
        return success();
      break;
    }
    if (!isa<IfOp>(parent))
      break;
    current = parent;
  }
  return op->emitOpError("must be inside the body of a loop");
}

LogicalResult BreakOp::verify() { return verifyInLoopBody(*this); }

LogicalResult ContinueOp::verify() { return verifyInLoopBody(*this); }

LogicalResult ReturnOp::verify() {
  auto function = (*this)->getParentOfType<FunctionOpInterface>();
  if (!function)
    return emitOpError("must be inside a function");
  ArrayRef<Type> results = function.getResultTypes();
  if (Value value = getValue()) {
    if (results.size() == 1 && results.front() == value.getType())
      return success();
    InFlightDiagnostic diag = emitOpError("returns ")
                              << value.getType()
                              << ", but the function returns ";
    if (results.empty())
      diag << "nothing";
    else
      diag << results.front();
    return diag;
  }
  if (!results.empty())
    return emitOpError("must return a value of type ") << results.front();
  return success();
}
