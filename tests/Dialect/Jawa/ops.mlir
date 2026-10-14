// RUN: jawa-opt %s | jawa-opt | FileCheck %s

// Every operation survives a print/parse round trip.

// CHECK: jawa.struct @Titik {x: i64, y: f64}
jawa.struct @Titik {x: i64, y: f64}
// CHECK: jawa.struct @Simpul {regane: i64, sabanjure: !jawa.struct<"Simpul">}
jawa.struct @Simpul {regane: i64, sabanjure: !jawa.struct<"Simpul">}

// CHECK-LABEL: func.func @scalars
func.func @scalars(%a: i64, %b: f64, %c: i1) -> i64 {
  // CHECK: jawa.const 42 : i64
  %0 = jawa.const 42 : i64
  // CHECK: jawa.add %{{.*}}, %{{.*}} : i64
  %1 = jawa.add %a, %0 : i64
  // CHECK: jawa.sub
  %2 = jawa.sub %1, %a : i64
  // CHECK: jawa.mul
  %3 = jawa.mul %2, %2 : i64
  // CHECK: jawa.div
  %4 = jawa.div %3, %0 : i64
  // CHECK: jawa.rem
  %5 = jawa.rem %4, %0 : i64
  // CHECK: jawa.neg %{{.*}} : f64
  %6 = jawa.neg %b : f64
  // CHECK: jawa.not %{{.*}}
  %7 = jawa.not %c
  // CHECK: jawa.cmp ge, %{{.*}}, %{{.*}} : f64
  %8 = jawa.cmp ge, %b, %6 : f64
  // CHECK: jawa.cast float_to_int %{{.*}} : f64 -> i64
  %9 = jawa.cast float_to_int %b : f64 -> i64
  // CHECK: jawa.cast bool_to_int %{{.*}} : i1 -> i64
  %10 = jawa.cast bool_to_int %8 : i1 -> i64
  jawa.return %5 : i64
}

// CHECK-LABEL: func.func @strings
func.func @strings(%n: i64) -> !jawa.str {
  // CHECK: jawa.str_const "sugeng rawuh"
  %0 = jawa.str_const "sugeng rawuh"
  // CHECK: jawa.to_str %{{.*}} : i64
  %1 = jawa.to_str %n : i64
  // CHECK: jawa.concat %{{.*}}, %{{.*}}, %{{.*}}
  %2 = jawa.concat %0, %1, %0
  // CHECK: jawa.str_len
  %3 = jawa.str_len %2
  // CHECK: jawa.cast str_to_float %{{.*}} : !jawa.str -> f64
  %4 = jawa.cast str_to_float %2 : !jawa.str -> f64
  // CHECK: jawa.cmp lt, %{{.*}}, %{{.*}} : !jawa.str
  %5 = jawa.cmp lt, %0, %2 : !jawa.str
  jawa.return %2 : !jawa.str
}

// CHECK-LABEL: func.func @heap
func.func @heap(%n: i64) {
  // CHECK: jawa.array_new(%{{.*}}, %{{.*}}) : (i64, i64) -> !jawa.array<i64>
  %xs = jawa.array_new(%n, %n) : (i64, i64) -> !jawa.array<i64>
  // CHECK: jawa.array_len %{{.*}} : !jawa.array<i64>
  %len = jawa.array_len %xs : !jawa.array<i64>
  // CHECK: jawa.array_get %{{.*}}[%{{.*}}] : !jawa.array<i64> -> i64
  %x = jawa.array_get %xs[%n] : !jawa.array<i64> -> i64
  // CHECK: jawa.array_set %{{.*}}[%{{.*}}], %{{.*}} : !jawa.array<i64>, i64
  jawa.array_set %xs[%n], %x : !jawa.array<i64>, i64
  // CHECK: jawa.array_push %{{.*}}, %{{.*}} : !jawa.array<i64>, i64
  jawa.array_push %xs, %x : !jawa.array<i64>, i64
  // CHECK: jawa.null : !jawa.struct<"Simpul">
  %null = jawa.null : !jawa.struct<"Simpul">
  // CHECK: jawa.struct_new(%{{.*}}, %{{.*}}) : (i64, !jawa.struct<"Simpul">) -> !jawa.struct<"Simpul">
  %node = jawa.struct_new(%n, %null) : (i64, !jawa.struct<"Simpul">) -> !jawa.struct<"Simpul">
  // CHECK: jawa.get_field %{{.*}}[1] : !jawa.struct<"Simpul"> -> !jawa.struct<"Simpul">
  %next = jawa.get_field %node[1] : !jawa.struct<"Simpul"> -> !jawa.struct<"Simpul">
  // CHECK: jawa.set_field %{{.*}}[0], %{{.*}} : !jawa.struct<"Simpul">, i64
  jawa.set_field %node[0], %n : !jawa.struct<"Simpul">, i64
  // CHECK: jawa.cmp eq, %{{.*}}, %{{.*}} : !jawa.struct<"Simpul">
  %is_end = jawa.cmp eq, %next, %null : !jawa.struct<"Simpul">
  // CHECK: jawa.print %{{.*}}, %{{.*}} : i1, !jawa.array<i64>
  jawa.print %is_end, %xs : i1, !jawa.array<i64>
  // CHECK: jawa.print{{$}}
  jawa.print
  jawa.return
}

// CHECK-LABEL: func.func @locals
func.func @locals(%n: i64) -> i64 {
  // CHECK: jawa.alloca "n" : !jawa.ptr<i64>
  %slot = jawa.alloca "n" : !jawa.ptr<i64>
  // CHECK: jawa.store %{{.*}}, %{{.*}} : !jawa.ptr<i64>
  jawa.store %n, %slot : !jawa.ptr<i64>
  // CHECK: jawa.load %{{.*}} : !jawa.ptr<i64>
  %v = jawa.load %slot : !jawa.ptr<i64>
  jawa.return %v : i64
}

// CHECK-LABEL: func.func @control_flow
func.func @control_flow(%c: i1) -> i64 {
  // CHECK: jawa.if %{{.*}} {
  // CHECK-NEXT: jawa.yield
  // CHECK-NEXT: } else {
  // CHECK-NEXT: jawa.unreachable
  jawa.if %c {
    jawa.yield
  } else {
    jawa.unreachable
  }
  // CHECK: jawa.while {
  // CHECK-NEXT: jawa.condition(%{{.*}})
  // CHECK-NEXT: } do {
  jawa.while {
    jawa.condition(%c)
  } do {
    // CHECK: jawa.if
    // CHECK-NEXT: jawa.break
    // CHECK: jawa.continue
    jawa.if %c {
      jawa.break
    }
    jawa.continue
  }
  // CHECK: jawa.for {
  // CHECK: } do {
  jawa.for {
    jawa.condition(%c)
  } do {
    %s = jawa.str_const "gagal"
    // CHECK: jawa.panic
    jawa.panic %s
    jawa.yield
    // CHECK: } step {
  } step {
    jawa.yield
  }
  %0 = jawa.const 1 : i64
  // CHECK: jawa.return %{{.*}} : i64
  jawa.return %0 : i64
}
