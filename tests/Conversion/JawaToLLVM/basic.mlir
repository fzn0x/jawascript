// RUN: jawa-opt %s -convert-jawa-to-llvm -split-input-file | FileCheck %s

// Functions get a `jw_` prefix; only the entry point stays external.
// CHECK: llvm.func @jawa_rt_print_int(i64)
// CHECK: llvm.func @jawa_rt_panic_index(i64, i64) attributes {passthrough = ["noreturn", "cold"]}
// CHECK-LABEL: llvm.func internal @jw_tambah(%arg0: i64, %arg1: i64) -> i64
// CHECK:   %[[R:.*]] = llvm.add %arg0, %arg1 : i64
// CHECK:   llvm.return %[[R]] : i64
// CHECK-LABEL: llvm.func @jw_utama()
// CHECK:   llvm.call @jw_tambah
// CHECK:   llvm.call @jawa_rt_print_int
// CHECK:   llvm.call @jawa_rt_print_newline
func.func @tambah(%a: i64, %b: i64) -> i64 {
  %0 = jawa.add %a, %b : i64
  return %0 : i64
}
func.func @utama() {
  %0 = jawa.const 1 : i64
  %1 = func.call @tambah(%0, %0) : (i64, i64) -> i64
  jawa.print %1 : i64
  return
}

// -----

// A string literal is a private constant laid out like the runtime's jawa_str.
// CHECK: llvm.mlir.global private constant @".str.0"() {addr_space = 0 : i32, alignment = 8 : i64} : !llvm.struct<(i64, array<5 x i8>)>
// CHECK:   llvm.mlir.constant(4 : i64) : i64
// CHECK:   llvm.mlir.constant("halo\00") : !llvm.array<5 x i8>
// CHECK-LABEL: llvm.func @jw_utama()
// CHECK:   %[[S:.*]] = llvm.mlir.addressof @".str.0" : !llvm.ptr
// CHECK:   llvm.call @jawa_rt_print_str(%[[S]])
func.func @utama() {
  %0 = jawa.str_const "halo"
  jawa.print %0 : !jawa.str
  return
}

// -----

// Concatenation is one variadic runtime call, whatever the number of parts.
// CHECK-LABEL: llvm.func internal @jw_join
// CHECK:   %[[N:.*]] = llvm.mlir.constant(3 : i64) : i64
// CHECK:   llvm.call @jawa_rt_str_concat(%[[N]], %arg0, %arg1, %arg0) vararg(!llvm.func<ptr (i64, ...)>)
func.func @join(%a: !jawa.str, %b: !jawa.str) -> !jawa.str {
  %0 = jawa.concat %a, %b, %a
  return %0 : !jawa.str
}
func.func @utama() {
  return
}

// -----

// Strings compare by content.
// CHECK-LABEL: llvm.func internal @jw_before
// CHECK:   %[[C:.*]] = llvm.call @jawa_rt_str_cmp(%arg0, %arg1) : (!llvm.ptr, !llvm.ptr) -> i32
// CHECK:   llvm.icmp "slt" %[[C]], %{{.*}} : i32
func.func @before(%a: !jawa.str, %b: !jawa.str) -> i1 {
  %0 = jawa.cmp lt, %a, %b : !jawa.str
  return %0 : i1
}
func.func @utama() {
  return
}
