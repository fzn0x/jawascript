//===- JawaToLLVM.cpp - Lower the Jawa dialect to LLVM --------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// The layouts used here must match runtime/jawa_rt.h:
//
//   ukara  -> ptr to { i64 len, [len + 1 x i8] data }
//   [T]    -> ptr to { i64 len, i64 cap, ptr data }, data holding T elements
//   wujud  -> ptr to { field0, field1, ... }
//
//===----------------------------------------------------------------------===//

#include "jawa/Conversion/Passes.h"
#include "jawa/Dialect/Jawa/IR/Jawa.h"

#include "mlir/Conversion/ArithToLLVM/ArithToLLVM.h"
#include "mlir/Conversion/ControlFlowToLLVM/ControlFlowToLLVM.h"
#include "mlir/Conversion/FuncToLLVM/ConvertFuncToLLVM.h"
#include "mlir/Conversion/LLVMCommon/ConversionTarget.h"
#include "mlir/Conversion/LLVMCommon/Pattern.h"
#include "mlir/Conversion/LLVMCommon/TypeConverter.h"
#include "mlir/Dialect/Arith/IR/Arith.h"
#include "mlir/Dialect/ControlFlow/IR/ControlFlowOps.h"
#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/Dialect/LLVMIR/LLVMDialect.h"
#include "mlir/IR/Matchers.h"
#include "mlir/IR/SymbolTable.h"
#include "mlir/Transforms/DialectConversion.h"
#include "llvm/ADT/StringMap.h"

namespace mlir::jawa {
#define GEN_PASS_DEF_CONVERTJAWATOLLVM
#include "jawa/Conversion/Passes.h.inc"
} // namespace mlir::jawa

using namespace mlir;
using namespace mlir::jawa;

namespace {

/// Element kinds for `jawa_rt_print_array`; see `enum jawa_kind`.
enum class PrintKind : int32_t { Int = 0, Float = 1, Bool = 2, Str = 3 };

/// Declarations of the runtime functions in `jawa_rt.h`.
struct Runtime {
  LLVM::LLVMFuncOp alloc, allocAtomic;
  LLVM::LLVMFuncOp concat, strEq, strCmp;
  LLVM::LLVMFuncOp intToStr, floatToStr, boolToStr, strToInt, strToFloat;
  LLVM::LLVMFuncOp printInt, printFloat, printBool, printStr, printArray,
      printSpace, printNewline;
  LLVM::LLVMFuncOp arrayNew, arrayPush;
  LLVM::LLVMFuncOp panic, panicIndex, panicNull, panicDivZero;
};

/// What the patterns share: the runtime, string literal globals and struct
/// layouts, all created before the conversion starts.
struct LoweringState {
  Runtime runtime;
  llvm::StringMap<LLVM::GlobalOp> strings;
  llvm::StringMap<LLVM::LLVMStructType> structs;
};

Type ptrType(MLIRContext *context) {
  return LLVM::LLVMPointerType::get(context);
}

Value constI64(OpBuilder &builder, Location loc, int64_t value) {
  return LLVM::ConstantOp::create(builder, loc, builder.getI64Type(), value);
}

Value constI32(OpBuilder &builder, Location loc, int32_t value) {
  return LLVM::ConstantOp::create(builder, loc, builder.getI32Type(), value);
}

/// Calls a runtime function and returns its result, or null for `void`.
Value callRuntime(OpBuilder &builder, Location loc, LLVM::LLVMFuncOp function,
                  ValueRange args) {
  auto call = LLVM::CallOp::create(builder, loc, function, args);
  return call.getNumResults() ? call.getResult() : Value();
}

/// The allocation size of `type`, computed with the `getelementptr null, 1`
/// idiom so LLVM folds it with the target's data layout.
Value sizeOf(OpBuilder &builder, Location loc, Type type) {
  MLIRContext *context = builder.getContext();
  Value null = LLVM::ZeroOp::create(builder, loc, ptrType(context));
  Value end = LLVM::GEPOp::create(builder, loc, ptrType(context), type, null,
                                  ArrayRef<LLVM::GEPArg>{1});
  return LLVM::PtrToIntOp::create(builder, loc, builder.getI64Type(), end);
}

/// The LLVM struct type of an array header.
LLVM::LLVMStructType arrayHeaderType(MLIRContext *context) {
  Type i64 = IntegerType::get(context, 64);
  return LLVM::LLVMStructType::getLiteral(context,
                                          {i64, i64, ptrType(context)});
}

/// Branches to a cold block that calls `onFailure` when `failed` is true, and
/// leaves the rewriter in the block where execution continues otherwise.
void emitCheck(ConversionPatternRewriter &rewriter, Location loc, Value failed,
               llvm::function_ref<void()> onFailure) {
  Block *current = rewriter.getInsertionBlock();
  Block *continuation =
      rewriter.splitBlock(current, rewriter.getInsertionPoint());
  Block *failure = rewriter.createBlock(continuation->getParent(),
                                        Region::iterator(continuation));
  onFailure();
  LLVM::UnreachableOp::create(rewriter, loc);

  rewriter.setInsertionPointToEnd(current);
  LLVM::CondBrOp::create(rewriter, loc, failed, failure, continuation);
  rewriter.setInsertionPointToStart(continuation);
}

/// Base class: an LLVM conversion pattern with access to the shared state.
template <typename SourceOp>
struct JawaPattern : public ConvertOpToLLVMPattern<SourceOp> {
  JawaPattern(const LLVMTypeConverter &converter, LoweringState &state)
      : ConvertOpToLLVMPattern<SourceOp>(converter), state(state) {}

