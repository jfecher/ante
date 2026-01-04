#![allow(mismatched_lifetime_syntaxes)]

mod codegen;
mod definition_collection;
mod find_files;
mod lexer;
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
mod paths;
mod vecmap;
