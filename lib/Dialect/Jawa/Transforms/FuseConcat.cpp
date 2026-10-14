//===- FuseConcat.cpp - Merge chains of string concatenation --------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// `"jeneng: " + jeneng + ", umur: " + umur` reaches the IR as a left-leaning
// tree of binary `jawa.concat` ops, and each one would allocate a string
// that the next one copies and throws away. This pass rewrites the tree into
// a single n-ary concat, which the runtime turns into one allocation, and
// joins neighbouring constant pieces at compile time.
//
//===----------------------------------------------------------------------===//

#include "jawa/Dialect/Jawa/IR/Jawa.h"
#include "jawa/Dialect/Jawa/Transforms/Passes.h"

#include "mlir/IR/PatternMatch.h"
#include "mlir/Transforms/GreedyPatternRewriteDriver.h"

namespace mlir::jawa {
#define GEN_PASS_DEF_JAWAFUSECONCAT
#include "jawa/Dialect/Jawa/Transforms/Passes.h.inc"
} // namespace mlir::jawa

using namespace mlir;
using namespace mlir::jawa;

namespace {

/// The text of a constant string piece, if it is one.
std::optional<StringRef> constantText(Value value) {
  if (auto constant = value.getDefiningOp<StrConstOp>())
    return constant.getValue();
  return std::nullopt;
}

/// concat(concat(a, b), c) -> concat(a, b, c), when the inner concat has no
/// other users.
struct InlineNestedConcat : public OpRewritePattern<ConcatOp> {
  using OpRewritePattern::OpRewritePattern;

  LogicalResult matchAndRewrite(ConcatOp op,
                                PatternRewriter &rewriter) const override {
    SmallVector<Value> parts;
    bool changed = false;
    for (Value part : op.getParts()) {
      auto inner = part.getDefiningOp<ConcatOp>();
      if (inner && inner->hasOneUse()) {
        llvm::append_range(parts, inner.getParts());
        changed = true;
      } else {
        parts.push_back(part);
      }
    }
    if (!changed)
      return failure();
    rewriter.replaceOpWithNewOp<ConcatOp>(op, parts);
    return success();
  }
};

/// concat("a", "b", x, "", y) -> concat("ab", x, y)
struct MergeConstantParts : public OpRewritePattern<ConcatOp> {
  using OpRewritePattern::OpRewritePattern;

  LogicalResult matchAndRewrite(ConcatOp op,
                                PatternRewriter &rewriter) const override {
    // A single part is left to the folder, which replaces the concat with it.
    if (op.getParts().size() < 2)
      return failure();

    // Only rewrite when two constants sit side by side or one is empty.
    bool mergeable = false;
    bool previousConstant = false;
    for (Value part : op.getParts()) {
      std::optional<StringRef> text = constantText(part);
      if (text && (text->empty() || previousConstant)) {
        mergeable = true;
        break;
      }
      previousConstant = text.has_value();
    }
    if (!mergeable)
      return failure();

    SmallVector<Value> parts;
    std::optional<std::string> pending;
    auto flush = [&] {
      if (!pending)
        return;
      parts.push_back(StrConstOp::create(rewriter, op.getLoc(), *pending));
      pending.reset();
    };
    for (Value part : op.getParts()) {
      if (std::optional<StringRef> text = constantText(part)) {
        if (!text->empty())
          pending = pending.value_or("") + text->str();
        continue;
      }
      flush();
      parts.push_back(part);
    }
    flush();
    if (parts.empty())
      parts.push_back(StrConstOp::create(rewriter, op.getLoc(), ""));
    rewriter.replaceOpWithNewOp<ConcatOp>(op, parts);
    return success();
  }
};

struct JawaFuseConcatPass
    : public mlir::jawa::impl::JawaFuseConcatBase<JawaFuseConcatPass> {
  void runOnOperation() override {
    RewritePatternSet patterns(&getContext());
    patterns.add<InlineNestedConcat, MergeConstantParts>(&getContext());
    if (failed(applyPatternsGreedily(getOperation(), std::move(patterns))))
      signalPassFailure();
  }
};

} // namespace
