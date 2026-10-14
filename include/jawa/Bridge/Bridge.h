//===- Bridge.h - The C++ side of the Rust/C++ bridge -----------*- C++ -*-===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// The Rust frontend drives code generation through `ModuleBuilder`, declared
// to Rust in crates/jawac/src/bridge.rs with the cxx bridge. Values and types
// cross the boundary as `uint32_t` handles, so this header needs no MLIR, and
// the cxx-generated glue compiles without it.
//
//===----------------------------------------------------------------------===//

#ifndef JAWA_BRIDGE_BRIDGE_H
#define JAWA_BRIDGE_BRIDGE_H

#include "rust/cxx.h"

#include <cstdint>
#include <memory>

namespace jawa::bridge {

// Shared enums, defined by the cxx-generated header.
enum class BinaryOp : std::uint8_t;
enum class Predicate : std::uint8_t;
enum class CastKind : std::uint8_t;
enum class Emit : std::uint8_t;

/// Builds a module in the `jawa` dialect, then runs it down the pipeline.
///
/// Control flow is built in the order the frontend walks the program:
///
///   begin_if(c) ... [begin_else() ...] end_if()
///   begin_while() <condition> begin_loop_body(c) ... end_while()
///   begin_for() <condition> begin_loop_body(c) ... begin_for_step() ...
///   end_for()
///
/// A region the frontend leaves without a terminator continues normally.
class ModuleBuilder {
public:
  explicit ModuleBuilder(rust::Str fileName);
  ~ModuleBuilder();
  ModuleBuilder(const ModuleBuilder &) = delete;
  ModuleBuilder &operator=(const ModuleBuilder &) = delete;

  // Types.
  std::uint32_t int_type();
  std::uint32_t float_type();
  std::uint32_t bool_type();
  std::uint32_t str_type();
  std::uint32_t array_type(std::uint32_t element);
  std::uint32_t struct_type(rust::Str name);

  // Structs and functions.
  void begin_struct(rust::Str name);
  void add_field(rust::Str name, std::uint32_t type);
  void end_struct();
  void declare_function(rust::Str name, rust::Slice<const std::uint32_t> params,
                        std::uint32_t result);
  void begin_function(rust::Str name);
  std::uint32_t param(std::uint32_t index);
  void end_function();
  void set_location(std::uint32_t line, std::uint32_t column);

  // Values.
  std::uint32_t const_int(std::int64_t value);
  std::uint32_t const_float(double value);
  std::uint32_t const_bool(bool value);
  std::uint32_t const_str(rust::Str value);
  std::uint32_t null_value(std::uint32_t type);
  std::uint32_t binary(BinaryOp op, std::uint32_t lhs, std::uint32_t rhs);
  std::uint32_t compare(Predicate predicate, std::uint32_t lhs,
                        std::uint32_t rhs);
  std::uint32_t negate(std::uint32_t value);
  std::uint32_t logical_not(std::uint32_t value);
  std::uint32_t concat(std::uint32_t lhs, std::uint32_t rhs);
  std::uint32_t to_str(std::uint32_t value);
  std::uint32_t cast(CastKind kind, std::uint32_t value);
  std::uint32_t str_len(std::uint32_t value);

  // Local variables.
  std::uint32_t local(std::uint32_t type, rust::Str name);
  std::uint32_t load(std::uint32_t slot);
  void store(std::uint32_t value, std::uint32_t slot);

  // Calls and built-in functions.
  std::uint32_t call(rust::Str callee, rust::Slice<const std::uint32_t> args);
  void print(rust::Slice<const std::uint32_t> values);
  void panic(std::uint32_t message);

  // Arrays and structs.
  std::uint32_t array_new(std::uint32_t type,
                          rust::Slice<const std::uint32_t> elements);
  std::uint32_t array_len(std::uint32_t array);
  std::uint32_t array_get(std::uint32_t array, std::uint32_t index);
  void array_set(std::uint32_t array, std::uint32_t index, std::uint32_t value);
  void array_push(std::uint32_t array, std::uint32_t value);
  std::uint32_t struct_new(std::uint32_t type,
                           rust::Slice<const std::uint32_t> fields);
  std::uint32_t get_field(std::uint32_t object, std::uint32_t index);
  void set_field(std::uint32_t object, std::uint32_t index,
                 std::uint32_t value);

  // Structured control flow.
  void begin_if(std::uint32_t condition);
  void begin_else();
  void end_if();
  void begin_while();
  void begin_for();
  void begin_loop_body(std::uint32_t condition);
  void begin_for_step();
  void end_while();
  void end_for();
  void emit_break();
  void emit_continue();
  void emit_return(std::uint32_t value);
  bool is_terminated() const;

  // Output.
  bool verify();
  bool emit(Emit kind, std::uint32_t optLevel, rust::Str path);
  rust::String error_message() const;

private:
  struct Impl;
  std::unique_ptr<Impl> impl;
};

std::unique_ptr<ModuleBuilder> new_module_builder(rust::Str fileName);

} // namespace jawa::bridge

#endif // JAWA_BRIDGE_BRIDGE_H
