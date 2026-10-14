//! The front half of the Jawa compiler: lexing, parsing and type checking.

pub mod ast;
pub mod diag;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod source;
pub mod token;
