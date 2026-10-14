## What and why

<!-- What does this change, and what problem does it solve? Link the issue if there is one. -->

## How I checked it

- [ ] `scripts/test.sh` passes
- [ ] New behaviour has tests: Rust unit tests for the frontend, lit tests for IR, diagnostics or program output
- [ ] Programs print the same at `-O0` and `-O2`
- [ ] `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` are clean, and C++ follows `.clang-format`
- [ ] The docs (`docs/language.md`, `docs/architecture.md`, README) match the change
