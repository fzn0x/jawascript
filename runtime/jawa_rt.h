/*===- jawa_rt.h - The Jawa runtime -----------------------------*- C -*-===//
 *
 * Part of JawaScript, under the MIT License. See LICENSE for details.
 *
 *===----------------------------------------------------------------------===//
 *
 * The functions compiled Jawa programs call. Every heap object (strings,
 * arrays, structs) is allocated from the Boehm-Demers-Weiser conservative
 * garbage collector, so programs never free memory themselves.
 *
 * The lowering in lib/Conversion/JawaToLLVM depends on these layouts and
 * signatures; change both together.
 *
 *===----------------------------------------------------------------------===*/

#ifndef JAWA_RT_H
#define JAWA_RT_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* `ukara`: an immutable string. `data` holds `len` bytes of UTF-8 followed by
 * a NUL, so it can be handed to C functions directly. String literals are
 * emitted with the same layout as constants in the program's data section. */
typedef struct jawa_str {
  int64_t len;
  char data[];
} jawa_str;

/* `[T]`: a growable array of fixed-size elements. */
typedef struct jawa_array {
  int64_t len;
  int64_t cap;
  void *data;
} jawa_array;

/* Element kinds for printing arrays. */
enum jawa_kind {
  JAWA_KIND_INT = 0,
  JAWA_KIND_FLOAT = 1,
  JAWA_KIND_BOOL = 2,
  JAWA_KIND_STR = 3,
};

/* Memory. `alloc` memory is scanned for pointers; `alloc_atomic` memory is
 * not, and must never hold one. */
void *jawa_rt_alloc(int64_t size);
void *jawa_rt_alloc_atomic(int64_t size);

/* Strings. `concat` joins `count` strings passed as variadic arguments. */
jawa_str *jawa_rt_str_concat(int64_t count, ...);
int32_t jawa_rt_str_eq(const jawa_str *a, const jawa_str *b);
int32_t jawa_rt_str_cmp(const jawa_str *a, const jawa_str *b);
jawa_str *jawa_rt_int_to_str(int64_t value);
jawa_str *jawa_rt_float_to_str(double value);
jawa_str *jawa_rt_bool_to_str(int32_t value);
int64_t jawa_rt_str_to_int(const jawa_str *text);
double jawa_rt_str_to_float(const jawa_str *text);

/* `tulis`. */
void jawa_rt_print_int(int64_t value);
void jawa_rt_print_float(double value);
void jawa_rt_print_bool(int32_t value);
void jawa_rt_print_str(const jawa_str *value);
void jawa_rt_print_array(const jawa_array *array, int32_t kind);
void jawa_rt_print_space(void);
void jawa_rt_print_newline(void);

/* Arrays. `push` grows the array by one element and returns the address of
 * the new slot for the caller to store into. */
jawa_array *jawa_rt_array_new(int64_t len, int64_t elem_size,
                              int32_t has_pointers);
void *jawa_rt_array_push(jawa_array *array, int64_t elem_size,
                         int32_t has_pointers);

/* Runtime errors. Each prints a message to stderr and exits with status 1. */
_Noreturn void jawa_rt_panic(const jawa_str *message);
_Noreturn void jawa_rt_panic_index(int64_t index, int64_t len);
_Noreturn void jawa_rt_panic_null(void);
_Noreturn void jawa_rt_panic_div_zero(void);

#ifdef __cplusplus
}
#endif

#endif /* JAWA_RT_H */
