# Pandhuan: a first tour of Jawa

This guide walks through Jawa from a first program to structs and linked lists. Each example is a whole program: save it as a `.jw` file and run it with `jawac run`. The text under each example is exactly what it prints, and the test suite checks that on every change.

If you have not built `jawac` yet, start with [Getting started](../README.md#getting-started) in the README. The [vocabulary](vocabulary.md) lists every Jawa word with its meaning, and the [language reference](language.md) has the exact rules.

## Your first program

```ts
gawe utama() {
    tulis("Sugeng rawuh ing JawaScript!")
}
```

```text
Sugeng rawuh ing JawaScript!
```

Every program starts at `gawe utama()`. *Gawe* ("make") declares a function, and *utama* ("main") is the one that runs first. `tulis` ("write") prints its arguments with spaces between them and ends the line.

Save the program as `sugeng.jw` and run it:

```sh
jawac run sugeng.jw      # compile and run in one go
jawac sugeng.jw          # or build an executable: ./sugeng (sugeng.exe on Windows)
./sugeng
```

`jawac` compiles through MLIR and LLVM to native code, so the executable runs on its own, with no interpreter or virtual machine behind it.

## Values and variables

```ts
gawe utama() {
    ayo umur = 20
    tetep jeneng = "Sri"
    ayo dhuwur: pecahan = 1.62
    umur += 1
    tulis(jeneng, umur, dhuwur)
}
```

```text
Sri 21 1.62
```

`ayo` ("let's") declares a variable that can change, like JavaScript's `let`. `tetep` ("stays the same") declares one that cannot, like `const`. A variable takes the type of its first value, or the type written after the `:`.

Jawa has four basic types:

| Type | Meaning | Holds | Examples |
| --- | --- | --- | --- |
| `wilangan` | number | 64-bit integers | `42`, `-7`, `1_000_000` |
| `pecahan` | fraction | 64-bit floats | `1.62`, `2e3` |
| `logika` | logic | booleans | `bener`, `salah` |
| `ukara` | sentence | text (strings) | `"Jawa"` |

Types are checked when you compile, and they do not change behind your back. Integers and floats are separate: a literal like `2` adapts to whichever one is needed, but a `wilangan` variable and a `pecahan` variable cannot be added until you convert one of them, as in `pecahan(n)`.

## Text

```ts
gawe utama() {
    tetep jeneng = "Sri"
    tetep umur = 20
    tulis("Jenenge " + jeneng + ", umure " + umur)
    tulis(`${jeneng} umure ${umur}, taun ngarep ${umur + 1}`)
    tulis(dawane(jeneng), jeneng == "Sri", ukara(3.5) + "!")
}
```

```text
Jenenge Sri, umure 20
Sri umure 20, taun ngarep 21
3 bener 3.5!
```

`+` joins an `ukara` with numbers and booleans, converting them as it goes; this is the only conversion Jawa does without being asked. Template strings in backticks work as in JavaScript and run any expression inside `${…}`. `dawane` ("its length") counts bytes, and `ukara(x)` turns a value into text.

## Deciding with `yen`

```ts
gawe utama() {
    tetep suhu = 31
    yen (suhu > 30) {
        tulis("panas")
    } liyane yen (suhu < 20) {
        tulis("adhem")
    } liyane {
        tulis("sedheng")
    }

    tetep umur = 20
    tetep duwe_karcis = bener
    tulis(umur >= 18 && duwe_karcis, umur < 13 utawa ora duwe_karcis)
}
```

```text
panas
bener salah
```

`yen` ("if") and `liyane` ("the other") read like `if` and `else`, and the parentheses around a condition are optional. A condition must be a `logika`: Jawa never treats `0` or `""` as false. Combine conditions with `&&`, `||` and `!`, or with the words `lan` ("and"), `utawa` ("or") and `ora` ("not"). `==` and `===` mean the same thing.

## Repeating with `sajrone` and `kanggo`

```ts
gawe utama() {
    ayo n = 1
    sajrone (n < 100) {
        n *= 3
    }
    tulis("n =", n)

    kanggo (ayo i = 0; i < 3; i++) {
        tulis("puteran", i)
    }

    kanggo i ing 1..=3 {
        tulis(i, "kaping", i, "=", i * i)
    }
}
```

```text
n = 243
puteran 0
puteran 1
puteran 2
1 kaping 1 = 1
2 kaping 2 = 4
3 kaping 3 = 9
```

`sajrone` ("during") is `while`. `kanggo` ("for") comes in the JavaScript form `kanggo (init; condition; update)` and in a shorter form over a range: `kanggo i ing 0..10` counts from 0 to 9, and `1..=10` includes the 10.

`mandheg` ("stop") leaves a loop and `terusna` ("carry on") skips to the next round:

```ts
gawe utama() {
    kanggo i ing 0..100 {
        yen (i % 2 == 0) {
            terusna
        }
        yen (i > 7) {
            mandheg
        }
        tulis(i)
    }
}
```

```text
1
3
5
7
```

## Functions with `gawe` and `bali`

```ts
gawe kuadrat(n: wilangan): wilangan {
    bali n * n
}

gawe sapa(jeneng: ukara) {
    tulis(`Sugeng enjing, ${jeneng}!`)
}

gawe utama() {
    tulis(kuadrat(7))
    sapa("Joko")
}
```

```text
49
Sugeng enjing, Joko!
```

Parameters need types, and the return type follows the `:` as in TypeScript. `bali` ("go back") returns a value. A function without a return type returns nothing, and a plain `bali` leaves it early. Functions can be declared in any order and may call themselves. The compiler checks that a function with a return type reaches a `bali` on every path.

## Arrays (larik)

```ts
gawe utama() {
    ayo angka = [3, 1, 4]
    tambah(angka, 1)
    angka[0] = 30
    tulis(angka, dawane(angka))

    ayo gunggung = 0
    kanggo x ing angka {
        gunggung += x
    }
    tulis("gunggung:", gunggung)

    ayo jeneng: ukara[] = []
    tambah(jeneng, "Sri")
    tulis(jeneng)
}
```

```text
[30, 1, 4, 1] 4
gunggung: 36
["Sri"]
```

An array holds values of one type, written `wilangan[]` (or `[wilangan]`). `tambah` ("add") appends, `dawane` gives the length, and indexes start at 0. An empty array needs its type spelled out, since nothing else says what it will hold. There are no methods: write `tambah(angka, 1)` where JavaScript has `angka.push(1)`.

## Structs with `wujud`

```ts
wujud Wong {
    jeneng: ukara
    umur: wilangan
}

gawe nambah_umur(w: Wong) {
    w.umur += 1
}

gawe utama() {
    ayo sri = Wong { jeneng: "Sri", umur: 20 }
    nambah_umur(sri)
    tulis(sri.jeneng, sri.umur)
}
```

```text
Sri 21
```

`wujud` ("shape, form") declares a struct, like a TypeScript `interface` that you can build. `Wong { jeneng: "Sri", umur: 20 }` builds one, with every field given in any order. Structs and arrays are references, as objects are in JavaScript: `nambah_umur` changes the same `Wong` that `utama` holds.

A field can refer to a struct of its own type, and `kosong` ("empty") is the reference to nothing, like `null`. That is enough for a linked list:

```ts
wujud Simpul {
    regane: wilangan
    sabanjure: Simpul
}

gawe utama() {
    ayo dhaptar: Simpul = kosong
    kanggo i ing 1..=3 {
        dhaptar = Simpul { regane: i, sabanjure: dhaptar }
    }

    ayo simpul = dhaptar
    sajrone (simpul != kosong) {
        tulis(simpul.regane)
        simpul = simpul.sabanjure
    }
}
```

```text
3
2
1
```

Memory is managed for you: a garbage collector frees structs, arrays and text once nothing refers to them.

## When something goes wrong

The compiler reads the whole file and reports every mistake it finds, each with a pointer to the spot. When a JavaScript habit slips in, it names the Jawa word:

```console
$ jawac check salah.jw
error: `let` is not a Jawa keyword
 --> salah.jw:2:5
  |
2 |     let umur = 20
  |     ^^^
  = note: Jawa writes `let` as `ayo`
error: could not compile `salah.jw` because of 1 error
```

Typos get a suggestion too: `tulis(umr)` asks whether you meant `umur`.

Some mistakes can only show up while the program runs, such as reading past the end of an array:

```ts
gawe utama() {
    ayo angka = [1, 2, 3]
    tulis("sadurunge")
    tulis(angka[3])
}
```

The program prints what it has so far, then stops with a message in Javanese that starts with `gagal:` ("failed"):

```console
$ jawac run indeks.jw
sadurunge
gagal: indeks 3 ora ana ing larik sing dawane 3
```

That reads "index 3 does not exist in an array whose length is 3". You can stop a program the same way with `gagal("…")`, where JavaScript would `throw`. The [vocabulary](vocabulary.md#runtime-messages) translates every message.

## Where next

- The [examples](https://github.com/fzn0x/jawascript/tree/main/examples) are longer programs: the Javanese *weton* calendar, the Sieve of Eratosthenes, quicksort, aksara Jawa and more.
- The [vocabulary](vocabulary.md) is a word list to keep open while you write.
- The [language reference](language.md) covers every rule: operators, conversions, scoping and what each built-in does.
- [How jawac works](architecture.md) follows a program through the compiler, from the Rust frontend to MLIR, LLVM and the linker.
