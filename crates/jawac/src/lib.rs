//! The Jawa compiler driver.
//!
//! The Rust side runs the frontend and generates the `jawa` dialect through
//! the cxx bridge; the C++ side owns MLIR and LLVM.

mod bridge;
mod builder;
mod codegen;
