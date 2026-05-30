#![allow(mismatched_lifetime_syntaxes)]

mod definition_collection;
pub mod find_files;
pub mod lexer;
pub mod name_resolution;
pub mod parser;
pub mod type_inference;

// Util modules:
pub mod diagnostics;
pub mod files;
pub mod incremental;
mod iterator_extensions;
pub mod paths;
pub mod timings;
pub mod vecmap;
