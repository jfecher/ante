# ante-rs

A WIP rewrite of Ante's compiler in rust.

### Why?

The original c++ compiler was written as ante's
design was evolving quickly. In fact, the repository
started as an interpreter for a gradually-typed scripting
language by the name of Zy. Over the years, the
compiler has had many features bolted on it wasn't defined
for - including hindley-milner type inference, a REPL,
and functional dependencies. This has resulted in not
only many bugs but also the complexity has increased
the difficulty of adding new features and fixing existing
bugs.

This new compiler to be built from the ground up with these
features in mind - hopefully enabling a cleaner codebase.
