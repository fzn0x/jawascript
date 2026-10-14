//! The Jawa compiler driver.
//!
//! `jawac` is a C++ executable (tools/jawac/main.cpp) whose `main` calls
//! [`jawac_main`]. The Rust side runs the frontend, generates the `jawa`
//! dialect through the cxx bridge, and links the result; the C++ side owns
//! MLIR and LLVM.

mod bridge;
mod builder;
mod codegen;
mod driver;
mod link;

use std::ffi::c_int;

/// The entry point the C++ `main` calls.
#[no_mangle]
pub extern "C" fn jawac_main() -> c_int {
    // The parser and type checker recurse once per nesting level; run them
    // with a stack far larger than any program needs.
    let result = std::thread::Builder::new()
        .name("jawac".to_string())
        .stack_size(64 << 20)
        .spawn(driver::main)
        .map(|handle| handle.join());
    match result {
        Ok(Ok(code)) => code,
        _ => {
            eprintln!(
                "error: internal compiler error\n  = note: please report this at https://github.com/fzn0x/jawascript/issues"
            );
            101
        }
    }
}
