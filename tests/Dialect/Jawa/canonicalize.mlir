// RUN: jawa-opt %s -canonicalize -split-input-file | FileCheck %s

// CHECK-LABEL: func.func @fold_arithmetic
// CHECK-NEXT: %[[C50:.*]] = jawa.const 50 : i64
// CHECK-NEXT: %[[R:.*]] = jawa.add %arg0, %[[C50]] : i64
// CHECK-NEXT: jawa.return %[[R]]
func.func @fold_arithmetic(%x: i64) -> i64 {
  %0 = jawa.const 0 : i64
  %1 = jawa.const 6 : i64
  %2 = jawa.const 7 : i64
  %3 = jawa.const 8 : i64
  %4 = jawa.mul %1, %2 : i64
  %5 = jawa.add %4, %3 : i64
  %6 = jawa.add %x, %0 : i64
  %7 = jawa.mul %6, %5 : i64
  %8 = jawa.div %7, %5 : i64
  %one = jawa.const 1 : i64
  %9 = jawa.div %x, %one : i64
  %10 = jawa.add %9, %5 : i64
  jawa.return %10 : i64
}

// -----

// Integer division wraps for MIN / -1 instead of being undefined.
// CHECK-LABEL: func.func @fold_division
// CHECK-DAG: jawa.const -9223372036854775808 : i64
// CHECK-DAG: jawa.const 0 : i64
// CHECK-DAG: jawa.const -1 : i64
// CHECK-DAG: jawa.const 2.500000e+00 : f64
func.func @fold_division() -> (i64, i64, i64, f64) {
  %min = jawa.const -9223372036854775808 : i64
  %m1 = jawa.const -1 : i64
  %q = jawa.div %min, %m1 : i64
  %r = jawa.rem %min, %m1 : i64
  %seven = jawa.const -7 : i64
  %two = jawa.const 2 : i64
  // Remainder takes the sign of the dividend: -7 % 2 == -1.
  %r2 = jawa.rem %seven, %two : i64
  %a = jawa.const 5.0 : f64
  %b = jawa.const 2.0 : f64
  %f = jawa.div %a, %b : f64
  return %q, %r, %r2, %f : i64, i64, i64, f64
}

// -----

// Division by zero stops the program at runtime, so it is neither folded nor
// removed when unused. Division by a non-zero constant is removed.
// CHECK-LABEL: func.func @keep_trapping_division
// CHECK: jawa.div %arg0, %{{.*}} : i64
// CHECK-NOT: jawa.div
// CHECK-NOT: jawa.rem
func.func @keep_trapping_division(%x: i64) {
  %zero = jawa.const 0 : i64
  %three = jawa.const 3 : i64
  %0 = jawa.div %x, %zero : i64
  %1 = jawa.div %x, %three : i64
  %2 = jawa.rem %x, %three : i64
  jawa.return
}

// -----

// CHECK-LABEL: func.func @fold_strings
// CHECK-DAG: %[[S:.*]] = jawa.str_const "umur: 42"
// CHECK-DAG: %[[L:.*]] = jawa.const 8 : i64
// CHECK: jawa.print %[[S]], %[[L]]
func.func @fold_strings() {
  %0 = jawa.str_const "umur: "
  %1 = jawa.const 42 : i64
  %2 = jawa.to_str %1 : i64
  %3 = jawa.concat %0, %2
  %4 = jawa.str_len %3
  jawa.print %3, %4 : !jawa.str, i64
  jawa.return
}

// -----

// CHECK-LABEL: func.func @fold_booleans
// CHECK-DAG: %[[T:.*]] = jawa.const true
// CHECK-DAG: %[[F:.*]] = jawa.const false
// CHECK-DAG: %[[S:.*]] = jawa.str_const "bener"
// CHECK: jawa.print %arg0, %[[T]], %[[F]], %[[S]]
func.func @fold_booleans(%c: i1) {
  %n = jawa.not %c
  %nn = jawa.not %n
  %a = jawa.const 1 : i64
  %b = jawa.const 2 : i64
  %lt = jawa.cmp lt, %a, %b : i64
  %s1 = jawa.str_const "a"
  %s2 = jawa.str_const "b"
  %gt = jawa.cmp gt, %s1, %s2 : !jawa.str
  %text = jawa.to_str %lt : i1
  jawa.print %nn, %lt, %gt, %text : i1, i1, i1, !jawa.str
  jawa.return
}

// -----

// A value equals itself, except a float that might be NaN.
// CHECK-LABEL: func.func @self_comparison
// CHECK: %[[T:.*]] = jawa.const true
// CHECK: %[[F:.*]] = jawa.cmp eq, %arg1, %arg1 : f64
// CHECK: jawa.print %[[T]], %[[F]]
func.func @self_comparison(%x: i64, %y: f64) {
  %0 = jawa.cmp eq, %x, %x : i64
  %1 = jawa.cmp eq, %y, %y : f64
  jawa.print %0, %1 : i1, i1
  jawa.return
}

// -----

// CHECK-LABEL: func.func @fold_casts
// CHECK-DAG: jawa.const 3.000000e+00 : f64
// CHECK-DAG: jawa.const -2 : i64
// CHECK-DAG: jawa.const 9223372036854775807 : i64
// CHECK-DAG: jawa.const 1 : i64
// CHECK: jawa.cast str_to_int
func.func @fold_casts() -> (f64, i64, i64, i64, i64) {
  %three = jawa.const 3 : i64
  %0 = jawa.cast int_to_float %three : i64 -> f64
  %f = jawa.const -2.9 : f64
  %1 = jawa.cast float_to_int %f : f64 -> i64
  %big = jawa.const 1.0e300 : f64
  %2 = jawa.cast float_to_int %big : f64 -> i64
  %t = jawa.const true
  %3 = jawa.cast bool_to_int %t : i1 -> i64
  %s = jawa.str_const "12"
  %4 = jawa.cast str_to_int %s : !jawa.str -> i64
  return %0, %1, %2, %3, %4 : f64, i64, i64, i64, i64
}
