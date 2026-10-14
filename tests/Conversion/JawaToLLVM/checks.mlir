// RUN: jawa-opt %s -convert-jawa-to-llvm -split-input-file | FileCheck %s

// Reading an array element checks the index with one unsigned comparison
// and branches to a cold block that reports the error.
// CHECK-LABEL: llvm.func internal @jw_get
// CHECK:   %[[LEN:.*]] = llvm.load %arg0 : !llvm.ptr -> i64
// CHECK:   %[[OOB:.*]] = llvm.icmp "uge" %arg1, %[[LEN]] : i64
// CHECK:   llvm.cond_br %[[OOB]], ^[[FAIL:bb[0-9]+]], ^[[OK:bb[0-9]+]]
// CHECK: ^[[FAIL]]:
// CHECK:   llvm.call @jawa_rt_panic_index(%arg1, %[[LEN]])
// CHECK:   llvm.unreachable
// CHECK: ^[[OK]]:
// CHECK:   %[[FIELD:.*]] = llvm.getelementptr %arg0[0, 2] : (!llvm.ptr) -> !llvm.ptr, !llvm.struct<(i64, i64, ptr)>
// CHECK:   %[[DATA:.*]] = llvm.load %[[FIELD]] : !llvm.ptr -> !llvm.ptr
// CHECK:   %[[ADDR:.*]] = llvm.getelementptr %[[DATA]][%arg1] : (!llvm.ptr, i64) -> !llvm.ptr, f64
// CHECK:   llvm.load %[[ADDR]] : !llvm.ptr -> f64
func.func @get(%xs: !jawa.array<f64>, %i: i64) -> f64 {
  %0 = jawa.array_get %xs[%i] : !jawa.array<f64> -> f64
  return %0 : f64
}
func.func @utama() {
  return
}

// -----

// Integer division checks for zero and gives MIN / -1 a defined result.
// CHECK-LABEL: llvm.func internal @jw_bagi
// CHECK:   %[[ZERO:.*]] = llvm.mlir.constant(0 : i64) : i64
// CHECK:   %[[IS_ZERO:.*]] = llvm.icmp "eq" %arg1, %[[ZERO]] : i64
// CHECK:   llvm.cond_br %[[IS_ZERO]], ^[[FAIL:bb[0-9]+]], ^[[OK:bb[0-9]+]]
// CHECK: ^[[FAIL]]:
// CHECK:   llvm.call @jawa_rt_panic_div_zero()
// CHECK: ^[[OK]]:
// CHECK:   %[[M1:.*]] = llvm.icmp "eq" %arg1, %{{.*}} : i64
// CHECK:   %[[SAFE:.*]] = llvm.select %[[M1]], %{{.*}}, %arg1 : i1, i64
// CHECK:   %[[Q:.*]] = llvm.sdiv %arg0, %[[SAFE]] : i64
// CHECK:   %[[NEG:.*]] = llvm.sub %[[ZERO]], %arg0 : i64
// CHECK:   llvm.select %[[M1]], %[[NEG]], %[[Q]] : i1, i64
func.func @bagi(%a: i64, %b: i64) -> i64 {
  %0 = jawa.div %a, %b : i64
  return %0 : i64
}
func.func @utama() {
  return
}

// -----

// A known non-zero divisor needs no check at all.
// CHECK-LABEL: llvm.func internal @jw_separo
// CHECK-NOT: llvm.cond_br
// CHECK:   llvm.sdiv %arg0, %{{.*}} : i64
// CHECK-NOT: llvm.cond_br
func.func @separo(%a: i64) -> i64 {
  %two = jawa.const 2 : i64
  %0 = jawa.div %a, %two : i64
  return %0 : i64
}
func.func @utama() {
  return
}

// -----

jawa.struct @Titik {x: i64, y: f64}
jawa.struct @Simpul {regane: i64, sabanjure: !jawa.struct<"Simpul">}

// Structs without references inside come from atomic GC memory; field
// access checks for `kosong`.
// CHECK-LABEL: llvm.func internal @jw_titik
// CHECK:   %[[END:.*]] = llvm.getelementptr %{{.*}}[1] : (!llvm.ptr) -> !llvm.ptr, !llvm.struct<(i64, f64)>
// CHECK:   %[[SIZE:.*]] = llvm.ptrtoint %[[END]] : !llvm.ptr to i64
// CHECK:   %[[P:.*]] = llvm.call @jawa_rt_alloc_atomic(%[[SIZE]])
// CHECK:   llvm.getelementptr %[[P]][0, 1] : (!llvm.ptr) -> !llvm.ptr, !llvm.struct<(i64, f64)>
// CHECK:   llvm.icmp "eq" %[[P]], %{{.*}} : !llvm.ptr
// CHECK:   llvm.call @jawa_rt_panic_null()
func.func @titik(%x: i64, %y: f64) -> f64 {
  %p = jawa.struct_new(%x, %y) : (i64, f64) -> !jawa.struct<"Titik">
  %0 = jawa.get_field %p[1] : !jawa.struct<"Titik"> -> f64
  return %0 : f64
}
// A struct holding a reference is scanned by the collector.
// CHECK-LABEL: llvm.func internal @jw_simpul
// CHECK:   llvm.call @jawa_rt_alloc(%{{.*}})
func.func @simpul(%x: i64) -> !jawa.struct<"Simpul"> {
  %null = jawa.null : !jawa.struct<"Simpul">
  %p = jawa.struct_new(%x, %null) : (i64, !jawa.struct<"Simpul">) -> !jawa.struct<"Simpul">
  return %p : !jawa.struct<"Simpul">
}
// CHECK-NOT: jawa.struct
func.func @utama() {
  return
}
