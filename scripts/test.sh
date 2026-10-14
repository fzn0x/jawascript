#!/usr/bin/env bash
# Builds JawaScript and runs every test: the frontend's Rust unit tests and
# the lit suite (dialect, passes, lowering, diagnostics, compiled programs,
# examples and the command line).
#
#   scripts/test.sh                   # everything
#   scripts/test.sh -v                # extra arguments go to lit
#   scripts/test.sh --filter Run/     # only some lit tests
#
# lit comes from PATH if it is installed, or from a virtualenv this script
# creates in the build directory. On Windows, run it from the MSYS2 UCRT64
# shell.
set -euo pipefail
cd "$(dirname "$0")/.."

build_dir=${BUILD_DIR:-build}

echo "==> Building"
scripts/build.sh

echo "==> Rust unit tests"
cargo test --package jawa-frontend --quiet

echo "==> lit tests"
lit=$(command -v lit || command -v llvm-lit || true)
if [ -z "$lit" ]; then
  venv="$build_dir/.venv"
  for candidate in "$venv/bin/lit" "$venv/bin/lit.exe" "$venv/Scripts/lit.exe"; do
    [ -x "$candidate" ] && lit=$candidate && break
  done
  if [ -z "$lit" ]; then
    echo "Installing lit into $venv"
    python3 -m venv "$venv"
    for python in "$venv/bin/python" "$venv/bin/python.exe" "$venv/Scripts/python.exe"; do
      [ -x "$python" ] && break
    done
    "$python" -m pip install --quiet lit
    for candidate in "$venv/bin/lit" "$venv/bin/lit.exe" "$venv/Scripts/lit.exe"; do
      [ -x "$candidate" ] && lit=$candidate && break
    done
  fi
fi
"$lit" -sv "$build_dir/tests" "$@"