  Type convert(Type type) const {
    return this->getTypeConverter()->convertType(type);
  }

  const Runtime &runtime() const { return state.runtime; }

  LoweringState &state;
};

//===----------------------------------------------------------------------===//
// Constants
//===----------------------------------------------------------------------===//

struct ConstantLowering : public JawaPattern<ConstantOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ConstantOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOpWithNewOp<LLVM::ConstantOp>(op, op.getType(),
                                                  op.getValue());
    return success();
  }
};

struct StrConstLowering : public JawaPattern<StrConstOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(StrConstOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    LLVM::GlobalOp global = state.strings.lookup(op.getValue());
    if (!global)
      return rewriter.notifyMatchFailure(op, "string literal has no global");
    rewriter.replaceOpWithNewOp<LLVM::AddressOfOp>(op, global);
    return success();
  }
};

struct NullLowering : public JawaPattern<NullOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(NullOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOpWithNewOp<LLVM::ZeroOp>(op, ptrType(getContext()));
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Arithmetic
//===----------------------------------------------------------------------===//

template <typename SourceOp, typename IntOp, typename FloatOp>
struct ArithLowering : public JawaPattern<SourceOp> {
  using JawaPattern<SourceOp>::JawaPattern;

  LogicalResult
  matchAndRewrite(SourceOp op, typename SourceOp::Adaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    if (op.getType().isF64())
      rewriter.replaceOpWithNewOp<FloatOp>(op, adaptor.getLhs(),
                                           adaptor.getRhs());
    else
      rewriter.replaceOpWithNewOp<IntOp>(op, adaptor.getLhs(),
                                         adaptor.getRhs());
    return success();
  }
};

using AddLowering = ArithLowering<AddOp, LLVM::AddOp, LLVM::FAddOp>;
using SubLowering = ArithLowering<SubOp, LLVM::SubOp, LLVM::FSubOp>;
using MulLowering = ArithLowering<MulOp, LLVM::MulOp, LLVM::FMulOp>;

/// Integer `div` and `rem`: stop the program on a zero divisor, and give
/// `MIN / -1` (which LLVM leaves undefined) the wrapping result.
template <typename SourceOp, typename FloatOp, bool IsRem>
struct DivRemLowering : public JawaPattern<SourceOp> {
  using JawaPattern<SourceOp>::JawaPattern;

  LogicalResult
  matchAndRewrite(SourceOp op, typename SourceOp::Adaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Value lhs = adaptor.getLhs();
    Value rhs = adaptor.getRhs();
    if (op.getType().isF64()) {
      rewriter.replaceOpWithNewOp<FloatOp>(op, lhs, rhs);
      return success();
    }

    Type i64 = rewriter.getI64Type();
    APInt divisor;
    bool constant = matchPattern(op.getRhs(), m_ConstantInt(&divisor));
    if (constant && !divisor.isZero() && !divisor.isAllOnes()) {
      if (IsRem)
        rewriter.replaceOpWithNewOp<LLVM::SRemOp>(op, lhs, rhs);
      else
        rewriter.replaceOpWithNewOp<LLVM::SDivOp>(op, lhs, rhs);
      return success();
    }

    Value zero = constI64(rewriter, loc, 0);
    if (!constant || divisor.isZero()) {
      Value isZero = LLVM::ICmpOp::create(rewriter, loc,
                                          LLVM::ICmpPredicate::eq, rhs, zero);
      emitCheck(rewriter, loc, isZero, [&] {
        callRuntime(rewriter, loc, this->runtime().panicDivZero, {});
      });
    }
    Value minusOne = constI64(rewriter, loc, -1);
    Value isMinusOne = LLVM::ICmpOp::create(
        rewriter, loc, LLVM::ICmpPredicate::eq, rhs, minusOne);
    Value one = constI64(rewriter, loc, 1);
    Value safe = LLVM::SelectOp::create(rewriter, loc, isMinusOne, one, rhs);
    Value result;
    if (IsRem) {
      Value rem = LLVM::SRemOp::create(rewriter, loc, i64, lhs, safe);
      result = LLVM::SelectOp::create(rewriter, loc, isMinusOne, zero, rem);
    } else {
      Value quotient = LLVM::SDivOp::create(rewriter, loc, i64, lhs, safe);
      Value negated = LLVM::SubOp::create(rewriter, loc, i64, zero, lhs);
      result =
          LLVM::SelectOp::create(rewriter, loc, isMinusOne, negated, quotient);
    }
    rewriter.replaceOp(op, result);
    return success();
  }
};

using DivLowering = DivRemLowering<DivOp, LLVM::FDivOp, false>;
using RemLowering = DivRemLowering<RemOp, LLVM::FRemOp, true>;

struct NegLowering : public JawaPattern<NegOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(NegOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    if (op.getType().isF64()) {
      rewriter.replaceOpWithNewOp<LLVM::FNegOp>(op, adaptor.getOperand());
      return success();
    }
    Value zero = constI64(rewriter, op.getLoc(), 0);
    rewriter.replaceOpWithNewOp<LLVM::SubOp>(op, zero, adaptor.getOperand());
    return success();
  }
};

