use clap::{ArgGroup, Parser, ValueEnum, ValueHint};
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(group(
        ArgGroup::new("complete_compile")
        .required(true),
))]
pub struct Cli {
    /// Generate shell completion for a given shell
    #[arg(long, group = "complete_compile")]
    pub shell_completion: Option<Shell>,

    /// Path to the source file
    #[arg(group = "complete_compile", value_hint=ValueHint::FilePath)]
    pub file: Option<String>,

    /// Print out the input file annotated with inferred lifetimes of heap allocations
    #[arg(long, short = 'L')]
    pub show_lifetimes: bool,

    /// Lex the file and output the resulting list of tokens
    #[arg(long, short, group = "compile_mode")]
    pub lex: bool,

    /// Parse the file and output the resulting Ast
    #[arg(long, short, group = "compile_mode")]
    pub parse: bool,

    /// Check the file for errors without compiling
    #[arg(long, short, group = "compile_mode")]
    pub check: bool,

    /// Build the resulting binary without running it afterward
    #[arg(long, short, group = "compile_mode")]
    pub build: bool,

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

    /// Print out the LLVM-IR or Cranelift IR of the compiled program
    #[arg(long, short = 'i')]
    pub show_ir: bool,

    /// Print out the HIR, Ante's post-monomorphisation IR
    #[arg(long, short = 'H')]
    pub show_hir: bool,

    /// Delete the resulting binary after compiling
    #[arg(long, short, group = "compile_mode")]
    pub delete_binary: bool,

    /// Print out the time each compiler pass takes for the given program
    #[arg(long)]
    pub show_time: bool,

    /// Print out the type of each definition
    #[arg(long, short = 't')]
    pub show_types: bool,
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
