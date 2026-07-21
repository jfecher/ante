use std::{ffi::OsString, path::PathBuf};

use clap::{Args, CommandFactory, Parser, ValueEnum, ValueHint, error::ErrorKind};
use clap_complete::Shell;

#[derive(Parser, Debug)]
pub struct Completions {
    #[arg(long)]
    pub shell_completion: Shell,
}
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Build(BuildArgs),
    Run(RunArgs),
    /// Clone a git dependency into this project's dependency directory
    Add(AddArgs),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub file: FileArgs,
}

impl Cli {
    pub fn validate(self) -> Result<Self, clap::Error> {
        if self.command.is_some() && self.file.has_arguments() {
            Err(Cli::command().error(
                ErrorKind::ArgumentConflict,
                "file and compiler arguments before a subcommand cannot be combined with a subcommand; place build and run options after the subcommand",
            ))
        } else {
            Ok(self)
        }
    }
}

#[derive(Args, Debug)]
pub struct FileArgs {
    /// Path to the source files
    #[arg(value_hint = ValueHint::FilePath)]
    pub files: Vec<PathBuf>,

    #[command(flatten)]
    pub compile: CompileArgs,

    /// Check the file for errors without compiling
    #[arg(long, short, group = "compile_mode")]
    pub check: bool,

    /// Build the resulting binary without running it afterward
    #[arg(long, short, group = "compile_mode")]
    pub build: bool,

    /// Tells the compiler to create something other than an executable
    #[arg(long, short, group = "compile_mode")]
    pub emit: Option<EmitTarget>,

    /// If set, all crates will be included in the `emit` output rather than just the local crate
    #[arg(long)]
    pub emit_all: bool,

    /// Delete the resulting binary after compiling
    #[arg(long, short, group = "compile_mode")]
    pub delete_binary: bool,
}

impl FileArgs {
    fn has_arguments(&self) -> bool {
        !self.files.is_empty()
            || self.compile.has_arguments()
            || self.check
            || self.build
            || self.emit.is_some()
            || self.emit_all
            || self.delete_binary
    }
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    #[command(flatten)]
    pub compile: CompileArgs,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[command(flatten)]
    pub compile: CompileArgs,

    /// Delete the resulting binary after running it
    #[arg(long, short)]
    pub delete_binary: bool,

    /// Arguments to pass to the compiled program
    #[arg(last = true, value_name = "ARGS")]
    pub program_args: Vec<OsString>,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Git repository URL to add as a dependency
    #[arg(value_hint = ValueHint::Url)]
    pub dep_url: String,
}

#[derive(Args, Debug)]
pub struct CompileArgs {
    /// Build with suggested optimizations
    #[arg(long, short)]
    pub release: bool,

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

    /// Print out the time each compiler pass takes for the given program
    #[arg(long)]
    pub show_time: bool,

    /// Enable incremental compilation by reading from and writing to metadata for the current program
    #[arg(long, short = 'i')]
    pub incremental: bool,

    /// Path to the file containing the `main` function to use as an entry-point
    #[arg(long, value_name = "NAME")]
    pub bin: Option<String>,

    /// Link the resulting binary against the given native library. May be repeated.
    #[arg(long = "link-lib", short = 'l', value_name = "LIB")]
    pub link_lib: Vec<String>,

    /// Add a directory to the native library search path when linking. May be repeated.
    #[arg(long = "link-search", short = 'L', value_name = "PATH", value_hint = ValueHint::DirPath)]
    pub link_search: Vec<PathBuf>,
}

impl CompileArgs {
    fn has_arguments(&self) -> bool {
        self.release
            || self.backend.is_some()
            || self.opt_level != '0'
            || self.no_color
            || self.show_time
            || self.incremental
            || self.bin.is_some()
            || !self.link_lib.is_empty()
            || !self.link_search.is_empty()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compile_options_after_build_subcommand() {
        let cli = Cli::try_parse_from(["ante", "build", "--backend", "c", "--release", "-O", "3", "--link-lib", "m"])
            .unwrap();

        let Some(Commands::Build(args)) = cli.command else {
            panic!("expected build command");
        };
        assert_eq!(args.compile.backend, Some(Backend::C));
        assert!(args.compile.release);
        assert_eq!(args.compile.opt_level, '3');
        assert_eq!(args.compile.link_lib, vec!["m"]);
    }

    #[test]
    fn parses_run_options_and_program_arguments() {
        let cli =
            Cli::try_parse_from(["ante", "run", "--backend", "c", "--delete-binary", "--", "42", "--verbose"]).unwrap();

        let Some(Commands::Run(args)) = cli.command else {
            panic!("expected run command");
        };
        assert_eq!(args.compile.backend, Some(Backend::C));
        assert!(args.delete_binary);
        assert_eq!(args.program_args, [OsString::from("42"), OsString::from("--verbose")]);
    }

    #[test]
    fn parses_legacy_file_compilation_options() {
        let cli = Cli::try_parse_from(["ante", "main.an", "--build", "--release", "--backend", "c"]).unwrap();

        assert!(cli.command.is_none());
        assert_eq!(cli.file.files, [PathBuf::from("main.an")]);
        assert!(cli.file.build);
        assert!(cli.file.compile.release);
        assert_eq!(cli.file.compile.backend, Some(Backend::C));
    }

    #[test]
    fn rejects_root_compile_options_before_subcommands() {
        let result = Cli::try_parse_from(["ante", "--backend", "c", "run"]).and_then(Cli::validate);

        assert!(result.is_err());
    }

    #[test]
    fn rejects_compile_options_for_add() {
        let result = Cli::try_parse_from(["ante", "add", "https://example.com/dependency", "--backend", "c"]);

        assert!(result.is_err());
    }
}