struct NotLowering : public JawaPattern<NotOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(NotOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Value one = LLVM::ConstantOp::create(rewriter, op.getLoc(),
                                         rewriter.getI1Type(), 1);
    rewriter.replaceOpWithNewOp<LLVM::XOrOp>(op, adaptor.getOperand(), one);
    return success();
  }
};

struct CmpLowering : public JawaPattern<CmpOp> {
  using JawaPattern::JawaPattern;

  static LLVM::ICmpPredicate signedPredicate(CmpPredicate predicate) {
    switch (predicate) {
    case CmpPredicate::eq:
      return LLVM::ICmpPredicate::eq;
    case CmpPredicate::ne:
      return LLVM::ICmpPredicate::ne;
    case CmpPredicate::lt:
      return LLVM::ICmpPredicate::slt;
    case CmpPredicate::le:
      return LLVM::ICmpPredicate::sle;
    case CmpPredicate::gt:
      return LLVM::ICmpPredicate::sgt;
    case CmpPredicate::ge:
      return LLVM::ICmpPredicate::sge;
    }
    llvm_unreachable("unknown predicate");
  }

  static LLVM::FCmpPredicate floatPredicate(CmpPredicate predicate) {
    // Ordered comparisons are false when either side is NaN; `ne` is the
    // exception and is true, as in IEEE 754.
    switch (predicate) {
    case CmpPredicate::eq:
      return LLVM::FCmpPredicate::oeq;
    case CmpPredicate::ne:
      return LLVM::FCmpPredicate::une;
    case CmpPredicate::lt:
      return LLVM::FCmpPredicate::olt;
    case CmpPredicate::le:
      return LLVM::FCmpPredicate::ole;
    case CmpPredicate::gt:
      return LLVM::FCmpPredicate::ogt;
    case CmpPredicate::ge:
      return LLVM::FCmpPredicate::oge;
    }
    llvm_unreachable("unknown predicate");
  }

  LogicalResult
  matchAndRewrite(CmpOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Type type = op.getLhs().getType();
    Value lhs = adaptor.getLhs();
    Value rhs = adaptor.getRhs();
    CmpPredicate predicate = op.getPredicate();

    if (type.isF64()) {
      rewriter.replaceOpWithNewOp<LLVM::FCmpOp>(op, floatPredicate(predicate),
                                                lhs, rhs);
      return success();
    }
    if (isa<StrType>(type)) {
      // Strings compare by content: equality through `str_eq`, order through
      // `str_cmp`'s -1/0/1.
      bool equality =
          predicate == CmpPredicate::eq || predicate == CmpPredicate::ne;
      Value result = callRuntime(rewriter, loc,
                                 equality ? runtime().strEq : runtime().strCmp,
                                 {lhs, rhs});
      Value zero = constI32(rewriter, loc, 0);
      LLVM::ICmpPredicate llvmPredicate =
          equality ? (predicate == CmpPredicate::eq ? LLVM::ICmpPredicate::ne
                                                    : LLVM::ICmpPredicate::eq)
                   : signedPredicate(predicate);
      rewriter.replaceOpWithNewOp<LLVM::ICmpOp>(op, llvmPredicate, result,
                                                zero);
      return success();
    }
    // Integers, booleans, and identity for arrays and structs.
    rewriter.replaceOpWithNewOp<LLVM::ICmpOp>(op, signedPredicate(predicate),
                                              lhs, rhs);
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Strings and conversions
//===----------------------------------------------------------------------===//

struct ConcatLowering : public JawaPattern<ConcatOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ConcatOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    SmallVector<Value> args;
    args.push_back(constI64(rewriter, op.getLoc(), adaptor.getParts().size()));
    llvm::append_range(args, adaptor.getParts());
    rewriter.replaceOp(
        op, callRuntime(rewriter, op.getLoc(), runtime().concat, args));
    return success();
  }
};

