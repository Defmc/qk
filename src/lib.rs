// miette's code generator is poorly handling the unused variables. Such thing is causing the
// compiler to generate warnings that I can't suppress in the struct-level. A fix is coming. Until
// then, a crate-level flag is necessary.
// https://github.com/rust-lang/rust/issues/147648
#![allow(unused_assignments)]

pub mod arts;
pub mod ast;
pub mod compiler;
pub mod cpu;
pub mod ir;
pub mod lexer;
pub mod parser;
