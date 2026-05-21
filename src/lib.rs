#![allow(mismatched_lifetime_syntaxes)]

pub mod codegen;
mod definition_collection;
pub mod find_files;
pub mod lexer;
mod mir;
pub mod name_resolution;
pub mod parser;
pub mod type_inference;

// Util modules:
mod cli;
pub mod diagnostics;
pub mod files;
pub mod incremental;
mod iterator_extensions;
pub mod paths;
pub mod timings;
mod vecmap;
