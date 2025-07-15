use std::path::PathBuf;

use inc_complete::{define_input, define_intermediate, impl_storage, storage::HashMapStorage};

use crate::parser::{ast::Ast, error::ParseError};

pub type Db = inc_complete::Db<Context>;
pub type DbHandle = inc_complete::DbHandle<Context>;

#[derive(Default)]
struct Context {
    files: HashMapStorage<File>,
}
impl_storage!(Context,
    files: File,
);

/////////////////////////////////////////////////////////////////////////
/// The only input to the compiler currently is each file used.
/// Maps a file's path to the full source text.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct File {
    pub path: PathBuf,
}
define_input!(0, File -> String, Context);


/////////////////////////////////////////////////////////////////////////
/// Parse a file, returning a parse tree along with any errors.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Parse {
    pub path: PathBuf,
}
define_intermediate!(1, Parse -> Result<Ast, ParseError>, Context, crate::parser::parse);
