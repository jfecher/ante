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
    #[arg(value_hint = ValueHint::FilePath, required = true)]
    pub files: Vec<PathBuf>,

    /// Print out the input file annotated with inferred lifetimes of heap allocations
    #[arg(long, short = 'L')]
    pub show_lifetimes: bool,

    /// Check the file for errors without compiling
    #[arg(long, short, group = "compile_mode")]
    pub check: bool,

    /// Build the resulting binary without running it afterward
    #[arg(long, short, group = "compile_mode")]
    pub build: bool,

    /// Build with suggested optimizations
    #[arg(long, short, group = "compile_mode")]
    pub release: bool,

    /// Tells the compiler to create something other than an executable
    #[arg(long, short, group = "compile_mode")]
    pub emit: Option<EmitTarget>,

    /// If set, all crates will be included in the `emit` output rather than just the local crate
    #[arg(long)]
    pub emit_all: bool,

    /// Specify the backend to use ('llvm', 'c', or 'cranelift'). Note that cranelift is only for debug builds and is currently unimplemented.
    /// The default priority for each backend is:
    /// - debug: cranelift > llvm > c
    /// - release: llvm > c
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
    /// The tokens issued from the lexer
    Tokens,

    /// The parse tree
    Ast,

    /// The parse tree annotated with the origin of each name
    AstR,

    /// The parse tree annotated with the type of all names
    AstT,

    /// A representation of the program with simpler control-flow created after type checking
    Mir,

    /// Mir after tail-resume calls have been optimized out
    MirTail,

    /// Monomorphized Mir
    MirMono,

    /// LLVM-IR or Cranelift IR depending on the selected backend
    Ir,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, ValueEnum)]
pub enum Backend {
    Cranelift,
    Llvm,
    C,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

impl OptLevel {
    #[cfg(feature = "llvm")]
    pub fn as_passes_string(self) -> &'static str {
        match self {
            OptLevel::O0 => "default<O0>",
            OptLevel::O1 => "default<O1>",
            OptLevel::O2 => "default<O2>",
            OptLevel::O3 => "default<O3>",
            OptLevel::Os => "default<Os>",
            OptLevel::Oz => "default<Oz>",
        }
    }

    #[cfg(feature = "llvm")]
    pub fn inkwell(self) -> inkwell::OptimizationLevel {
        use inkwell::OptimizationLevel;
        match self {
            OptLevel::O0 => OptimizationLevel::None,
            OptLevel::O1 => OptimizationLevel::Less,
            OptLevel::O2 | OptLevel::Os | OptLevel::Oz => OptimizationLevel::Default,
            OptLevel::O3 => OptimizationLevel::Aggressive,
        }
    }

    pub fn as_cc_opt_string(&self) -> &'static str {
        match self {
            OptLevel::O0 => "-O0",
            OptLevel::O1 => "-O1",
            OptLevel::O2 => "-O2",
            OptLevel::O3 => "-O3",
            OptLevel::Os => "-Os",
            OptLevel::Oz => "-Oz",
        }
    }
}

fn validate_opt_argument(arg: &str) -> Result<char, &'static str> {
    match arg {
        "0" | "1" | "2" | "3" | "s" | "z" => Ok(arg.chars().next().unwrap()),
        _ => Err("Argument to -O must be one of: 0, 1, 2, 3, s, or z"),
    }
}
