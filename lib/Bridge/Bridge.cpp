//===- Bridge.cpp - The C++ side of the Rust/C++ bridge -------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//

#include "jawa/Bridge/Bridge.h"
#include "jawac/src/bridge.rs.h"

#include "jawa/Dialect/Jawa/IR/Jawa.h"
#include "jawa/Pipeline/Pipeline.h"

#include "mlir/Dialect/Func/IR/FuncOps.h"
#include "mlir/IR/Builders.h"
#include "mlir/IR/Diagnostics.h"
#include "mlir/IR/MLIRContext.h"
#include "mlir/IR/Verifier.h"
#include "mlir/Pass/PassManager.h"
#include "mlir/Support/FileUtilities.h"
#include "llvm/Support/ToolOutputFile.h"
#include "llvm/Support/raw_ostream.h"

#include <limits>

using namespace mlir;
using namespace mlir::jawa;

namespace jawa::bridge {

/// The handle for "no value": a call to a `suwung` function, or `bali`
/// without a value.
static constexpr std::uint32_t kNone =
    std::numeric_limits<std::uint32_t>::max();

static StringRef str(rust::Str text) {
  return StringRef(text.data(), text.size());
}

static DialectRegistry makeRegistry() {
  DialectRegistry registry;
  registerPipelineDialects(registry);
  return registry;
}

struct ModuleBuilder::Impl {
  explicit Impl(StringRef fileName)
      // jawac compiles one file at a time, so threads buy little; and MSYS2's
      // MLIR libraries race when threads create attributes concurrently.
      : context(makeRegistry(), MLIRContext::Threading::DISABLED),
        builder(&context), fileName(fileName.str()),
        location(FileLineColLoc::get(&context, fileName, 1, 1)) {
    context.loadAllAvailableDialects();
    module = ModuleOp::create(location);
    builder.setInsertionPointToEnd(module->getBody());
  }

  MLIRContext context;
  OwningOpRef<ModuleOp> module;
  OpBuilder builder;
  std::string fileName;
  Location location;

  std::vector<Type> types;
  std::vector<Value> values;

  // The struct being declared.
  std::string structName;
  SmallVector<Attribute> fieldNames;
  SmallVector<Attribute> fieldTypes;

  // The function being built, and its last stack slot.
  func::FuncOp function;
  Operation *lastSlot = nullptr;

  // The `jawa.if`, `jawa.while` and `jawa.for` operations being built.
  SmallVector<Operation *> open;

  std::string error;

  std::uint32_t addType(Type type) {
    types.push_back(type);
    return static_cast<std::uint32_t>(types.size() - 1);
  }

  std::uint32_t addValue(Value value) {
    values.push_back(value);
    return static_cast<std::uint32_t>(values.size() - 1);
  }

  Value value(std::uint32_t handle) const { return values.at(handle); }

  SmallVector<Value> valueList(rust::Slice<const std::uint32_t> handles) {
    SmallVector<Value> list;
    for (std::uint32_t handle : handles)
      list.push_back(value(handle));
    return list;
  }

  bool terminated() const {
    Block *block = builder.getInsertionBlock();
    return block && !block->empty() &&
           block->back().hasTrait<OpTrait::IsTerminator>();
  }

  /// Ends the current region with `jawa.yield` unless it already jumps away.
  void yieldIfOpen() {
    if (!terminated())
      YieldOp::create(builder, location);
  }

  /// Collects MLIR diagnostics into `error` for as long as it lives.
  std::unique_ptr<ScopedDiagnosticHandler> collectErrors() {
    error.clear();
    return std::make_unique<ScopedDiagnosticHandler>(
        &context, [this](Diagnostic &diagnostic) {
          llvm::raw_string_ostream os(error);
          if (!error.empty())
            os << '\n';
          os << diagnostic.getLocation() << ": " << diagnostic.str();
          return success();
        });
  }