/// Passes a boolean to C as an `int32_t`.
Value boolToI32(OpBuilder &builder, Location loc, Value value) {
  return LLVM::ZExtOp::create(builder, loc, builder.getI32Type(), value);
}

struct ToStrLowering : public JawaPattern<ToStrOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ToStrOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Type type = op.getValue().getType();
    Value value = adaptor.getValue();
    Value result;
    if (type.isF64())
      result = callRuntime(rewriter, loc, runtime().floatToStr, value);
    else if (type.isInteger(1))
      result = callRuntime(rewriter, loc, runtime().boolToStr,
                           boolToI32(rewriter, loc, value));
    else
      result = callRuntime(rewriter, loc, runtime().intToStr, value);
    rewriter.replaceOp(op, result);
    return success();
  }
};

struct CastLowering : public JawaPattern<CastOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(CastOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Value value = adaptor.getValue();
    Type i64 = rewriter.getI64Type();
    switch (op.getKind()) {
    case CastKind::int_to_float:
      rewriter.replaceOpWithNewOp<LLVM::SIToFPOp>(op, rewriter.getF64Type(),
                                                  value);
      return success();
    case CastKind::bool_to_int:
      rewriter.replaceOpWithNewOp<LLVM::ZExtOp>(op, i64, value);
      return success();
    case CastKind::str_to_int:
      rewriter.replaceOp(op,
                         callRuntime(rewriter, loc, runtime().strToInt, value));
      return success();
    case CastKind::str_to_float:
      rewriter.replaceOp(
          op, callRuntime(rewriter, loc, runtime().strToFloat, value));
      return success();
    case CastKind::float_to_int: {
      // Truncate, saturating at the ends of the i64 range; NaN becomes 0.
      // `fptosi` is poison out of range, but the selects never pick it then.
      Type f64 = rewriter.getF64Type();
      Value raw = LLVM::FPToSIOp::create(rewriter, loc, i64, value);
      Value limit = LLVM::ConstantOp::create(
          rewriter, loc, f64, rewriter.getF64FloatAttr(9223372036854775808.0));
      Value negLimit = LLVM::ConstantOp::create(
          rewriter, loc, f64, rewriter.getF64FloatAttr(-9223372036854775808.0));
      Value tooBig = LLVM::FCmpOp::create(
          rewriter, loc, LLVM::FCmpPredicate::oge, value, limit);
      Value tooSmall = LLVM::FCmpOp::create(
          rewriter, loc, LLVM::FCmpPredicate::olt, value, negLimit);
      Value isNaN = LLVM::FCmpOp::create(
          rewriter, loc, LLVM::FCmpPredicate::uno, value, value);
      Value max = constI64(rewriter, loc, std::numeric_limits<int64_t>::max());
      Value min = constI64(rewriter, loc, std::numeric_limits<int64_t>::min());
      Value zero = constI64(rewriter, loc, 0);
      Value result = LLVM::SelectOp::create(rewriter, loc, tooBig, max, raw);
      result = LLVM::SelectOp::create(rewriter, loc, tooSmall, min, result);
      result = LLVM::SelectOp::create(rewriter, loc, isNaN, zero, result);
      rewriter.replaceOp(op, result);
      return success();
    }
    }
    llvm_unreachable("unknown cast kind");
  }
};

/// Loads the `i64` length at the start of a string or array header.
Value loadLength(OpBuilder &builder, Location loc, Value object) {
  return LLVM::LoadOp::create(builder, loc, builder.getI64Type(), object);
}

