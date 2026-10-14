//===- FlattenCFG.cpp - Structured control flow to branches ---------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// Turns `jawa.if`, `jawa.while` and `jawa.for` into blocks joined by `cf`
// branches. For a `jawa.for`:
//
//            ^pre:  ...  cf.br ^cond
//            ^cond: ...  cf.cond_br %c, ^body, ^exit
//            ^body: ...  cf.br ^step      (jawa.yield, jawa.continue)
//                        cf.br ^exit      (jawa.break)
//            ^step: ...  cf.br ^cond
//            ^exit: ...
//
// A `jawa.while` is the same without `^step`: its latch is `^cond`.
//
//===----------------------------------------------------------------------===//

#include "jawa/Dialect/Jawa/IR/Jawa.h"
#include "jawa/Dialect/Jawa/Transforms/Passes.h"

#include "mlir/Dialect/ControlFlow/IR/ControlFlowOps.h"
#include "mlir/IR/PatternMatch.h"

namespace mlir::jawa {
#define GEN_PASS_DEF_JAWAFLATTENCFG
#include "jawa/Dialect/Jawa/Transforms/Passes.h.inc"
} // namespace mlir::jawa

using namespace mlir;
using namespace mlir::jawa;

namespace {

/// Moves the blocks of `region` before `before` and returns them in order.
SmallVector<Block *> inlineBlocks(RewriterBase &rewriter, Region &region,
                                  Block *before) {
  SmallVector<Block *> blocks;
  for (Block &block : region)
    blocks.push_back(&block);
  rewriter.inlineRegionBefore(region, before);
  return blocks;
}

/// Replaces a terminator with an unconditional branch to `dest`.
void branchTo(RewriterBase &rewriter, Operation *terminator, Block *dest) {
  rewriter.setInsertionPoint(terminator);
  rewriter.replaceOpWithNewOp<cf::BranchOp>(terminator, dest);
}

/// Splits the block holding `op` right after it; the rest of the block,
/// including its terminator, moves to the returned continuation block.
Block *splitAfter(RewriterBase &rewriter, Operation *op) {
  return rewriter.splitBlock(op->getBlock(), std::next(op->getIterator()));
}

void flattenIf(RewriterBase &rewriter, IfOp op) {
  Block *continuation = splitAfter(rewriter, op);
  Block *thenEntry = &op.getThenRegion().front();
  Block *elseEntry =
      op.getElseRegion().empty() ? continuation : &op.getElseRegion().front();

  SmallVector<Block *> blocks =
      inlineBlocks(rewriter, op.getThenRegion(), continuation);
  blocks.append(inlineBlocks(rewriter, op.getElseRegion(), continuation));

  // Inner operations are already flattened, so every `jawa.yield` left in
  // these blocks belongs to this `if`. `break`, `continue` and `return`
  // belong to something further out and stay for now.
  for (Block *block : blocks)
    if (auto yield = dyn_cast<YieldOp>(block->getTerminator()))
      branchTo(rewriter, yield, continuation);

  rewriter.setInsertionPoint(op);
  rewriter.replaceOpWithNewOp<cf::CondBranchOp>(
      op, op.getCondition(), thenEntry, ValueRange(), elseEntry, ValueRange());
}

void flattenLoop(RewriterBase &rewriter, Operation *op, Region &cond,
                 Region &body, Region *step) {
  Block *exit = splitAfter(rewriter, op);
  Block *condEntry = &cond.front();
  Block *bodyEntry = &body.front();
  Block *latch = step ? &step->front() : condEntry;

  SmallVector<Block *> condBlocks = inlineBlocks(rewriter, cond, exit);
  SmallVector<Block *> bodyBlocks = inlineBlocks(rewriter, body, exit);
  SmallVector<Block *> stepBlocks;
  if (step)
    stepBlocks = inlineBlocks(rewriter, *step, exit);

  for (Block *block : condBlocks) {
    if (auto condition = dyn_cast<ConditionOp>(block->getTerminator())) {
      rewriter.setInsertionPoint(condition);
      rewriter.replaceOpWithNewOp<cf::CondBranchOp>(
          condition, condition.getCondition(), bodyEntry, ValueRange(), exit,
          ValueRange());
    }
  }
  for (Block *block : bodyBlocks) {
    Operation *terminator = block->getTerminator();
    if (isa<YieldOp, ContinueOp>(terminator))
      branchTo(rewriter, terminator, latch);
    else if (isa<BreakOp>(terminator))
      branchTo(rewriter, terminator, exit);
  }
  for (Block *block : stepBlocks)
    if (auto yield = dyn_cast<YieldOp>(block->getTerminator()))
      branchTo(rewriter, yield, condEntry);

  rewriter.setInsertionPoint(op);
  rewriter.replaceOpWithNewOp<cf::BranchOp>(op, condEntry);
}

struct JawaFlattenCFGPass
    : public mlir::jawa::impl::JawaFlattenCFGBase<JawaFlattenCFGPass> {
  void runOnOperation() override {
    func::FuncOp function = getOperation();
    IRRewriter rewriter(&getContext());

    // Innermost first: post-order visits nested operations before the
    // operations holding them.
    SmallVector<Operation *> structured;
    function.walk<WalkOrder::PostOrder>([&](Operation *op) {
      if (isa<IfOp, WhileOp, ForOp>(op))
        structured.push_back(op);
    });

    for (Operation *op : structured) {
      if (auto ifOp = dyn_cast<IfOp>(op))
        flattenIf(rewriter, ifOp);
      else if (auto whileOp = dyn_cast<WhileOp>(op))
        flattenLoop(rewriter, op, whileOp.getCond(), whileOp.getBody(),
                    /*step=*/nullptr);
      else if (auto forOp = dyn_cast<ForOp>(op))
        flattenLoop(rewriter, op, forOp.getCond(), forOp.getBody(),
                    &forOp.getStep());
    }

    // Every `jawa.return` now sits directly in the function body.
    function.walk([&](ReturnOp ret) {
      rewriter.setInsertionPoint(ret);
      rewriter.replaceOpWithNewOp<func::ReturnOp>(ret, ret->getOperands());
    });
  }
};

} // namespace
