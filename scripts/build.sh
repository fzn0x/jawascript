#!/usr/bin/env bash
# Configures (once) and builds JawaScript into ./build.
#
#   scripts/build.sh                  # Release build in ./build
#   BUILD_TYPE=Debug scripts/build.sh
#   BUILD_DIR=out CMAKE_ARGS="-DMLIR_DIR=/opt/llvm/lib/cmake/mlir" scripts/build.sh
#
# On Windows, run it from the MSYS2 UCRT64 shell.
set -euo pipefail
cd "$(dirname "$0")/.."

build_dir=${BUILD_DIR:-build}
build_type=${BUILD_TYPE:-Release}

if [ ! -f "$build_dir/CMakeCache.txt" ]; then
  args=(-G Ninja -DCMAKE_BUILD_TYPE="$build_type")
  if [ -n "${MSYSTEM:-}" ]; then
    # MSYS2's MLIR libraries are built with Clang and linked with LLD;
    # GCC objects and GNU ld do not link against them.
    args+=(-DCMAKE_C_COMPILER=clang -DCMAKE_CXX_COMPILER=clang++ -DLLVM_USE_LINKER=lld)
  fi
  # shellcheck disable=SC2086 # CMAKE_ARGS is a list of words on purpose.
  cmake -S . -B "$build_dir" "${args[@]}" ${CMAKE_ARGS:-}
fi
cmake --build "$build_dir"