struct StrLenLowering : public JawaPattern<StrLenOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(StrLenOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOp(op, loadLength(rewriter, op.getLoc(), adaptor.getStr()));
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Arrays
//===----------------------------------------------------------------------===//

/// The address of element `index` of `array`, with elements of LLVM type
/// `elementType`.
Value elementAddress(OpBuilder &builder, Location loc, Value array,
                     Type elementType, Value index) {
  MLIRContext *context = builder.getContext();
  Value dataField = LLVM::GEPOp::create(builder, loc, ptrType(context),
                                        arrayHeaderType(context), array,
                                        ArrayRef<LLVM::GEPArg>{0, 2});
  Value data = LLVM::LoadOp::create(builder, loc, ptrType(context), dataField);
  return LLVM::GEPOp::create(builder, loc, ptrType(context), elementType, data,
                             ArrayRef<LLVM::GEPArg>{index});
}

/// Stops the program unless `0 <= index < len(array)`. One unsigned
/// comparison covers both ends, since a negative index wraps to a huge one.
void emitBoundsCheck(ConversionPatternRewriter &rewriter, Location loc,
                     const Runtime &runtime, Value array, Value index) {
  Value len = loadLength(rewriter, loc, array);
  Value outOfBounds =
      LLVM::ICmpOp::create(rewriter, loc, LLVM::ICmpPredicate::uge, index, len);
  emitCheck(rewriter, loc, outOfBounds, [&] {
    callRuntime(rewriter, loc, runtime.panicIndex, {index, len});
  });
}

struct ArrayNewLowering : public JawaPattern<ArrayNewOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ArrayNewOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Type element = op.getType().getElementType();
    Type llvmElement = convert(element);
    ValueRange elements = adaptor.getElements();
    Value array = callRuntime(
        rewriter, loc, runtime().arrayNew,
        {constI64(rewriter, loc, elements.size()),
         sizeOf(rewriter, loc, llvmElement),
         constI32(rewriter, loc, isReferenceType(element) ? 1 : 0)});
    for (auto [index, value] : llvm::enumerate(elements)) {
      Value address = elementAddress(rewriter, loc, array, llvmElement,
                                     constI64(rewriter, loc, index));
      LLVM::StoreOp::create(rewriter, loc, value, address);
    }
    rewriter.replaceOp(op, array);
    return success();
  }
};

struct ArrayLenLowering : public JawaPattern<ArrayLenOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ArrayLenOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOp(op,
                       loadLength(rewriter, op.getLoc(), adaptor.getArray()));
    return success();
  }
};

struct ArrayGetLowering : public JawaPattern<ArrayGetOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ArrayGetOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    emitBoundsCheck(rewriter, loc, runtime(), adaptor.getArray(),
                    adaptor.getIndex());
    Type element = convert(op.getType());
    Value address = elementAddress(rewriter, loc, adaptor.getArray(), element,
                                   adaptor.getIndex());
    rewriter.replaceOpWithNewOp<LLVM::LoadOp>(op, element, address);
    return success();
  }
};

struct ArraySetLowering : public JawaPattern<ArraySetOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ArraySetOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    emitBoundsCheck(rewriter, loc, runtime(), adaptor.getArray(),
                    adaptor.getIndex());
    Type element = convert(op.getValue().getType());
    Value address = elementAddress(rewriter, loc, adaptor.getArray(), element,
                                   adaptor.getIndex());
    rewriter.replaceOpWithNewOp<LLVM::StoreOp>(op, adaptor.getValue(), address);
    return success();
  }
};

struct ArrayPushLowering : public JawaPattern<ArrayPushOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(ArrayPushOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    Type element = op.getValue().getType();
    Value slot = callRuntime(
        rewriter, loc, runtime().arrayPush,
        {adaptor.getArray(), sizeOf(rewriter, loc, convert(element)),
         constI32(rewriter, loc, isReferenceType(element) ? 1 : 0)});
    rewriter.replaceOpWithNewOp<LLVM::StoreOp>(op, adaptor.getValue(), slot);
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Structs
//===----------------------------------------------------------------------===//

/// Stops the program if `object` is `kosong`.
void emitNullCheck(ConversionPatternRewriter &rewriter, Location loc,
                   const Runtime &runtime, Value object) {
  Value null =
      LLVM::ZeroOp::create(rewriter, loc, ptrType(rewriter.getContext()));
  Value isNull = LLVM::ICmpOp::create(rewriter, loc, LLVM::ICmpPredicate::eq,
                                      object, null);
  emitCheck(rewriter, loc, isNull,
            [&] { callRuntime(rewriter, loc, runtime.panicNull, {}); });
}

struct StructNewLowering : public JawaPattern<StructNewOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(StructNewOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    LLVM::LLVMStructType layout = state.structs.lookup(op.getType().getName());
    if (!layout)
      return rewriter.notifyMatchFailure(op, "unknown struct");
    // A struct with no references inside need not be scanned by the GC.
    bool hasReferences = llvm::any_of(
        op.getFields().getTypes(), [](Type t) { return isReferenceType(t); });
    Value object = callRuntime(
        rewriter, loc, hasReferences ? runtime().alloc : runtime().allocAtomic,
        sizeOf(rewriter, loc, layout));
    for (auto [index, value] : llvm::enumerate(adaptor.getFields())) {
      Value address = LLVM::GEPOp::create(
          rewriter, loc, ptrType(getContext()), layout, object,
          ArrayRef<LLVM::GEPArg>{0, static_cast<int32_t>(index)});
      LLVM::StoreOp::create(rewriter, loc, value, address);
    }
    rewriter.replaceOp(op, object);
    return success();
  }
};

