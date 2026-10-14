// RUN: jawa-opt %s -jawa-fuse-concat -split-input-file | FileCheck %s

// "jeneng: " + jeneng + ", umur: " + ukara(umur) builds one string, not three.
// CHECK-LABEL: func.func @chain
// CHECK-DAG: %[[A:.*]] = jawa.str_const "jeneng: "
// CHECK-DAG: %[[B:.*]] = jawa.str_const ", umur: "
// CHECK-DAG: %[[N:.*]] = jawa.to_str %arg1 : i64
// CHECK: %[[R:.*]] = jawa.concat %[[A]], %arg0, %[[B]], %[[N]]
// CHECK-NOT: jawa.concat
// CHECK: return %[[R]]
func.func @chain(%name: !jawa.str, %age: i64) -> !jawa.str {
  %0 = jawa.str_const "jeneng: "
  %1 = jawa.concat %0, %name
  %2 = jawa.str_const ", umur: "
  %3 = jawa.concat %1, %2
  %4 = jawa.to_str %age : i64
  %5 = jawa.concat %3, %4
  return %5 : !jawa.str
}

// -----

// Neighbouring constants are joined and empty ones dropped.
// CHECK-LABEL: func.func @constants
// CHECK: %[[AB:.*]] = jawa.str_const "sugeng rawuh"
// CHECK: %[[R:.*]] = jawa.concat %[[AB]], %arg0
// CHECK: return %[[R]]
func.func @constants(%x: !jawa.str) -> !jawa.str {
  %0 = jawa.str_const "sugeng "
  %1 = jawa.str_const "rawuh"
  %2 = jawa.str_const ""
  %3 = jawa.concat %0, %1, %2, %x
  return %3 : !jawa.str
}

// -----

// A concat that is used elsewhere keeps its own string.
// CHECK-LABEL: func.func @shared
// CHECK: %[[INNER:.*]] = jawa.concat %arg0, %arg1
// CHECK: %[[OUTER:.*]] = jawa.concat %[[INNER]], %arg0
// CHECK: return %[[INNER]], %[[OUTER]]
func.func @shared(%a: !jawa.str, %b: !jawa.str) -> (!jawa.str, !jawa.str) {
  %0 = jawa.concat %a, %b
  %1 = jawa.concat %0, %a
  return %0, %1 : !jawa.str, !jawa.str
}

// -----

// Only constants: the whole expression becomes one constant.
// CHECK-LABEL: func.func @all_constant
// CHECK: %[[S:.*]] = jawa.str_const "aku jawa"
// CHECK-NOT: jawa.concat
// CHECK: return %[[S]]
func.func @all_constant() -> !jawa.str {
  %0 = jawa.str_const "aku"
  %1 = jawa.str_const " "
  %2 = jawa.str_const "jawa"
  %3 = jawa.concat %0, %1
  %4 = jawa.concat %3, %2
  return %4 : !jawa.str
}
