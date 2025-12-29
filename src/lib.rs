#![allow(mismatched_lifetime_syntaxes)]

mod backend;
mod definition_collection;
mod find_files;
mod lexer;
pub mod name_resolution;
mod parser;
pub mod type_inference;
mod codegen;
mod mir;

// Util modules:
mod cli;
pub mod diagnostics;
pub mod files;
pub mod incremental;
mod iterator_extensions;
mod paths;
mod vecmap;
