//===- JawaMemorySlot.cpp - mem2reg support for local variables -----------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// Implements the memory slot interfaces on `jawa.alloca`, `jawa.load` and
// `jawa.store`, so MLIR's generic `mem2reg` pass can turn local variables
// into SSA values once the control flow has been flattened.
//
//===----------------------------------------------------------------------===//

#include "jawa/Dialect/Jawa/IR/Jawa.h"

#include "mlir/IR/Builders.h"

using namespace mlir;
using namespace mlir::jawa;

//===----------------------------------------------------------------------===//
// AllocaOp
//===----------------------------------------------------------------------===//

SmallVector<MemorySlot> AllocaOp::getPromotableSlots() {
  return {MemorySlot{getAddr(), getAddr().getType().getPointee()}};
}

Value AllocaOp::getDefaultValue(const MemorySlot &slot, OpBuilder &builder) {
  // Jawa variables always start with a value, so the default is only seen on
  // paths that never read the variable. Any value of the right type works.
  Type type = slot.elemType;
  if (isa<IntegerType>(type))
    return ConstantOp::create(builder, getLoc(), type,
                              builder.getIntegerAttr(type, 0));
  if (isa<FloatType>(type))
    return ConstantOp::create(builder, getLoc(), type,
                              builder.getFloatAttr(type, 0.0));
  return NullOp::create(builder, getLoc(), type);
}

void AllocaOp::handleBlockArgument(const MemorySlot &, BlockArgument,
                                   OpBuilder &) {}

std::optional<PromotableAllocationOpInterface>
AllocaOp::handlePromotionComplete(const MemorySlot &, Value defaultValue,
                                  OpBuilder &) {
  if (defaultValue && defaultValue.use_empty())
    defaultValue.getDefiningOp()->erase();
  erase();
  return std::nullopt;
}

//===----------------------------------------------------------------------===//
// LoadOp
//===----------------------------------------------------------------------===//

bool LoadOp::loadsFrom(const MemorySlot &slot) { return getAddr() == slot.ptr; }

bool LoadOp::storesTo(const MemorySlot &) { return false; }

Value LoadOp::getStored(const MemorySlot &, OpBuilder &, Value,
                        const DataLayout &) {
  llvm_unreachable("jawa.load does not store");
}

bool LoadOp::canUsesBeRemoved(const MemorySlot &slot,
                              const SmallPtrSetImpl<OpOperand *> &blockingUses,
                              SmallVectorImpl<OpOperand *> &,
                              const DataLayout &) {
  if (blockingUses.size() != 1)
    return false;
  Value used = (*blockingUses.begin())->get();
  return used == slot.ptr && getAddr() == slot.ptr &&
         getType() == slot.elemType;
}

DeletionKind LoadOp::removeBlockingUses(const MemorySlot &,
                                        const SmallPtrSetImpl<OpOperand *> &,
                                        OpBuilder &, Value reachingDefinition,
                                        const DataLayout &) {
  getResult().replaceAllUsesWith(reachingDefinition);
  return DeletionKind::Delete;
}

//===----------------------------------------------------------------------===//
// StoreOp
//===----------------------------------------------------------------------===//

bool StoreOp::loadsFrom(const MemorySlot &) { return false; }

bool StoreOp::storesTo(const MemorySlot &slot) { return getAddr() == slot.ptr; }

Value StoreOp::getStored(const MemorySlot &, OpBuilder &, Value,
                         const DataLayout &) {
  return getValue();
}

bool StoreOp::canUsesBeRemoved(const MemorySlot &slot,
                               const SmallPtrSetImpl<OpOperand *> &blockingUses,
                               SmallVectorImpl<OpOperand *> &,
                               const DataLayout &) {
  if (blockingUses.size() != 1)
    return false;
  Value used = (*blockingUses.begin())->get();
  // Storing the slot's own address somewhere would let it escape.
  return used == slot.ptr && getAddr() == slot.ptr && getValue() != slot.ptr &&
         getValue().getType() == slot.elemType;
}

DeletionKind StoreOp::removeBlockingUses(const MemorySlot &,
                                         const SmallPtrSetImpl<OpOperand *> &,
                                         OpBuilder &, Value,
                                         const DataLayout &) {
  return DeletionKind::Delete;
}
