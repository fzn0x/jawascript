# Kosakata: the Jawa vocabulary

Every word Jawa uses, what it means in Javanese, and what it corresponds to in JavaScript and TypeScript. The keywords are ordinary words of everyday Javanese (*ngoko*, the informal register), so a line of Jawa reads close to a Javanese sentence: `yen umur >= 18 { tulis("wis diwasa") }` is "if age ≥ 18, write 'already grown up'".

<!-- site: filter -->

## Keywords

| Jawa | Literally | JavaScript / TypeScript | Example |
| --- | --- | --- | --- |
| `gawe` | make, do | `function` | `gawe tikel(n: wilangan): wilangan { … }` |
| `ayo` | let's | `let` | `ayo umur = 20` |
| `tetep` | stays the same | `const` | `tetep jeneng = "Sri"` |
| `yen` | if, when | `if` | `yen (n > 0) { … }` |
| `liyane` | the other, otherwise | `else` | `} liyane { … }` |
| `sajrone` | during, while | `while` | `sajrone (n > 0) { … }` |
| `kanggo` | for | `for` | `kanggo (ayo i = 0; i < 3; i++) { … }` |
| `ing` | in, at | `of` in `for … of` | `kanggo x ing larik { … }` |
| `mandheg` | stop | `break` | `mandheg` |
| `terusna` | carry on! | `continue` | `terusna` |
| `bali` | go back, return | `return` | `bali n * 2` |
| `wujud` | shape, form | `interface`, a class with only fields | `wujud Titik { x: pecahan; y: pecahan }` |

## Values and operators

| Jawa | Literally | JavaScript / TypeScript | Example |
| --- | --- | --- | --- |
| `bener` | right, correct | `true` | `ayo rampung = bener` |
| `salah` | wrong, mistaken | `false` | `yen rampung == salah { … }` |
| `kosong` | empty | `null` | `ayo sirah: Simpul = kosong` |
| `lan` | and | `&&` (which also works) | `umur >= 18 lan duwe_karcis` |
| `utawa` | or | `\|\|` (which also works) | `n < 0 utawa n > 9` |
| `ora` | not, no | `!` (which also works) | `ora rampung` |

`ora` binds more loosely than the comparisons, as `not` does in Python, so `ora a == b` means `ora (a == b)`. `!` binds tightly, as in JavaScript.

## Types

| Jawa | Literally | JavaScript / TypeScript | Example |
| --- | --- | --- | --- |
| `wilangan` | number, count | `number` (integers: 64-bit) | `ayo cacah: wilangan = 0` |
| `pecahan` | fraction | `number` (floats: 64-bit) | `ayo dhuwur: pecahan = 1.62` |
| `logika` | logic | `boolean` | `ayo wis: logika = salah` |
| `ukara` | sentence | `string` | `ayo jeneng: ukara = "Sri"` |
| `suwung` | empty, deserted | `void` | `gawe sapa(): suwung { … }` |
| `T[]` or `[T]` | *larik*: row, array | `T[]` | `ayo angka: wilangan[] = []` |

## Built-in functions

| Jawa | Literally | JavaScript / TypeScript | Example |
| --- | --- | --- | --- |
| `utama` | main, foremost | the entry point | `gawe utama() { … }` |
| `tulis` | write | `console.log` | `tulis("umur:", umur)` |
| `dawane` | its length | `.length` | `dawane(larik)` |
| `tambah` | add | `.push()` | `tambah(larik, 4)` |
| `gagal` | fail | `throw new Error()` | `gagal("ora ketemu")` |
| `wilangan(x)` | number | `Math.trunc(x)`, `parseInt(s)` | `wilangan("42")` |
| `pecahan(x)` | fraction | `Number(x)`, `parseFloat(s)` | `pecahan(3)` |
| `ukara(x)` | sentence | `String(x)` | `ukara(3.5)` |
| `logika(x)` | logic | `Boolean(n)` | `logika(1)` |

## Runtime messages

When a program stops with an error, it prints one of these to standard error:

| Message | Meaning |
| --- | --- |
| `gagal: …` | failed: … (every message starts this way) |
| `indeks 3 ora ana ing larik sing dawane 3` | index 3 does not exist in an array whose length is 3 |
| `` `kosong` ora duwe isi `` | `kosong` has no contents (a field of `kosong` was used) |
| `dibagi nol` | divided by zero |
| `"abc" dudu wilangan` | "abc" is not a `wilangan` |
| `"abc" dudu pecahan` | "abc" is not a `pecahan` |
| `memori entek` | the memory is used up |

## Words in the examples

These are not part of the language, just names the examples and guides use. They make handy names in your own programs.

| Jawa | Meaning |
| --- | --- |
| `angka` | digit, number |
| `asil` | result |
| `baris` | row, line |
| `bener`, `salah` | right, wrong |
| `cacah` | count, amount |
| `dawa` | long, length |
| `dhaptar` | list |
| `dhuwur` | tall, height |
| `dina` | day |
| `gunggung` | total, sum |
| `jarak` | distance |
| `jeneng` | name |
| `kanca` | friend |
| `kiwa`, `tengen` | left, right |
| `kolom` | column |
| `kuadrat` | square |
| `langkah` | step |
| `larik` | row; an array |
| `neptu` | the number belonging to a day in the Javanese calendar |
| `pasaran` | the five-day market week: Legi, Pahing, Pon, Wage, Kliwon |
| `regane` | its value |
| `sabanjure` | the next one |
| `sadurunge`, `sawise` | before, after |
| `saiki` | now |
| `simpul` | knot; a node |
| `siji`, `loro`, `telu` | one, two, three |
| `sirah` | head |
| `teks` | text |
| `tikel` | multiple; to double |
| `titik` | point, dot |
| `turah` | left over; remainder |
| `umur` | age |
| `urut` | in order |
| `weton` | the day of birth in the Javanese calendar |
| `wis`, `durung` | already, not yet |
| `wong` | person |

## Greetings

| Jawa | Meaning |
| --- | --- |
| `Sugeng rawuh` | welcome |
| `Sugeng enjing` | good morning |
| `Piye kabare?` | how are you? |
| `Matur nuwun` | thank you |

## Reading Javanese spelling

Javanese is written in the Latin alphabet as it sounds, with a few letters to know:

- **dh** and **th** are a *d* and *t* with the tongue curled back, as in Indian English; they are never the English *th*. Plain **d** and **t** touch the teeth. Compare `dhaptar` and `dina`, `bathang` and `tulis`.
- **ng** is the sound at the end of English *sing*, and can start a word: `nganggo`. **ny** is the *ny* of *canyon*: `nyambung`.
- **c** is the *ch* of *church*: `cacah`. **j** is the *j* of *jam*.
- **e** is either the short, neutral vowel of *about* (`tetep`, `terusna`) or the *e* of *bed* (`gawe`, `yen`). The spelling does not tell them apart.
- **a** at the end of a word, and in the syllable before it when that ends in *a* too, sounds close to the *o* of *pot*: `Jawa` is said "Jowo" and `utawa` "utowo".
