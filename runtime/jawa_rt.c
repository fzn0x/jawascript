/*===- jawa_rt.c - The Jawa runtime ---------------------------------------===//
 *
 * Part of JawaScript, under the MIT License. See LICENSE for details.
 *
 *===----------------------------------------------------------------------===*/

#include "jawa_rt.h"

#include <gc/gc.h>

#include <errno.h>
#include <inttypes.h>
#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#include <windows.h>
#endif

/* The program's entry point, `gawe utama()`. Compiled Jawa functions are
 * prefixed with `jw_` so they never clash with C symbols. */
extern void jw_utama(void);

/*===----------------------------------------------------------------------===
 * Errors
 *===----------------------------------------------------------------------===*/

static _Noreturn void fail(const char *format, ...) {
  /* Whatever the program printed so far comes first. */
  fflush(stdout);
  fputs("gagal: ", stderr);
  va_list args;
  va_start(args, format);
  vfprintf(stderr, format, args);
  va_end(args);
  fputc('\n', stderr);
  exit(1);
}

_Noreturn void jawa_rt_panic(const jawa_str *message) {
  fail("%s", message->data);
}

_Noreturn void jawa_rt_panic_index(int64_t index, int64_t len) {
  fail("index %" PRId64 " is out of bounds for an array of length %" PRId64,
       index, len);
}

_Noreturn void jawa_rt_panic_null(void) { fail("used a field of `kosong`"); }

_Noreturn void jawa_rt_panic_div_zero(void) { fail("division by zero"); }

/*===----------------------------------------------------------------------===
 * Memory
 *===----------------------------------------------------------------------===*/

void *jawa_rt_alloc(int64_t size) {
  void *memory = GC_MALLOC((size_t)size);
  if (!memory)
    fail("out of memory");
  return memory;
}

void *jawa_rt_alloc_atomic(int64_t size) {
  void *memory = GC_MALLOC_ATOMIC((size_t)size);
  if (!memory)
    fail("out of memory");
  return memory;
}

/*===----------------------------------------------------------------------===
 * Strings
 *===----------------------------------------------------------------------===*/

/* A string with room for `len` bytes, NUL-terminated. Strings hold no
 * pointers, so the collector never scans them. */
static jawa_str *str_alloc(int64_t len) {
  jawa_str *text = jawa_rt_alloc_atomic((int64_t)sizeof(jawa_str) + len + 1);
  text->len = len;
  text->data[len] = '\0';
  return text;
}

static jawa_str *str_from(const char *bytes, int64_t len) {
  jawa_str *text = str_alloc(len);
  memcpy(text->data, bytes, (size_t)len);
  return text;
}

jawa_str *jawa_rt_str_concat(int64_t count, ...) {
  va_list args;
  int64_t total = 0;
  va_start(args, count);
  for (int64_t i = 0; i < count; i++)
    total += va_arg(args, const jawa_str *)->len;
  va_end(args);

  jawa_str *result = str_alloc(total);
  char *out = result->data;
  va_start(args, count);
  for (int64_t i = 0; i < count; i++) {
    const jawa_str *part = va_arg(args, const jawa_str *);
    memcpy(out, part->data, (size_t)part->len);
    out += part->len;
  }
  va_end(args);
  return result;
}

int32_t jawa_rt_str_eq(const jawa_str *a, const jawa_str *b) {
  return a == b ||
         (a->len == b->len && memcmp(a->data, b->data, (size_t)a->len) == 0);
}

int32_t jawa_rt_str_cmp(const jawa_str *a, const jawa_str *b) {
  int64_t shorter = a->len < b->len ? a->len : b->len;
  int order = memcmp(a->data, b->data, (size_t)shorter);
  if (order != 0)
    return order < 0 ? -1 : 1;
  return a->len < b->len ? -1 : (a->len > b->len ? 1 : 0);
}

jawa_str *jawa_rt_int_to_str(int64_t value) {
  char buffer[32];
  int len = snprintf(buffer, sizeof buffer, "%" PRId64, value);
  return str_from(buffer, len);
}

/* Formats a float with the fewest digits that read back as the same value,
 * and always with a `.` or exponent so it never looks like a `wilangan`.
 * Like Python and JavaScript, values from 1e-4 up to 1e16 print in plain
 * decimal (100.0, 0.0001) and the rest in scientific notation (1e+16). */
static int format_float(char *buffer, size_t size, double value) {
  if (isnan(value))
    return snprintf(buffer, size, "nan");
  if (isinf(value))
    return snprintf(buffer, size, value < 0 ? "-inf" : "inf");
  if (value == 0)
    return snprintf(buffer, size, signbit(value) ? "-0.0" : "0.0");

  /* The fewest significant digits that read back as `value`. */
  int digits = 1;
  int len = 0;
  for (; digits <= 17; digits++) {
    len = snprintf(buffer, size, "%.*e", digits - 1, value);
    if (strtod(buffer, NULL) == value)
      break;
  }
  int exponent = atoi(strchr(buffer, 'e') + 1);
  if (exponent < -4 || exponent >= 16)
    return len;

  int decimals = digits - 1 - exponent;
  return snprintf(buffer, size, "%.*f", decimals < 1 ? 1 : decimals, value);
}

