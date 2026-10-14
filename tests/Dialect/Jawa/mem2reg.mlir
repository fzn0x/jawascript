// RUN: jawa-opt %s -jawa-flatten-cfg -mem2reg -canonicalize -split-input-file | FileCheck %s

// Local variable slots become SSA values once the control flow is flat:
// the loop counter and the running total turn into block arguments.

// CHECK-LABEL: func.func @sum_to
// CHECK-NOT: jawa.alloca
// CHECK-NOT: jawa.load
// CHECK-NOT: jawa.store
// CHECK: cf.br ^[[COND:bb[0-9]+]](%[[ZERO:.*]], %[[ZERO]] : i64, i64)
// CHECK: ^[[COND]](%[[TOTAL:.*]]: i64, %[[I:.*]]: i64):
// CHECK:   %[[C:.*]] = jawa.cmp lt, %[[I]], %arg0 : i64
// CHECK:   cf.cond_br %[[C]], ^[[BODY:bb[0-9]+]], ^[[EXIT:bb[0-9]+]]
// CHECK: ^[[BODY]]:
// CHECK:   %[[NEXT_TOTAL:.*]] = jawa.add %[[TOTAL]], %[[I]] : i64
// CHECK:   %[[NEXT_I:.*]] = jawa.add %[[I]], %{{.*}} : i64
// CHECK:   cf.br ^[[COND]](%[[NEXT_TOTAL]], %[[NEXT_I]] : i64, i64)
// CHECK: ^[[EXIT]]:
// CHECK:   return %[[TOTAL]] : i64
func.func @sum_to(%n: i64) -> i64 {
  %total = jawa.alloca "total" : !jawa.ptr<i64>
  %i = jawa.alloca "i" : !jawa.ptr<i64>
  %zero = jawa.const 0 : i64
  %one = jawa.const 1 : i64
  jawa.store %zero, %total : !jawa.ptr<i64>
  jawa.store %zero, %i : !jawa.ptr<i64>
  jawa.for {
    %iv = jawa.load %i : !jawa.ptr<i64>
    %c = jawa.cmp lt, %iv, %n : i64
    jawa.condition(%c)
  } do {
    %t = jawa.load %total : !jawa.ptr<i64>
    %iv = jawa.load %i : !jawa.ptr<i64>
    %sum = jawa.add %t, %iv : i64
    jawa.store %sum, %total : !jawa.ptr<i64>
    jawa.yield
  } step {
    %iv = jawa.load %i : !jawa.ptr<i64>
    %next = jawa.add %iv, %one : i64
    jawa.store %next, %i : !jawa.ptr<i64>
    jawa.yield
  }
  %result = jawa.load %total : !jawa.ptr<i64>
  jawa.return %result : i64
}

// -----

// Slots of reference type are promoted too; the resulting diamond folds
// into a select.
// CHECK-LABEL: func.func @pick
// CHECK-NOT: jawa.alloca
// CHECK: %[[S:.*]] = arith.select %arg0, %arg1, %arg2 : !jawa.str
// CHECK: return %[[S]] : !jawa.str
func.func @pick(%c: i1, %a: !jawa.str, %b: !jawa.str) -> !jawa.str {
  %slot = jawa.alloca "s" : !jawa.ptr<!jawa.str>
  jawa.store %b, %slot : !jawa.ptr<!jawa.str>
  jawa.if %c {
    jawa.store %a, %slot : !jawa.ptr<!jawa.str>
    jawa.yield
  }
  %s = jawa.load %slot : !jawa.ptr<!jawa.str>
  jawa.return %s : !jawa.str
}
