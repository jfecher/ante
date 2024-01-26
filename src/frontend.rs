use std::path::Path;

use crate::cache::ModuleCache;
use crate::lexer::Lexer;
use crate::nameresolution::NameResolver;
use crate::parser;
use crate::types;
use crate::util;

#[derive(PartialEq)]
pub enum FrontendPhase {
    Lex,
    Parse,
    TypeCheck,
}

pub enum FrontendResult {
    Done,
    ContinueCompilation,
    Errors,
}

pub fn check<'a>(
    filename: &'a Path, main_file_contents: String, cache: &mut ModuleCache<'a>, phase: FrontendPhase, show_time: bool,
) -> FrontendResult {
    util::timing::time_passes(show_time);

    // Phase 1: Lexing
    util::timing::start_time("Lexing");
    let tokens = Lexer::new(filename, &main_file_contents).collect::<Vec<_>>();

    if phase == FrontendPhase::Lex {
        tokens.iter().for_each(|(token, _)| println!("{}", token));
        return FrontendResult::Done;
    }

    // Phase 2: Parsing
    util::timing::start_time("Parsing");

    let root = match parser::parse(&tokens) {
        Ok(root) => root,
        Err(parse_error) => {
            // Parse errors are currently always fatal
            cache.push_full_diagnostic(parse_error.into_diagnostic());
            return FrontendResult::Errors;
        },
    };

    if phase == FrontendPhase::Parse {
        println!("{}", root);
        return FrontendResult::Done;
    }

    // Phase 3: Name resolution
    // Timing for name resolution is within the start method to
    // break up the declare and define passes
    NameResolver::start(root, cache);

    if cache.error_count() != 0 {
        return FrontendResult::Errors;
    }

    // Phase 4: Type inference
    util::timing::start_time("Type Inference");
    let ast = cache.parse_trees.get_mut(0).unwrap();
    types::typechecker::infer_ast(ast, cache);

    if cache.error_count() != 0 {
        FrontendResult::Errors
    } else {
        FrontendResult::ContinueCompilation
    }
}