jawa_str *jawa_rt_float_to_str(double value) {
  char buffer[48];
  int len = format_float(buffer, sizeof buffer, value);
  return str_from(buffer, len);
}

jawa_str *jawa_rt_bool_to_str(int32_t value) {
  return value ? str_from("bener", 5) : str_from("salah", 5);
}

static int is_space(char c) {
  return c == ' ' || c == '\t' || c == '\n' || c == '\r';
}

/* The text between leading and trailing whitespace. */
static void trim(const jawa_str *text, const char **begin, const char **end) {
  *begin = text->data;
  *end = text->data + text->len;
  while (*begin < *end && is_space(**begin))
    (*begin)++;
  while (*end > *begin && is_space((*end)[-1]))
    (*end)--;
}

int64_t jawa_rt_str_to_int(const jawa_str *text) {
  const char *begin, *end;
  trim(text, &begin, &end);
  const char *digits = begin;
  if (digits < end && (*digits == '-' || *digits == '+'))
    digits++;
  int valid = digits < end;
  for (const char *p = digits; p < end; p++)
    if (*p < '0' || *p > '9')
      valid = 0;
  if (valid) {
    errno = 0;
    /* `data` is NUL-terminated, and whitespace after `end` stops strtoll. */
    char *stop;
    long long value = strtoll(begin, &stop, 10);
    if (errno == 0 && stop == end)
      return value;
  }
  fail("cannot read \"%s\" as a wilangan", text->data);
}

double jawa_rt_str_to_float(const jawa_str *text) {
  const char *begin, *end;
  trim(text, &begin, &end);
  if (begin < end) {
    char *stop;
    double value = strtod(begin, &stop);
    if (stop == end)
      return value;
  }
  fail("cannot read \"%s\" as a pecahan", text->data);
}

/*===----------------------------------------------------------------------===
 * Printing
 *===----------------------------------------------------------------------===*/

void jawa_rt_print_int(int64_t value) { printf("%" PRId64, value); }

void jawa_rt_print_float(double value) {
  char buffer[48];
  format_float(buffer, sizeof buffer, value);
  fputs(buffer, stdout);
}

void jawa_rt_print_bool(int32_t value) {
  fputs(value ? "bener" : "salah", stdout);
}

void jawa_rt_print_str(const jawa_str *value) {
  fwrite(value->data, 1, (size_t)value->len, stdout);
}

static void print_quoted(const jawa_str *value) {
  putchar('"');
  for (int64_t i = 0; i < value->len; i++) {
    char c = value->data[i];
    switch (c) {
    case '"':
      fputs("\\\"", stdout);
      break;
    case '\\':
      fputs("\\\\", stdout);
      break;
    case '\n':
      fputs("\\n", stdout);
      break;
    case '\t':
      fputs("\\t", stdout);
      break;
    default:
      putchar(c);
    }
  }
  putchar('"');
}

void jawa_rt_print_array(const jawa_array *array, int32_t kind) {
  putchar('[');
  for (int64_t i = 0; i < array->len; i++) {
    if (i > 0)
      fputs(", ", stdout);
    switch (kind) {
    case JAWA_KIND_INT:
      jawa_rt_print_int(((const int64_t *)array->data)[i]);
      break;
    case JAWA_KIND_FLOAT:
      jawa_rt_print_float(((const double *)array->data)[i]);
      break;
    case JAWA_KIND_BOOL:
      jawa_rt_print_bool(((const uint8_t *)array->data)[i] & 1);
      break;
    case JAWA_KIND_STR:
      print_quoted(((const jawa_str *const *)array->data)[i]);
      break;
    }
  }
  putchar(']');
}

void jawa_rt_print_space(void) { putchar(' '); }

void jawa_rt_print_newline(void) { putchar('\n'); }

/*===----------------------------------------------------------------------===
 * Arrays
 *===----------------------------------------------------------------------===*/

static void *alloc_elements(int64_t count, int64_t elem_size,
                            int32_t has_pointers) {
  int64_t size = count * elem_size;
  return has_pointers ? jawa_rt_alloc(size) : jawa_rt_alloc_atomic(size);
}

jawa_array *jawa_rt_array_new(int64_t len, int64_t elem_size,
                              int32_t has_pointers) {
  jawa_array *array = jawa_rt_alloc((int64_t)sizeof(jawa_array));
  array->len = len;
  array->cap = len < 4 ? 4 : len;
  array->data = alloc_elements(array->cap, elem_size, has_pointers);
  return array;
}

void *jawa_rt_array_push(jawa_array *array, int64_t elem_size,
                         int32_t has_pointers) {
  if (array->len == array->cap) {
    int64_t cap = array->cap * 2;
    void *data = alloc_elements(cap, elem_size, has_pointers);
    memcpy(data, array->data, (size_t)(array->len * elem_size));
    array->data = data;
    array->cap = cap;
  }
  return (char *)array->data + array->len++ * elem_size;
}

/*===----------------------------------------------------------------------===
 * Entry point
 *===----------------------------------------------------------------------===*/

int main(void) {
  GC_INIT();
#ifdef _WIN32
  /* Print `\n` as is, like on every other platform, and let the console
   * show UTF-8 text, including aksara Jawa. */
  _setmode(_fileno(stdout), _O_BINARY);
  SetConsoleOutputCP(CP_UTF8);
#endif
  jw_utama();
  return 0;
}
