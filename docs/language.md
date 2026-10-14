# The Jawa language

Jawa is a small, statically typed, garbage-collected language with Javanese keywords. This document describes all of it. Source files use the `.jw` extension and UTF-8.

- [A first program](#a-first-program)
- [Lexical structure](#lexical-structure)
- [Types](#types)
- [Variables](#variables)
- [Expressions](#expressions)
- [Statements](#statements)
- [Functions](#functions)
- [Structs](#structs)
- [Built-in functions](#built-in-functions)
- [Conversions](#conversions)
- [Runtime errors](#runtime-errors)
- [Keyword list](#keyword-list)

## A first program

```rust
gawe utama() {
    tulis("Sugeng rawuh ing JawaScript!")
}
```

Every program has exactly one `gawe utama()`, taking no parameters and returning nothing; it runs when the program starts. Functions and structs may appear in any order.

## Lexical structure

**Comments** run from `//` to the end of the line, or between `/*` and `*/`.

**Identifiers** start with a letter or `_` and continue with letters, digits or `_`. Letters may be any Unicode letters, so `éntuk` is a valid name. Keywords cannot be used as names.

**Numbers.** `42`, `1_000_000` (underscores are ignored), `0xff`, `0b1010` are `wilangan`. `3.14`, `2e3` and `1.5e-3` are `pecahan`. A `.` must be followed by a digit to make a float, so `0..10` is a range.

**Strings** are written in double quotes and may use the escapes `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, `\'` and `\u{…}` (a Unicode code point in hex, such as `\u{A9}` for ©). A string cannot span lines.

**Semicolons are optional.** As in Go, a line break ends a statement when the line ends with something that can end one: a name, a literal, `bali`, `mandheg`, `terusna`, a type name, or a closing `)`, `]` or `}`. Line breaks inside `( … )` and `[ … ]` never end a statement, so long argument lists and arrays can span lines. Two consequences:

- An operator that continues an expression goes at the end of the line, not the start of the next:

  ```rust
  ayo total = siji +
      loro
  ```

- An opening brace goes on the same line as `gawe`, `yen`, `sajrone` or `kanggo`. (`liyane` may start the line after a closing `}`.)

You can still separate statements on one line with `;`.

## Types

| Type | Values | Notes |
| --- | --- | --- |
| `wilangan` | 64-bit signed integers | Arithmetic wraps on overflow |
| `pecahan` | 64-bit IEEE 754 floats | |
| `logika` | `bener` (true) and `salah` (false) | |
| `ukara` | Immutable UTF-8 strings | A reference to the GC heap |
| `[T]` | Growable arrays of `T` | A reference; `T` may itself be an array |
| A struct name | Instances of a `wujud` | A reference, or `kosong` |
| `suwung` | No value | Only as a function's return type |

Arrays and structs are references: assigning one, or passing it to a function, shares it rather than copying it.

```rust
ayo a = [1, 2, 3]
ayo b = a
b[0] = 100
tulis(a)    // [100, 2, 3]
```

## Variables

```rust
ayo umur = 20             // can change
tetep jeneng = "Sri"      // cannot change
ayo dhuwur: pecahan = 1   // with a type
ayo xs: [wilangan] = []   // an empty array needs its type spelled out
```

Every variable starts with a value. Without a type, it takes the type of its value; an empty array `[]` and `kosong` need a type, since nothing else says what they hold. Assigning to a `tetep` variable is a compile error. A `tetep` array or struct can still have its contents changed; only the variable is fixed.

Variables are scoped to the block they are declared in. A new `ayo` with the same name hides the old one until the end of the block.

## Expressions

From loosest to tightest:

| Operators | Meaning |
| --- | --- |
| `utawa` | Logical or |
| `lan` | Logical and |
| `ora` (prefix) | Logical not |
| `==` `!=` | Equality |
| `<` `<=` `>` `>=` | Order |
| `+` `-` | Addition, subtraction, string joining |
| `*` `/` `%` | Multiplication, division, remainder |
| `-` (prefix) | Negation |
| `f(…)` `a[i]` `a.field` | Calls, indexing, fields |

`ora` binds looser than comparisons, as `not` does in Python: `ora x == 0` means `ora (x == 0)`. Use parentheses to group anything else.

`lan` and `utawa` take `logika` operands and short-circuit: the right side is evaluated only when the left does not already decide the result.

**Arithmetic** needs both sides to have the same number type; mix `wilangan` and `pecahan` with an explicit conversion. Integer literals adapt to `pecahan` where one is expected, so `x * 2` works for a float `x`.

- Integer `/` truncates toward zero (`-7 / 2` is `-3`), and `%` takes the sign of the dividend (`-7 % 3` is `-1`).
- Integer overflow wraps: the largest `wilangan` plus one is the smallest. `-9223372036854775808 / -1` also wraps instead of failing.
- Integer division or remainder by zero is a [runtime error](#runtime-errors). Float division follows IEEE 754 (`1.0 / 0.0` is `inf`).

**Joining text.** `+` with an `ukara` on either side joins strings, and converts a `wilangan`, `pecahan` or `logika` on the other side to text:

```rust
tulis("umur: " + 20 + ", diwasa: " + bener)   // umur: 20, diwasa: bener
```

**Comparison.** Numbers and strings support all six comparisons; strings compare by their bytes. `logika` supports `==` and `!=`. Arrays and structs support `==` and `!=`, which compare identity, and a struct can be compared with `kosong`.

**Array and struct literals:**

```rust
ayo xs = [1, 2, 3]
ayo p = Titik { x: 1.5, y: 2 }   // every field, in any order
```

## Statements

**Assignment**: `x = v`, `xs[i] = v`, `p.field = v`, and the compound forms `+=`, `-=`, `*=`, `/=`, `%=`. `s += t` appends to a string variable.

**Conditionals:**

```rust
yen n < 0 {
    tulis("negatif")
} liyane yen n == 0 {
    tulis("nol")
} liyane {
    tulis("positif")
}
```

The condition must be a `logika`; there is no truthiness.

**Loops:**

```rust
sajrone n != 1 {            // while
    n = langkah(n)
}

kanggo i ing 0..10 { }      // 0 to 9
kanggo i ing 1..=10 { }     // 1 to 10
kanggo x ing xs { }         // each element of an array
```

A range's end is evaluated once, before the loop. A loop over an array re-reads its length each time round, so elements appended in the body are visited too. The loop variable cannot be assigned to.

`mandheg` leaves the innermost loop. `terusna` skips to its next iteration; in a `kanggo` loop the counter still advances.

**Blocks** `{ … }` can stand on their own to limit the scope of variables.

## Functions

```rust
gawe jarak(a: Titik, b: Titik) -> pecahan {
    tetep dx = a.x - b.x
    tetep dy = a.y - b.y
    bali dx * dx + dy * dy
}
```

Parameters need types. A function with a return type must `bali` a value on every path; the compiler checks this. A function without one returns nothing, and `bali` on its own leaves it early. Parameters behave like `ayo` variables inside the function. Functions can call each other, and themselves, in any order. Functions cannot be nested, and there are no methods: write `tambah(xs, x)`, not `xs.tambah(x)`.

## Structs

```rust
wujud Simpul {
    regane: wilangan
    sabanjure: Simpul
}
```

Fields are separated by line breaks or commas. A struct may refer to itself, which is how linked structures are built; `kosong` marks the end:

```rust
ayo dhaptar: Simpul = kosong
kanggo i ing 0..3 {
    dhaptar = Simpul { regane: i, sabanjure: dhaptar }
}
```

## Built-in functions

| Function | Does |
| --- | --- |
| `tulis(a, b, …)` | Prints its arguments separated by spaces, then a line break. It prints numbers, `logika`, `ukara`, and arrays of those. |
| `dawane(x)` | The length of an `ukara` (in bytes) or an array |
| `tambah(xs, x)` | Appends `x` to the array `xs` |
| `gagal(pesen)` | Stops the program with the message `pesen` |

Floats print with the fewest digits that read back as the same number, and always with a point or an exponent: `2.0`, `0.30000000000000004`, `1e+16`. Booleans print as `bener` and `salah`. Strings inside arrays print quoted: `["a", "b"]`.

## Conversions

A type name used like a function converts a value:

| Conversion | From | Behaviour |
| --- | --- | --- |
| `wilangan(x)` | `pecahan` | Truncates toward zero; saturates at the ends of the range; NaN gives 0 |
| | `logika` | `bener` is 1, `salah` is 0 |
| | `ukara` | Parses a decimal integer, ignoring surrounding whitespace; anything else is a runtime error |
| `pecahan(x)` | `wilangan` | The nearest float |
| | `ukara` | Parses a decimal float |
| `ukara(x)` | `wilangan`, `pecahan`, `logika` | Formats as `tulis` prints it |
| `logika(x)` | `wilangan` | `bener` unless zero |

## Runtime errors

Some mistakes can only be seen while the program runs. Each one prints a message starting with `gagal:` to standard error, after anything the program has printed so far, and exits with status 1:

| Cause | Message |
| --- | --- |
| An index outside `0 .. dawane(xs) - 1` | `gagal: index 3 is out of bounds for an array of length 3` |
| A field of `kosong` | ``gagal: used a field of `kosong` `` |
| Integer division or remainder by zero | `gagal: division by zero` |
| `wilangan(s)` or `pecahan(s)` on text that is not a number | `gagal: cannot read "abc" as a wilangan` |
| `gagal(pesen)` | `gagal: ` followed by the message |

## Keyword list

`ayo` `bali` `bener` `gawe` `ing` `kanggo` `kosong` `lan` `liyane` `logika` `mandheg` `ora` `pecahan` `sajrone` `salah` `suwung` `terusna` `tetep` `ukara` `utawa` `wilangan` `wujud` `yen`

The built-in functions `tulis`, `dawane`, `tambah` and `gagal` are not keywords, but cannot be redefined.
