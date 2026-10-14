# Contributing

Thanks for helping improve JawaScript. This guide covers how to report problems, propose changes, and get a pull request merged.

## Ways to help

- **Report a bug.** Open an issue with the smallest Jawa program that shows the problem, what you expected, and what happened: the compiler's output, the program's output, or the crash. A failing program is the most useful thing you can send.
- **Suggest a feature.** Open an issue describing what you want to write before you write code, so we can agree on the approach. New keywords should be Javanese (ngoko) and easy to read; say what the word means.
- **Improve the docs or the examples.** Small fixes can go straight to a pull request. Examples are welcome, especially ones that show Javanese culture the way `weton.jw` and `aksara.jw` do.
- **Report a security problem** privately, as described in [SECURITY.md](SECURITY.md). Please don't use a public issue.

## Development setup

Install the dependencies listed in the [README](README.md#building-from-source), then:

```sh
git clone https://github.com/fzn0x/jawascript.git
cd jawascript
scripts/test.sh          # builds into ./build and runs every test
```

`scripts/test.sh` passes extra arguments to lit, so `scripts/test.sh --filter Dialect` runs only the dialect tests. While working on the frontend alone, `cargo test -p jawa-frontend` is enough and needs no LLVM.

Useful while debugging:

```sh
build/bin/jawac --emit=mlir file.jw         # what the frontend builds
build/bin/jawac --emit=mlir-opt file.jw     # after the Jawa passes
build/bin/jawa-opt file.mlir -jawa-flatten-cfg -mlir-print-ir-after-all
```

## Project layout

- `crates/jawa-frontend`: lexer, parser, type checker and HIR. Most language changes start here, with unit tests.
- `crates/jawac`: the driver, the HIR-to-MLIR code generator, and the Rust half of the cxx bridge.
- `include/jawa`, `lib`: the `jawa` dialect (TableGen and C++), its passes, the lowering to LLVM, the pipeline, and the C++ half of the bridge.
- `runtime`: the C runtime. Its data layouts must match `lib/Conversion/JawaToLLVM`.
- `tests`: the lit suite. `docs/architecture.md` explains how the pieces fit.

## Ground rules

1. **Every change comes with tests.** Frontend behaviour gets Rust unit tests. Anything visible in IR, diagnostics or program output gets a lit test in the matching `tests/` directory.
2. **Programs never hit undefined behaviour.** Generated code must not rely on anything LLVM treats as undefined or poison. A mistake the compiler cannot catch becomes a clear runtime error (`gagal: …`), never a crash or silent corruption.
3. **Operations that can stop the program say so.** A new op that can trap must report a side effect, so the optimizer never deletes it.
4. **Every op has a verifier and a round-trip test.** Malformed IR should be rejected with a message that says what is wrong.
5. **Diagnostics are part of the language.** Messages are lowercase and specific, point at the right span, and add a `note` when there is an obvious fix. A mistake should produce one error, not a cascade.
6. **Same result at every optimization level.** `tests/Run` runs programs at `-O0` and `-O2`; keep them agreeing.
7. **No new dependencies without discussion.** The frontend has none, and the compiler needs only LLVM/MLIR, cxx and bdwgc.

## Making a change

1. Fork the repository and create a branch from `main`, for example `fix/range-overflow` or `feat/string-index`.
2. Make your change, with tests.
3. Run `scripts/test.sh` and make sure everything passes.
4. Open a pull request against `main` and fill in the template. Describe what changed and why.

Keep pull requests focused: one fix or feature each. Use plain, imperative commit messages, such as `Check the loop variable is not assigned`.

## Code style

- **Rust:** `cargo fmt`, and `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
- **C++ and C:** LLVM style; `.clang-format` covers it. Follow MLIR conventions: `Op::create(builder, loc, …)`, ODS for anything TableGen can express, and a comment where the *why* isn't obvious.
- Name things for what they do, keep functions small, and match the surrounding code rather than introducing a new pattern.

## Conduct

Be kind and assume good faith. Harassment, insults and personal attacks aren't welcome in issues, pull requests or discussions. The maintainer may remove comments or block participants who don't follow this.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