  Region &loopBody(Operation *loop) {
    if (auto whileOp = llvm::dyn_cast<WhileOp>(loop))
      return whileOp.getBody();
    return llvm::cast<ForOp>(loop).getBody();
  }
};

ModuleBuilder::ModuleBuilder(rust::Str fileName)
    : impl(std::make_unique<Impl>(str(fileName))) {}

ModuleBuilder::~ModuleBuilder() = default;

std::unique_ptr<ModuleBuilder> new_module_builder(rust::Str fileName) {
  return std::make_unique<ModuleBuilder>(fileName);
}

//===----------------------------------------------------------------------===//
// Types
//===----------------------------------------------------------------------===//

std::uint32_t ModuleBuilder::int_type() {
  return impl->addType(impl->builder.getI64Type());
}

std::uint32_t ModuleBuilder::float_type() {
  return impl->addType(impl->builder.getF64Type());
}

std::uint32_t ModuleBuilder::bool_type() {
  return impl->addType(impl->builder.getI1Type());
}

std::uint32_t ModuleBuilder::str_type() {
  return impl->addType(StrType::get(&impl->context));
}

std::uint32_t ModuleBuilder::array_type(std::uint32_t element) {
  return impl->addType(ArrayType::get(&impl->context, impl->types.at(element)));
}

std::uint32_t ModuleBuilder::struct_type(rust::Str name) {
  return impl->addType(StructType::get(&impl->context, str(name)));
}

//===----------------------------------------------------------------------===//
// Structs and functions
//===----------------------------------------------------------------------===//

void ModuleBuilder::begin_struct(rust::Str name) {
  impl->structName = std::string(str(name));
  impl->fieldNames.clear();
  impl->fieldTypes.clear();
}

void ModuleBuilder::add_field(rust::Str name, std::uint32_t type) {
  impl->fieldNames.push_back(impl->builder.getStringAttr(str(name)));
  impl->fieldTypes.push_back(TypeAttr::get(impl->types.at(type)));
}

void ModuleBuilder::end_struct() {
  OpBuilder &builder = impl->builder;
  OpBuilder::InsertionGuard guard(builder);
  builder.setInsertionPointToEnd(impl->module->getBody());
  StructDefOp::create(builder, impl->location, impl->structName,
                      builder.getArrayAttr(impl->fieldNames),
                      builder.getArrayAttr(impl->fieldTypes));
}

void ModuleBuilder::declare_function(rust::Str name,
                                     rust::Slice<const std::uint32_t> params,
                                     std::uint32_t result) {
  OpBuilder &builder = impl->builder;
  SmallVector<Type> paramTypes;
  for (std::uint32_t param : params)
    paramTypes.push_back(impl->types.at(param));
  SmallVector<Type> resultTypes;
  if (result != kNone)
    resultTypes.push_back(impl->types.at(result));

  OpBuilder::InsertionGuard guard(builder);
  builder.setInsertionPointToEnd(impl->module->getBody());
  auto function =
      func::FuncOp::create(builder, impl->location, str(name),
                           builder.getFunctionType(paramTypes, resultTypes));
  // Only the entry point is public, so unused functions can be dropped.
  if (str(name) != "utama")
    function.setPrivate();
}

void ModuleBuilder::begin_function(rust::Str name) {
  impl->function = impl->module->lookupSymbol<func::FuncOp>(str(name));
  impl->lastSlot = nullptr;
  impl->open.clear();
  Block *entry = impl->function.addEntryBlock();
  impl->builder.setInsertionPointToStart(entry);
}

std::uint32_t ModuleBuilder::param(std::uint32_t index) {
  return impl->addValue(impl->function.getArgument(index));
}

void ModuleBuilder::end_function() {
  if (!impl->terminated()) {
    // The type checker guarantees a function with a result returns on every
    // path, so falling off the end can only happen in a `suwung` function.
    if (impl->function.getNumResults() == 0)
      ReturnOp::create(impl->builder, impl->location, Value());
    else
      UnreachableOp::create(impl->builder, impl->location);
  }
  impl->builder.setInsertionPointToEnd(impl->module->getBody());
}

void ModuleBuilder::set_location(std::uint32_t line, std::uint32_t column) {
  impl->location =
      FileLineColLoc::get(&impl->context, impl->fileName, line, column);
}

//===----------------------------------------------------------------------===//
// Values
//===----------------------------------------------------------------------===//

std::uint32_t ModuleBuilder::const_int(std::int64_t value) {
  OpBuilder &builder = impl->builder;
  return impl->addValue(ConstantOp::create(builder, impl->location,
                                           builder.getI64Type(),
                                           builder.getI64IntegerAttr(value)));
}

std::uint32_t ModuleBuilder::const_float(double value) {
  OpBuilder &builder = impl->builder;
  return impl->addValue(ConstantOp::create(builder, impl->location,
                                           builder.getF64Type(),
                                           builder.getF64FloatAttr(value)));
}

std::uint32_t ModuleBuilder::const_bool(bool value) {
  OpBuilder &builder = impl->builder;
  return impl->addValue(ConstantOp::create(builder, impl->location,
                                           builder.getI1Type(),
                                           builder.getBoolAttr(value)));
}

std::uint32_t ModuleBuilder::const_str(rust::Str value) {
  return impl->addValue(
      StrConstOp::create(impl->builder, impl->location, str(value)));
}

std::uint32_t ModuleBuilder::null_value(std::uint32_t type) {
  return impl->addValue(
      NullOp::create(impl->builder, impl->location, impl->types.at(type)));
}

std::uint32_t ModuleBuilder::binary(BinaryOp op, std::uint32_t lhs,
                                    std::uint32_t rhs) {
  OpBuilder &builder = impl->builder;
  Location loc = impl->location;
  Value a = impl->value(lhs);
  Value b = impl->value(rhs);
  Type type = a.getType();
  Value result;
  switch (op) {
  case BinaryOp::Add:
    result = AddOp::create(builder, loc, type, a, b);
    break;
  case BinaryOp::Sub:
    result = SubOp::create(builder, loc, type, a, b);
    break;
  case BinaryOp::Mul:
    result = MulOp::create(builder, loc, type, a, b);
    break;
  case BinaryOp::Div:
    result = DivOp::create(builder, loc, type, a, b);
    break;
  case BinaryOp::Rem:
    result = RemOp::create(builder, loc, type, a, b);
    break;
  }
  return impl->addValue(result);
}

std::uint32_t ModuleBuilder::compare(Predicate predicate, std::uint32_t lhs,
                                     std::uint32_t rhs) {
  CmpPredicate kind = CmpPredicate::eq;
  switch (predicate) {
  case Predicate::Eq:
    kind = CmpPredicate::eq;
    break;
  case Predicate::Ne:
    kind = CmpPredicate::ne;
    break;
  case Predicate::Lt:
    kind = CmpPredicate::lt;
    break;
  case Predicate::Le:
    kind = CmpPredicate::le;
    break;
  case Predicate::Gt:
    kind = CmpPredicate::gt;
    break;
  case Predicate::Ge:
    kind = CmpPredicate::ge;
    break;
  }
  OpBuilder &builder = impl->builder;
  return impl->addValue(CmpOp::create(builder, impl->location,
                                      builder.getI1Type(), kind,
                                      impl->value(lhs), impl->value(rhs)));
}

std::uint32_t ModuleBuilder::negate(std::uint32_t value) {
  Value operand = impl->value(value);
  return impl->addValue(
      NegOp::create(impl->builder, impl->location, operand.getType(), operand));
}

std::uint32_t ModuleBuilder::logical_not(std::uint32_t value) {
  Value operand = impl->value(value);
  return impl->addValue(
      NotOp::create(impl->builder, impl->location, operand.getType(), operand));
}

std::uint32_t ModuleBuilder::concat(std::uint32_t lhs, std::uint32_t rhs) {
  return impl->addValue(
      ConcatOp::create(impl->builder, impl->location,
                       ValueRange{impl->value(lhs), impl->value(rhs)}));
}

std::uint32_t ModuleBuilder::to_str(std::uint32_t value) {
  return impl->addValue(
      ToStrOp::create(impl->builder, impl->location, impl->value(value)));
}

std::uint32_t ModuleBuilder::cast(CastKind kind, std::uint32_t value) {
  OpBuilder &builder = impl->builder;
  mlir::jawa::CastKind castKind = mlir::jawa::CastKind::int_to_float;
  Type result = builder.getI64Type();
  switch (kind) {
  case CastKind::IntToFloat:
    castKind = mlir::jawa::CastKind::int_to_float;
    result = builder.getF64Type();
    break;
  case CastKind::FloatToInt:
    castKind = mlir::jawa::CastKind::float_to_int;
    break;
  case CastKind::BoolToInt:
    castKind = mlir::jawa::CastKind::bool_to_int;
    break;
  case CastKind::StrToInt:
    castKind = mlir::jawa::CastKind::str_to_int;
    break;
  case CastKind::StrToFloat:
    castKind = mlir::jawa::CastKind::str_to_float;
    result = builder.getF64Type();
    break;
  }
  return impl->addValue(CastOp::create(builder, impl->location, result,
                                       castKind, impl->value(value)));
}

std::uint32_t ModuleBuilder::str_len(std::uint32_t value) {
  OpBuilder &builder = impl->builder;
  return impl->addValue(StrLenOp::create(
      builder, impl->location, builder.getI64Type(), impl->value(value)));
}

//===----------------------------------------------------------------------===//
// Local variables
//===----------------------------------------------------------------------===//

std::uint32_t ModuleBuilder::local(std::uint32_t type, rust::Str name) {
  // Every slot lives at the top of the entry block, so a variable declared
  // in a loop body does not grow the stack on each iteration.
  OpBuilder &builder = impl->builder;
  OpBuilder::InsertionGuard guard(builder);
  if (impl->lastSlot)
    builder.setInsertionPointAfter(impl->lastSlot);
  else
    builder.setInsertionPointToStart(&impl->function.getBody().front());
  auto slot = AllocaOp::create(
      builder, impl->location,
      PtrType::get(&impl->context, impl->types.at(type)), str(name));
  impl->lastSlot = slot;
  return impl->addValue(slot);
}

std::uint32_t ModuleBuilder::load(std::uint32_t slot) {
  Value address = impl->value(slot);
  Type type = llvm::cast<PtrType>(address.getType()).getPointee();
  return impl->addValue(
      LoadOp::create(impl->builder, impl->location, type, address));
}

void ModuleBuilder::store(std::uint32_t value, std::uint32_t slot) {
  StoreOp::create(impl->builder, impl->location, impl->value(value),
                  impl->value(slot));
}

//===----------------------------------------------------------------------===//
// Calls and built-in functions
//===----------------------------------------------------------------------===//

std::uint32_t ModuleBuilder::call(rust::Str callee,
                                  rust::Slice<const std::uint32_t> args) {
  auto function = impl->module->lookupSymbol<func::FuncOp>(str(callee));
  auto call = func::CallOp::create(impl->builder, impl->location, function,
                                   impl->valueList(args));
  if (call.getNumResults() == 0)
    return kNone;
  return impl->addValue(call.getResult(0));
}

void ModuleBuilder::print(rust::Slice<const std::uint32_t> values) {
  PrintOp::create(impl->builder, impl->location, impl->valueList(values));
}

void ModuleBuilder::panic(std::uint32_t message) {
  PanicOp::create(impl->builder, impl->location, impl->value(message));
}

//===----------------------------------------------------------------------===//
// Arrays and structs
//===----------------------------------------------------------------------===//

std::uint32_t
ModuleBuilder::array_new(std::uint32_t type,
                         rust::Slice<const std::uint32_t> elements) {
  return impl->addValue(ArrayNewOp::create(impl->builder, impl->location,
                                           impl->types.at(type),
                                           impl->valueList(elements)));
}

std::uint32_t ModuleBuilder::array_len(std::uint32_t array) {
  OpBuilder &builder = impl->builder;
  return impl->addValue(ArrayLenOp::create(
      builder, impl->location, builder.getI64Type(), impl->value(array)));
}

std::uint32_t ModuleBuilder::array_get(std::uint32_t array,
                                       std::uint32_t index) {
  Value arrayValue = impl->value(array);
  Type element = llvm::cast<ArrayType>(arrayValue.getType()).getElementType();
  return impl->addValue(ArrayGetOp::create(
      impl->builder, impl->location, element, arrayValue, impl->value(index)));
}

void ModuleBuilder::array_set(std::uint32_t array, std::uint32_t index,
                              std::uint32_t value) {
  ArraySetOp::create(impl->builder, impl->location, impl->value(array),
                     impl->value(index), impl->value(value));
}

void ModuleBuilder::array_push(std::uint32_t array, std::uint32_t value) {
  ArrayPushOp::create(impl->builder, impl->location, impl->value(array),
                      impl->value(value));
}

std::uint32_t
ModuleBuilder::struct_new(std::uint32_t type,
                          rust::Slice<const std::uint32_t> fields) {
  return impl->addValue(StructNewOp::create(impl->builder, impl->location,
                                            impl->types.at(type),
                                            impl->valueList(fields)));
}

std::uint32_t ModuleBuilder::get_field(std::uint32_t object,
                                       std::uint32_t index) {
  Value objectValue = impl->value(object);
  auto type = llvm::cast<StructType>(objectValue.getType());
  auto def = impl->module->lookupSymbol<StructDefOp>(type.getName());
  return impl->addValue(GetFieldOp::create(impl->builder, impl->location,
                                           def.getFieldType(index), objectValue,
                                           index));
}

void ModuleBuilder::set_field(std::uint32_t object, std::uint32_t index,
                              std::uint32_t value) {
  SetFieldOp::create(impl->builder, impl->location, impl->value(object), index,
                     impl->value(value));
}

//===----------------------------------------------------------------------===//
// Structured control flow
//===----------------------------------------------------------------------===//

void ModuleBuilder::begin_if(std::uint32_t condition) {
  auto op = IfOp::create(impl->builder, impl->location, impl->value(condition));
  impl->open.push_back(op);
  impl->builder.createBlock(&op.getThenRegion());
}

void ModuleBuilder::begin_else() {
  auto op = llvm::cast<IfOp>(impl->open.back());
  impl->yieldIfOpen();
  impl->builder.createBlock(&op.getElseRegion());
}

void ModuleBuilder::end_if() {
  impl->yieldIfOpen();
  impl->builder.setInsertionPointAfter(impl->open.pop_back_val());
}

void ModuleBuilder::begin_while() {
  auto op = WhileOp::create(impl->builder, impl->location);
  impl->open.push_back(op);
  impl->builder.createBlock(&op.getCond());
}

void ModuleBuilder::begin_for() {
  auto op = ForOp::create(impl->builder, impl->location);
  impl->open.push_back(op);
  impl->builder.createBlock(&op.getCond());
}

void ModuleBuilder::begin_loop_body(std::uint32_t condition) {
  ConditionOp::create(impl->builder, impl->location, impl->value(condition));
  impl->builder.createBlock(&impl->loopBody(impl->open.back()));
}

void ModuleBuilder::begin_for_step() {
  auto op = llvm::cast<ForOp>(impl->open.back());
  impl->yieldIfOpen();
  impl->builder.createBlock(&op.getStep());
}

void ModuleBuilder::end_while() {
  impl->yieldIfOpen();
  impl->builder.setInsertionPointAfter(impl->open.pop_back_val());
}

void ModuleBuilder::end_for() {
  impl->yieldIfOpen();
  impl->builder.setInsertionPointAfter(impl->open.pop_back_val());
}

void ModuleBuilder::emit_break() {
  BreakOp::create(impl->builder, impl->location);
}

void ModuleBuilder::emit_continue() {
  ContinueOp::create(impl->builder, impl->location);
}

void ModuleBuilder::emit_return(std::uint32_t value) {
  ReturnOp::create(impl->builder, impl->location,
                   value == kNone ? Value() : impl->value(value));
}

bool ModuleBuilder::is_terminated() const { return impl->terminated(); }

//===----------------------------------------------------------------------===//
// Output
//===----------------------------------------------------------------------===//

bool ModuleBuilder::verify() {
  auto errors = impl->collectErrors();
  return succeeded(mlir::verify(*impl->module));
}

bool ModuleBuilder::emit(Emit kind, std::uint32_t optLevel, rust::Str path) {
  auto errors = impl->collectErrors();
  ModuleOp module = *impl->module;
  if (failed(mlir::verify(module)))
    return false;

  // `-` writes to standard output.
  auto open = [&]() -> std::unique_ptr<llvm::ToolOutputFile> {
    std::string message;
    auto file = openOutputFile(str(path), &message);
    if (!file)
      impl->error = message;
    return file;
  };

  if (kind == Emit::JawaIr) {
    auto file = open();
    if (!file)
      return false;
    module.print(file->os());
    file->keep();
    return true;
  }

  PassManager passes(&impl->context);
  if (kind == Emit::OptimizedIr)
    buildJawaOptimizationPipeline(passes, optLevel);
  else
    buildJawaToLLVMPipeline(passes, optLevel);
  if (failed(passes.run(module)))
    return false;

  if (kind == Emit::OptimizedIr || kind == Emit::LlvmDialect) {
    auto file = open();
    if (!file)
      return false;
    module.print(file->os());
    file->keep();
    return true;
  }

  llvm::LLVMContext llvmContext;
  std::unique_ptr<llvm::Module> llvmModule =
      translateToLLVMIR(module, llvmContext, optLevel, impl->error);
  if (!llvmModule)
    return false;
  llvmModule->setSourceFileName(impl->fileName);

  if (kind == Emit::LlvmIr) {
    auto file = open();
    if (!file)
      return false;
    llvmModule->print(file->os(), nullptr);
    file->keep();
    return true;
  }
  return succeeded(
      writeObjectFile(*llvmModule, str(path), optLevel, impl->error));
}

rust::String ModuleBuilder::error_message() const {
  return rust::String(impl->error);
}

} // namespace jawa::bridge
