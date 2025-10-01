use std::path::PathBuf;

use clap::{Parser, ValueEnum, ValueHint};
use clap_complete::Shell;

#[derive(Parser, Debug)]
pub struct Completions {
    #[arg(long)]
    pub shell_completion: Shell,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the source files
    #[arg(value_hint=ValueHint::FilePath)]
    pub files: Vec<PathBuf>,

    /// Print out the input file annotated with inferred lifetimes of heap allocations
    #[arg(long, short = 'L')]
    pub show_lifetimes: bool,

    /// Lex the file and output the resulting list of tokens
    #[arg(long, group = "compile_mode")]
    pub show_tokens: bool,

    /// Parse the file and output the resulting Ast
    #[arg(long, group = "compile_mode")]
    pub show_parse: bool,

    /// Resolve the file and show the resulting resolved Ast
    #[arg(long, short, group = "compile_mode")]
    pub show_resolved: bool,

    /// Type check the file and show the resulting typed Ast
    #[arg(long, group = "compile_mode")]
    pub show_types: bool,

    /// Check the file for errors without compiling
    #[arg(long, short, group = "compile_mode")]
    pub check: bool,

    /// Build the resulting binary without running it afterward
    #[arg(long, short, group = "compile_mode")]
    pub build: bool,

    /// Tells the compiler to create something other than an executable
    #[arg(long, short, group = "compile_mode")]
    pub emit: Option<EmitTarget>,

    /// Specify the backend to use ('llvm' or 'cranelift'). Note that cranelift is only for debug builds.
    /// Ante will use cranelift by default for debug builds and llvm by default for optimized builds,
    /// unless overridden by this flag
    #[arg(long)]
    pub backend: Option<Backend>,

    /// Sets the current optimization level from 0 (no optimization) to 3 (aggressive optimization).
    /// Set to s or z to optimize for size.
    #[arg(short = 'O', default_value = "0", value_parser = validate_opt_argument)]
    pub opt_level: char,

    /// Use plaintext and an indicator line instead of color for pointing out error locations
    #[arg(long)]
    pub no_color: bool,

    /// Delete the resulting binary after compiling
    #[arg(long, short, group = "compile_mode")]
    pub delete_binary: bool,

    /// Print out the time each compiler pass takes for the given program
    #[arg(long)]
    pub show_time: bool,

    /// Enable incremental compilation by reading from and writing to metadata for the current program
    #[arg(long, short = 'i')]
    pub incremental: bool,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
pub enum EmitTarget {
    /// LLVM-IR or Cranelift IR depending on the selected backend
    Ir,

    /// Ante's post-monomorphisation HIR representation
    Hir,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
pub enum Backend {
    Cranelift,
    Llvm,
}

fn validate_opt_argument(arg: &str) -> Result<char, &'static str> {
    match arg {
        "0" | "1" | "2" | "3" | "s" | "z" => Ok(arg.chars().next().unwrap()),
        _ => Err("Argument to -O must be one of: 0, 1, 2, 3, s, or z"),
    }
}