/// The address of field `index` of `object`, after a null check.
FailureOr<Value> fieldAddress(ConversionPatternRewriter &rewriter, Location loc,
                              LoweringState &state, StructType type,
                              Value object, uint64_t index) {
  LLVM::LLVMStructType layout = state.structs.lookup(type.getName());
  if (!layout)
    return failure();
  emitNullCheck(rewriter, loc, state.runtime, object);
  return LLVM::GEPOp::create(
             rewriter, loc, ptrType(rewriter.getContext()), layout, object,
             ArrayRef<LLVM::GEPArg>{0, static_cast<int32_t>(index)})
      .getResult();
}

struct GetFieldLowering : public JawaPattern<GetFieldOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(GetFieldOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    FailureOr<Value> address =
        fieldAddress(rewriter, op.getLoc(), state, op.getObject().getType(),
                     adaptor.getObject(), op.getIndex());
    if (failed(address))
      return rewriter.notifyMatchFailure(op, "unknown struct");
    rewriter.replaceOpWithNewOp<LLVM::LoadOp>(op, convert(op.getType()),
                                              *address);
    return success();
  }
};

struct SetFieldLowering : public JawaPattern<SetFieldOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(SetFieldOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    FailureOr<Value> address =
        fieldAddress(rewriter, op.getLoc(), state, op.getObject().getType(),
                     adaptor.getObject(), op.getIndex());
    if (failed(address))
      return rewriter.notifyMatchFailure(op, "unknown struct");
    rewriter.replaceOpWithNewOp<LLVM::StoreOp>(op, adaptor.getValue(),
                                               *address);
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Local variables
//===----------------------------------------------------------------------===//

struct AllocaLowering : public JawaPattern<AllocaOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(AllocaOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Value one = constI64(rewriter, op.getLoc(), 1);
    rewriter.replaceOpWithNewOp<LLVM::AllocaOp>(
        op, ptrType(getContext()), convert(op.getType().getPointee()), one);
    return success();
  }
};

struct LoadLowering : public JawaPattern<LoadOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(LoadOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOpWithNewOp<LLVM::LoadOp>(op, convert(op.getType()),
                                              adaptor.getAddr());
    return success();
  }
};

struct StoreLowering : public JawaPattern<StoreOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(StoreOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOpWithNewOp<LLVM::StoreOp>(op, adaptor.getValue(),
                                               adaptor.getAddr());
    return success();
  }
};

//===----------------------------------------------------------------------===//
// Built-in functions
//===----------------------------------------------------------------------===//

struct PrintLowering : public JawaPattern<PrintOp> {
  using JawaPattern::JawaPattern;

  static PrintKind kindOf(Type type) {
    if (type.isF64())
      return PrintKind::Float;
    if (type.isInteger(1))
      return PrintKind::Bool;
    if (isa<StrType>(type))
      return PrintKind::Str;
    return PrintKind::Int;
  }

  LogicalResult
  matchAndRewrite(PrintOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    Location loc = op.getLoc();
    for (auto [index, original, value] :
         llvm::enumerate(op.getValues(), adaptor.getValues())) {
      if (index > 0)
        callRuntime(rewriter, loc, runtime().printSpace, {});
      Type type = original.getType();
      if (auto array = dyn_cast<ArrayType>(type)) {
        Value kind =
            constI32(rewriter, loc,
                     static_cast<int32_t>(kindOf(array.getElementType())));
        callRuntime(rewriter, loc, runtime().printArray, {value, kind});
        continue;
      }
      switch (kindOf(type)) {
      case PrintKind::Int:
        callRuntime(rewriter, loc, runtime().printInt, value);
        break;
      case PrintKind::Float:
        callRuntime(rewriter, loc, runtime().printFloat, value);
        break;
      case PrintKind::Bool:
        callRuntime(rewriter, loc, runtime().printBool,
                    boolToI32(rewriter, loc, value));
        break;
      case PrintKind::Str:
        callRuntime(rewriter, loc, runtime().printStr, value);
        break;
      }
    }
    callRuntime(rewriter, loc, runtime().printNewline, {});
    rewriter.eraseOp(op);
    return success();
  }
};

struct PanicLowering : public JawaPattern<PanicOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(PanicOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    callRuntime(rewriter, op.getLoc(), runtime().panic, adaptor.getMessage());
    rewriter.eraseOp(op);
    return success();
  }
};

