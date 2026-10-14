// RUN: jawa-opt %s -split-input-file -verify-diagnostics

func.func @order_booleans(%a: i1, %b: i1) {
  // expected-error @+1 {{predicate 'lt' needs i64, f64 or !jawa.str operands}}
  %0 = jawa.cmp lt, %a, %b : i1
  jawa.return
}

// -----

func.func @wrong_cast(%a: i64) {
  // expected-error @+1 {{'float_to_int' converts f64 -> i64}}
  %0 = jawa.cast float_to_int %a : i64 -> i64
  jawa.return
}

// -----

func.func @mixed_array(%a: i64, %b: f64) {
  // expected-error @+1 {{element 1 has type 'f64', expected 'i64'}}
  %0 = jawa.array_new(%a, %b) : (i64, f64) -> !jawa.array<i64>
  jawa.return
}

// -----

func.func @set_wrong_type(%xs: !jawa.array<i64>, %i: i64, %v: f64) {
  // expected-error @+1 {{value type 'f64' does not match the element type 'i64'}}
  jawa.array_set %xs[%i], %v : !jawa.array<i64>, f64
  jawa.return
}

// -----

func.func @unknown_struct(%a: i64) {
  // expected-error @+1 {{refers to undefined struct 'Hantu'}}
  %0 = jawa.struct_new(%a) : (i64) -> !jawa.struct<"Hantu">
  jawa.return
}

// -----

jawa.struct @Titik {x: i64, y: i64}

func.func @too_few_fields(%a: i64) {
  // expected-error @+1 {{expects 2 field values, got 1}}
  %0 = jawa.struct_new(%a) : (i64) -> !jawa.struct<"Titik">
  jawa.return
}

// -----

jawa.struct @Titik {x: i64, y: i64}

func.func @field_out_of_range(%p: !jawa.struct<"Titik">) {
  // expected-error @+1 {{field index 2 is out of range for 'Titik'}}
  %0 = jawa.get_field %p[2] : !jawa.struct<"Titik"> -> i64
  jawa.return
}

// -----

// expected-error @+1 {{declares field 'x' twice}}
jawa.struct @Kembar {x: i64, x: i64}

// -----

func.func @break_outside_loop() {
  // expected-error @+1 {{must be inside the body of a loop}}
  jawa.break
}

// -----

func.func @continue_in_condition(%c: i1) {
  jawa.while {
    jawa.if %c {
      // expected-error @+1 {{must be inside the body of a loop}}
      jawa.continue
    }
    jawa.condition(%c)
  } do {
    jawa.yield
  }
  jawa.return
}

// -----

func.func @return_wrong_type(%a: f64) -> i64 {
  // expected-error @+1 {{returns 'f64', but the function returns 'i64'}}
  jawa.return %a : f64
}

// -----

func.func @return_missing_value() -> i64 {
  // expected-error @+1 {{must return a value of type 'i64'}}
  jawa.return
}

// -----

func.func @print_struct(%p: !jawa.struct<"Titik">) {
  // expected-error @+1 {{cannot print a value of type '!jawa.struct<"Titik">'}}
  jawa.print %p : !jawa.struct<"Titik">
  jawa.return
}

// -----

func.func @if_without_terminator(%c: i1) {
  // expected-error @+1 {{'then' region cannot end in 'jawa.condition'}}
  "jawa.if"(%c) ({
    "jawa.condition"(%c) : (i1) -> ()
  }, {}) : (i1) -> ()
  jawa.return
}

// -----

func.func @empty_concat() {
  // expected-error @+1 {{needs at least one string}}
  %0 = "jawa.concat"() : () -> !jawa.str
  jawa.return
}
