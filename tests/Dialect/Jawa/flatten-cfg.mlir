// RUN: jawa-opt %s -jawa-flatten-cfg -split-input-file | FileCheck %s

// CHECK-LABEL: func.func @if_else
// CHECK:   cf.cond_br %arg0, ^[[THEN:bb[0-9]+]], ^[[ELSE:bb[0-9]+]]
// CHECK: ^[[THEN]]:
// CHECK:   jawa.print
// CHECK:   cf.br ^[[CONT:bb[0-9]+]]
// CHECK: ^[[ELSE]]:
// CHECK:   cf.br ^[[CONT]]
// CHECK: ^[[CONT]]:
// CHECK:   return
func.func @if_else(%c: i1) {
  jawa.if %c {
    jawa.print
    jawa.yield
  } else {
    jawa.yield
  }
  jawa.return
}

// -----

// Without `liyane`, a false condition goes straight to the continuation.
// CHECK-LABEL: func.func @if_only
// CHECK:   cf.cond_br %arg0, ^[[THEN:bb[0-9]+]], ^[[CONT:bb[0-9]+]]
// CHECK: ^[[THEN]]:
// CHECK-NEXT: cf.br ^[[CONT]]
// CHECK: ^[[CONT]]:
// CHECK-NEXT: return
func.func @if_only(%c: i1) {
  jawa.if %c {
    jawa.yield
  }
  jawa.return
}

// -----

// `bali` inside nested regions becomes a plain return in its block.
// CHECK-LABEL: func.func @early_return
// CHECK:   cf.cond_br %arg0, ^[[THEN:bb[0-9]+]], ^[[CONT:bb[0-9]+]]
// CHECK: ^[[THEN]]:
// CHECK-NEXT: return %arg1 : i64
// CHECK: ^[[CONT]]:
// CHECK: return %{{.*}} : i64
// CHECK-NOT: jawa.return
func.func @early_return(%c: i1, %x: i64) -> i64 {
  jawa.if %c {
    jawa.return %x : i64
  }
  %zero = jawa.const 0 : i64
  jawa.return %zero : i64
}

// -----

// CHECK-LABEL: func.func @while_loop
// CHECK:   cf.br ^[[COND:bb[0-9]+]]
// CHECK: ^[[COND]]:
// CHECK:   %[[C:.*]] = jawa.load
// CHECK:   cf.cond_br %[[C]], ^[[BODY:bb[0-9]+]], ^[[EXIT:bb[0-9]+]]
// CHECK: ^[[BODY]]:
// CHECK:   jawa.print
// CHECK:   cf.br ^[[COND]]
// CHECK: ^[[EXIT]]:
// CHECK:   return
func.func @while_loop(%slot: !jawa.ptr<i1>) {
  jawa.while {
    %c = jawa.load %slot : !jawa.ptr<i1>
    jawa.condition(%c)
  } do {
    jawa.print
    jawa.yield
  }
  jawa.return
}

// -----

// `continue` in a `for` runs the step; `break` leaves the loop.
// CHECK-LABEL: func.func @for_loop
// CHECK:   cf.br ^[[COND:bb[0-9]+]]
// CHECK: ^[[COND]]:
// CHECK:   cf.cond_br %arg0, ^[[BODY:bb[0-9]+]], ^[[EXIT:bb[0-9]+]]
// CHECK: ^[[BODY]]:
// CHECK:   cf.cond_br %arg1, ^[[BREAK:bb[0-9]+]], ^[[AFTER:bb[0-9]+]]
// CHECK: ^[[BREAK]]:
// CHECK-NEXT: cf.br ^[[EXIT]]
// CHECK: ^[[AFTER]]:
// CHECK:   cf.cond_br %arg2, ^[[CONTINUE:bb[0-9]+]], ^[[REST:bb[0-9]+]]
// CHECK: ^[[CONTINUE]]:
// CHECK-NEXT: cf.br ^[[STEP:bb[0-9]+]]
// CHECK: ^[[REST]]:
// CHECK:   jawa.print
// CHECK:   cf.br ^[[STEP]]
// CHECK: ^[[STEP]]:
// CHECK:   jawa.store
// CHECK:   cf.br ^[[COND]]
// CHECK: ^[[EXIT]]:
// CHECK:   return
func.func @for_loop(%c: i1, %stop: i1, %skip: i1, %slot: !jawa.ptr<i64>, %n: i64) {
  jawa.for {
    jawa.condition(%c)
  } do {
    jawa.if %stop {
      jawa.break
    }
    jawa.if %skip {
      jawa.continue
    }
    jawa.print
    jawa.yield
  } step {
    jawa.store %n, %slot : !jawa.ptr<i64>
    jawa.yield
  }
  jawa.return
}

// -----

// A `break` in an inner loop only leaves the inner loop.
// CHECK-LABEL: func.func @nested_loops
// CHECK:   cf.br ^[[OUTER:bb[0-9]+]]
// CHECK: ^[[OUTER]]:
// CHECK:   cf.cond_br %arg0, ^[[OUTER_BODY:bb[0-9]+]], ^[[OUTER_EXIT:bb[0-9]+]]
// CHECK: ^[[OUTER_BODY]]:
// CHECK:   cf.br ^[[INNER:bb[0-9]+]]
// CHECK: ^[[INNER]]:
// CHECK:   cf.cond_br %arg1, ^[[INNER_BODY:bb[0-9]+]], ^[[INNER_EXIT:bb[0-9]+]]
// CHECK: ^[[INNER_BODY]]:
// CHECK-NEXT: cf.br ^[[INNER_EXIT]]
// CHECK: ^[[INNER_EXIT]]:
// CHECK-NEXT: cf.br ^[[OUTER]]
// CHECK: ^[[OUTER_EXIT]]:
// CHECK-NEXT: return
func.func @nested_loops(%a: i1, %b: i1) {
  jawa.while {
    jawa.condition(%a)
  } do {
    jawa.while {
      jawa.condition(%b)
    } do {
      jawa.break
    }
    jawa.yield
  }
  jawa.return
}

// -----

// Short-circuit logic in a loop condition puts a `jawa.if` inside the
// condition region; it is flattened in place.
// CHECK-LABEL: func.func @condition_with_if
// CHECK:   cf.br ^[[COND:bb[0-9]+]]
// CHECK: ^[[COND]]:
// CHECK:   cf.cond_br %arg0, ^[[RHS:bb[0-9]+]], ^[[JOIN:bb[0-9]+]]
// CHECK: ^[[RHS]]:
// CHECK:   jawa.store %arg1
// CHECK:   cf.br ^[[JOIN]]
// CHECK: ^[[JOIN]]:
// CHECK:   %[[V:.*]] = jawa.load
// CHECK:   cf.cond_br %[[V]], ^[[BODY:bb[0-9]+]], ^[[EXIT:bb[0-9]+]]
func.func @condition_with_if(%a: i1, %b: i1, %tmp: !jawa.ptr<i1>) {
  jawa.while {
    jawa.store %a, %tmp : !jawa.ptr<i1>
    jawa.if %a {
      jawa.store %b, %tmp : !jawa.ptr<i1>
      jawa.yield
    }
    %v = jawa.load %tmp : !jawa.ptr<i1>
    jawa.condition(%v)
  } do {
    jawa.yield
  }
  jawa.return
}