struct UnreachableLowering : public JawaPattern<UnreachableOp> {
  using JawaPattern::JawaPattern;

  LogicalResult
  matchAndRewrite(UnreachableOp op, OpAdaptor adaptor,
                  ConversionPatternRewriter &rewriter) const override {
    rewriter.replaceOpWithNewOp<LLVM::UnreachableOp>(op);
    return success();
  }
};

//===----------------------------------------------------------------------===//
// The pass
//===----------------------------------------------------------------------===//

struct ConvertJawaToLLVMPass
    : public mlir::jawa::impl::ConvertJawaToLLVMBase<ConvertJawaToLLVMPass> {
  void runOnOperation() override {
    ModuleOp module = getOperation();
    MLIRContext *context = &getContext();

    LLVMTypeConverter converter(context);
    auto toPointer = [context](Type) -> Type { return ptrType(context); };
    converter.addConversion([=](StrType type) { return toPointer(type); });
    converter.addConversion([=](ArrayType type) { return toPointer(type); });
    converter.addConversion([=](StructType type) { return toPointer(type); });
    converter.addConversion([=](PtrType type) { return toPointer(type); });

    LoweringState state;
    if (failed(prepare(module, converter, state)))
      return signalPassFailure();

    RewritePatternSet patterns(context);
    patterns.add<ConstantLowering, StrConstLowering, NullLowering, AddLowering,
                 SubLowering, MulLowering, DivLowering, RemLowering,
                 NegLowering, NotLowering, CmpLowering, ConcatLowering,
                 ToStrLowering, CastLowering, StrLenLowering, ArrayNewLowering,
                 ArrayLenLowering, ArrayGetLowering, ArraySetLowering,
                 ArrayPushLowering, StructNewLowering, GetFieldLowering,
                 SetFieldLowering, AllocaLowering, LoadLowering, StoreLowering,
                 PrintLowering, PanicLowering, UnreachableLowering>(converter,
                                                                    state);
    populateFuncToLLVMConversionPatterns(converter, patterns);
    cf::populateControlFlowToLLVMConversionPatterns(converter, patterns);
    arith::populateArithToLLVMConversionPatterns(converter, patterns);

    LLVMConversionTarget target(*context);
    target.addLegalOp<ModuleOp, StructDefOp>();
    target.addIllegalDialect<JawaDialect, func::FuncDialect,
                             cf::ControlFlowDialect, arith::ArithDialect>();
    target.addLegalOp<StructDefOp>();
    if (failed(applyPartialConversion(module, target, std::move(patterns))))
      return signalPassFailure();

    SmallVector<UnrealizedConversionCastOp> casts;
    module.walk(
        [&](UnrealizedConversionCastOp cast) { casts.push_back(cast); });
    reconcileUnrealizedCasts(casts);

    // Struct layouts now live in the GEPs; the definitions can go.
    for (auto def : llvm::make_early_inc_range(module.getOps<StructDefOp>()))
      def.erase();

    // Only the entry point is visible outside the program, which lets LLVM
    // inline and drop the rest freely.
    for (auto function : module.getOps<LLVM::LLVMFuncOp>())
      if (!function.isExternal() && function.getName() != "jw_utama")
        function.setLinkage(LLVM::Linkage::Internal);
  }

  /// Renames functions, declares the runtime, and creates the string
  /// literal globals and struct layouts the patterns use.
  LogicalResult prepare(ModuleOp module, LLVMTypeConverter &converter,
                        LoweringState &state) {
    MLIRContext *context = module.getContext();
    OpBuilder builder(context);
    builder.setInsertionPointToStart(module.getBody());
    Location loc = module.getLoc();

    // `fib` becomes `jw_fib`, out of the way of C symbols.
    for (auto function : module.getOps<func::FuncOp>()) {
      if (function.isExternal())
        continue;
      auto name = StringAttr::get(context, "jw_" + function.getName());
      if (failed(SymbolTable::replaceAllSymbolUses(function, name, module)))
        return function.emitOpError("could not rename the function");
      SymbolTable::setSymbolName(function, name);
    }

    declareRuntime(builder, loc, state.runtime);

    for (auto def : module.getOps<StructDefOp>()) {
      SmallVector<Type> fields;
      for (unsigned i = 0, e = def.getNumFields(); i < e; ++i)
        fields.push_back(converter.convertType(def.getFieldType(i)));
      state.structs[def.getSymName()] =
          LLVM::LLVMStructType::getLiteral(context, fields);
    }

    module.walk([&](StrConstOp op) {
      StringRef text = op.getValue();
      if (state.strings.count(text))
        return;
      std::string name = ".str." + std::to_string(state.strings.size());
      state.strings[text] = createStringGlobal(builder, loc, name, text);
    });
    return success();
  }

  /// A private constant `{ i64 len, [len + 1 x i8] data }` holding `text`.
  static LLVM::GlobalOp createStringGlobal(OpBuilder &builder, Location loc,
                                           StringRef name, StringRef text) {
    MLIRContext *context = builder.getContext();
    Type i64 = builder.getI64Type();
    auto bytes = LLVM::LLVMArrayType::get(builder.getI8Type(), text.size() + 1);
    auto type = LLVM::LLVMStructType::getLiteral(context, {i64, bytes});

    OpBuilder::InsertionGuard guard(builder);
    auto global = LLVM::GlobalOp::create(
        builder, loc, type, /*isConstant=*/true, LLVM::Linkage::Private, name,
        Attribute(), /*alignment=*/8);
    Block *init = builder.createBlock(&global.getInitializerRegion());
    builder.setInsertionPointToStart(init);
    std::string data = text.str();
    data.push_back('\0');
    Value value = LLVM::UndefOp::create(builder, loc, type);
    value = LLVM::InsertValueOp::create(builder, loc, value,
                                        constI64(builder, loc, text.size()),
                                        ArrayRef<int64_t>{0});
    Value array = LLVM::ConstantOp::create(builder, loc, bytes,
                                           builder.getStringAttr(data));
    value = LLVM::InsertValueOp::create(builder, loc, value, array,
                                        ArrayRef<int64_t>{1});
    LLVM::ReturnOp::create(builder, loc, value);
    return global;
  }

  static void declareRuntime(OpBuilder &builder, Location loc,
                             Runtime &runtime) {
    MLIRContext *context = builder.getContext();
    Type ptr = ptrType(context);
    Type i64 = builder.getI64Type();
    Type i32 = builder.getI32Type();
    Type f64 = builder.getF64Type();
    Type none = LLVM::LLVMVoidType::get(context);

    auto declare = [&](StringRef name, Type result, ArrayRef<Type> params,
                       bool isVarArg = false) {
      auto type = LLVM::LLVMFunctionType::get(result, params, isVarArg);
      return LLVM::LLVMFuncOp::create(builder, loc, name, type);
    };
    auto declareNoReturn = [&](StringRef name, ArrayRef<Type> params) {
      LLVM::LLVMFuncOp function = declare(name, none, params);
      function.setPassthroughAttr(builder.getArrayAttr(
          {builder.getStringAttr("noreturn"), builder.getStringAttr("cold")}));
      return function;
    };

    runtime.alloc = declare("jawa_rt_alloc", ptr, {i64});
    runtime.allocAtomic = declare("jawa_rt_alloc_atomic", ptr, {i64});
    runtime.concat = declare("jawa_rt_str_concat", ptr, {i64}, true);
    runtime.strEq = declare("jawa_rt_str_eq", i32, {ptr, ptr});
    runtime.strCmp = declare("jawa_rt_str_cmp", i32, {ptr, ptr});
    runtime.intToStr = declare("jawa_rt_int_to_str", ptr, {i64});
    runtime.floatToStr = declare("jawa_rt_float_to_str", ptr, {f64});
    runtime.boolToStr = declare("jawa_rt_bool_to_str", ptr, {i32});
    runtime.strToInt = declare("jawa_rt_str_to_int", i64, {ptr});
    runtime.strToFloat = declare("jawa_rt_str_to_float", f64, {ptr});
    runtime.printInt = declare("jawa_rt_print_int", none, {i64});
    runtime.printFloat = declare("jawa_rt_print_float", none, {f64});
    runtime.printBool = declare("jawa_rt_print_bool", none, {i32});
    runtime.printStr = declare("jawa_rt_print_str", none, {ptr});
    runtime.printArray = declare("jawa_rt_print_array", none, {ptr, i32});
    runtime.printSpace = declare("jawa_rt_print_space", none, {});
    runtime.printNewline = declare("jawa_rt_print_newline", none, {});
    runtime.arrayNew = declare("jawa_rt_array_new", ptr, {i64, i64, i32});
    runtime.arrayPush = declare("jawa_rt_array_push", ptr, {ptr, i64, i32});
    runtime.panic = declareNoReturn("jawa_rt_panic", {ptr});
    runtime.panicIndex = declareNoReturn("jawa_rt_panic_index", {i64, i64});
    runtime.panicNull = declareNoReturn("jawa_rt_panic_null", {});
    runtime.panicDivZero = declareNoReturn("jawa_rt_panic_div_zero", {});
  }
};

} // namespace
