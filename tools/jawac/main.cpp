//===- main.cpp - The jawac executable ------------------------------------===//
//
// Part of JawaScript, under the MIT License. See LICENSE for details.
//
//===----------------------------------------------------------------------===//
//
// The driver is written in Rust (crates/jawac); this file only gives the
// executable a C++ `main`, so the C++ backend and the Rust frontend link into
// one program.
//
//===----------------------------------------------------------------------===//

extern "C" int jawac_main(void);

int main() { return jawac_main(); }
