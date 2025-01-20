//! token.rs - Defines the Token type which represents
//! a single grammatical unit of ante source code. This
//! can be an identifier, type name, string literal, integer
//! literal, operator, etc.
//!
//! Converting a stream of characters into a Vec<Token> is the goal of
//! the lexing phase of the compiler. The resulting tokens are then
//! fed into the parser to verify the program's grammar and create
//! an abstract syntax tree.
use std::fmt::{self, Display};

/// Lexing can fail with these errors, though the Lexer just
/// returns the LexerError inside of an Invalid token which
/// the parser will fail on. Currently the parser fails immediately
/// when it finds these tokens but in the future it may be able
/// to issue the error then continue on to output as many errors
/// as possible.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LexerError {
    InvalidCharacterInSignificantWhitespace(char), // Only spaces are allowed in significant whitespace
    InvalidEscapeSequence(char),
    InvalidIntegerSuffx,
    InvalidFloatSuffx,
    IndentChangeTooSmall, // All indentation changes must be >= 2 spaces in size difference relative to the previous level
    UnindentToNewLevel,   // Unindented to a new indent level rather than returning to a previous one
    Expected(char),
    UnknownChar(char),
}

/// Each Token::IntegerLiteral and Ast::LiteralKind::Integer has
/// an IntegerKind representing the size of the integer.
///
/// Integer literals in ante are polymorphic in the `Int a` type. The 'a'
/// here is a type variable which will later resolve to an IntegerKind once
/// the integer size and sign are known.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerKind {
    I8,
    I16,
    I32,
    I64,
    Isz,
    U8,
    U16,
    U32,
    U64,
    Usz,
}

/// Each float literal is polymorphic over the `Float a` type. The `a` is the
/// specific FloatKind of the float which is later resolved to one of these
/// variants (or kept generic if the code allows).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FloatKind {
    F32,
    F64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Lexer sends an end of input token before stopping so we get a proper error location when
    // reporting parsing errors that expect a token but found the end of a file instead.
    EndOfInput,
    Invalid(LexerError),
    Newline,
    Indent,
    Unindent,

    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(u64, Option<IntegerKind>),
    FloatLiteral(f64, Option<FloatKind>),
    CharLiteral(char),
    BooleanLiteral(bool),
    UnitLiteral,

    // Types
    TypeName(String),
    IntegerType(IntegerKind),
    FloatType(FloatKind),
    PolymorphicIntType,
    PolymorphicFloatType,
    CharType,
    StringType,
    PointerType,
    BooleanType,
    UnitType,
    Mut,

    // Keywords
    And,
    As,
    Block,
    Boxed,
    Do,
    Effect,
    Else,
    Extern,
    Fn,
    Given,
    Handle,
    If,
    Impl,
    Import,
    In,
    Loop,
    Match,
    Methods,
    Module,
    Not,
    Or,
    Owned,
    Ref,
    Return,
    Shared,
    Then,
    Trait,
    Type,
    While,
    With,

    // Operators
    Equal,              // =
    Assignment,         // :=
    EqualEqual,         // ==
    NotEqual,           // !=
    Range,              // ...
    RightArrow,         // ->
    FatArrow,           // =>
    ApplyLeft,          // <|
    ApplyRight,         // |>
    Append,             // ++
    Index,              // #
    Modulus,            // %
    Multiply,           // *
    ParenthesisLeft,    // (
    ParenthesisRight,   // )
    Subtract,           // -
    Add,                // +
    BracketLeft,        // [
    BracketRight,       // ]
    InterpolateLeft,    // ${
    InterpolateRight,   // }
    Pipe,               // |
    Colon,              // :
    Semicolon,          // ;
    Comma,              // ,
    MemberAccess,       // .
    MemberRef,          // .&
    MemberMutRef,       // .!
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
    Divide,             // /
    Backslash,          // \
    Ampersand,          // &
    At,                 // @
    ExclamationMark,    // !
    QuestionMark,       // ?
}

impl Token {
    pub fn is_overloadable_operator(&self) -> bool {
        use Token::*;
        matches!(
            self,
            And | As
                | At
                | In
                | Not
                | Or
                | EqualEqual
                | NotEqual
                | ApplyLeft
                | ApplyRight
                | Append
                | Index
                | Modulus
                | Multiply
                | Comma
                | Subtract
                | Add
                | LessThan
                | GreaterThan
                | LessThanOrEqual
                | GreaterThanOrEqual
                | Divide
                | Range
        )
    }
}

impl Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LexerError::*;
        match self {
            InvalidCharacterInSignificantWhitespace(c) => {
                let char_str = if *c == '\t' {
                    "a tab".to_string()
                } else {
                    format!("U+{:x}", *c as u32)
                };
                write!(f, "Only spaces are allowed in significant whitespace, {} is not allowed here", char_str)
            },
            InvalidEscapeSequence(c) => write!(f, "Invalid character in escape sequence: '{}' (U+{:x})", c, *c as u32),
            InvalidIntegerSuffx => write!(f, "Invalid suffix after integer literal. Expected an integer type like i32 or a space to separate the two tokens"),
            InvalidFloatSuffx => write!(f, "Invalid suffix after float literal. Expected either 'f', 'f32', 'f64', or a space to separate the two tokens"),
            IndentChangeTooSmall => write!(f, "This indent/unindent is too small, it should be at least 2 spaces apart from the previous indentation level"),
            UnindentToNewLevel => write!(f, "This unindent doesn't return to any previous indentation level"),
            Expected(c) => write!(f, "Expected {} (U+{:x}) while lexing", *c, *c as u32),
            UnknownChar(c) => write!(f, "Unknown character '{}' (U+{:x}) in file", *c, *c as u32),
        }
    }
}

impl Display for IntegerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IntegerKind::*;
        match self {
            I8 => write!(f, "I8"),
            I16 => write!(f, "I16"),
            I32 => write!(f, "I32"),
            I64 => write!(f, "I64"),
            Isz => write!(f, "Isz"),
            U8 => write!(f, "U8"),
            U16 => write!(f, "U16"),
            U32 => write!(f, "U32"),
            U64 => write!(f, "U64"),
            Usz => write!(f, "Usz"),
        }
    }
}

impl Display for FloatKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FloatKind::F32 => write!(f, "F32"),
            FloatKind::F64 => write!(f, "F64"),
        }
    }
}

impl Display for Token {
    /// This formatting is shown when the parser outputs its
    /// "expected one of ..." tokens list after finding a syntax error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::EndOfInput => write!(f, "end of input"),
            Token::Invalid(error) => write!(f, "{:?}", error),
            Token::Newline => write!(f, "a newline"),
            Token::Indent => write!(f, "an indent"),
            Token::Unindent => write!(f, "an unindent"),

            Token::Identifier(_) => write!(f, "an identifier"),
            Token::StringLiteral(_) => write!(f, "a string literal"),
            Token::IntegerLiteral(_, _) => write!(f, "an integer literal"),
            Token::FloatLiteral(_, _) => write!(f, "a float literal"),
            Token::CharLiteral(_) => write!(f, "a char literal"),
            Token::BooleanLiteral(_) => write!(f, "a boolean literal"),
            Token::UnitLiteral => write!(f, "'()'"),

            // Types
            Token::TypeName(_) => write!(f, "a typename"),
            Token::IntegerType(kind) => write!(f, "'{}'", kind),
            Token::FloatType(kind) => write!(f, "'{}'", kind),
            Token::PolymorphicIntType => write!(f, "'Int'"),
            Token::PolymorphicFloatType => write!(f, "'Float'"),
            Token::CharType => write!(f, "'char'"),
            Token::StringType => write!(f, "'string'"),
            Token::PointerType => write!(f, "'Ptr'"),
            Token::BooleanType => write!(f, "'bool'"),
            Token::UnitType => write!(f, "'unit'"),
            Token::Mut => write!(f, "'mut'"),

            // Keywords
            Token::And => write!(f, "'and'"),
            Token::As => write!(f, "'as'"),
            Token::Block => write!(f, "'block'"),
            Token::Boxed => write!(f, "'boxed'"),
            Token::Do => write!(f, "'do'"),
            Token::Effect => write!(f, "'effect'"),
            Token::Else => write!(f, "'else'"),
            Token::Extern => write!(f, "'extern'"),
            Token::Fn => write!(f, "'fn'"),
            Token::Given => write!(f, "'given'"),
            Token::Handle => write!(f, "'handle'"),
            Token::If => write!(f, "'if'"),
            Token::Impl => write!(f, "'impl'"),
            Token::Import => write!(f, "'import'"),
            Token::In => write!(f, "'in'"),
            Token::Loop => write!(f, "'loop'"),
            Token::Match => write!(f, "'match'"),
            Token::Methods => write!(f, "'methods'"),
            Token::Module => write!(f, "'module'"),
            Token::Not => write!(f, "'not'"),
            Token::Or => write!(f, "'or'"),
            Token::Owned => write!(f, "'owned'"),
            Token::Return => write!(f, "'return'"),
            Token::Ref => write!(f, "'ref'"),
            Token::Shared => write!(f, "'shared'"),
            Token::Then => write!(f, "'then'"),
            Token::Trait => write!(f, "'trait'"),
            Token::Type => write!(f, "'type'"),
            Token::While => write!(f, "'while'"),
            Token::With => write!(f, "'with'"),

            // Operators
            Token::Equal => write!(f, "'='"),
            Token::Assignment => write!(f, "':='"),
            Token::EqualEqual => write!(f, "'=='"),
            Token::NotEqual => write!(f, "'!='"),
            Token::Range => write!(f, "'..'"),
            Token::RightArrow => write!(f, "'->'"),
            Token::FatArrow => write!(f, "'=>'"),
            Token::ApplyLeft => write!(f, "'<|'"),
            Token::ApplyRight => write!(f, "'|>'"),
            Token::Append => write!(f, "'++'"),
            Token::Index => write!(f, "'#'"),
            Token::Modulus => write!(f, "'%'"),
            Token::Multiply => write!(f, "'*'"),
            Token::ParenthesisLeft => write!(f, "'('"),
            Token::ParenthesisRight => write!(f, "')'"),
            Token::Subtract => write!(f, "'-'"),
            Token::Add => write!(f, "'+'"),
            Token::BracketLeft => write!(f, "'['"),
            Token::BracketRight => write!(f, "']'"),
            Token::InterpolateLeft => write!(f, "'${{'"),
            Token::InterpolateRight => write!(f, "'}}'"),
            Token::Pipe => write!(f, "'|'"),
            Token::Colon => write!(f, "':'"),
            Token::Semicolon => write!(f, "';'"),
            Token::Comma => write!(f, "','"),
            Token::MemberAccess => write!(f, "'.'"),
            Token::MemberRef => write!(f, "'.&'"),
            Token::MemberMutRef => write!(f, "'.!'"),
            Token::LessThan => write!(f, "'<'"),
            Token::GreaterThan => write!(f, "'>'"),
            Token::LessThanOrEqual => write!(f, "'<='"),
            Token::GreaterThanOrEqual => write!(f, "'>='"),
            Token::Divide => write!(f, "'/'"),
            Token::Backslash => write!(f, "'\\'"),
            Token::Ampersand => write!(f, "'&'"),
            Token::At => write!(f, "'@'"),
            Token::ExclamationMark => write!(f, "'!'"),
            Token::QuestionMark => write!(f, "'?'"),
        }
    }
}

pub fn lookup_keyword(word: &str) -> Option<Token> {
    match word {
        "I8" => Some(Token::IntegerType(IntegerKind::I8)),
        "I16" => Some(Token::IntegerType(IntegerKind::I16)),
        "I32" => Some(Token::IntegerType(IntegerKind::I32)),
        "I64" => Some(Token::IntegerType(IntegerKind::I64)),
        "Isz" => Some(Token::IntegerType(IntegerKind::Isz)),
        "U8" => Some(Token::IntegerType(IntegerKind::U8)),
        "U16" => Some(Token::IntegerType(IntegerKind::U16)),
        "U32" => Some(Token::IntegerType(IntegerKind::U32)),
        "U64" => Some(Token::IntegerType(IntegerKind::U64)),
        "Usz" => Some(Token::IntegerType(IntegerKind::Usz)),
        "F32" => Some(Token::FloatType(FloatKind::F32)),
        "F64" => Some(Token::FloatType(FloatKind::F64)),
        "Int" => Some(Token::PolymorphicIntType),
        "Float" => Some(Token::PolymorphicFloatType),
        "Char" => Some(Token::CharType),
        "String" => Some(Token::StringType),
        "Ptr" => Some(Token::PointerType),
        "Bool" => Some(Token::BooleanType),
        "Unit" => Some(Token::UnitType),
        "mut" => Some(Token::Mut),
        "true" => Some(Token::BooleanLiteral(true)),
        "false" => Some(Token::BooleanLiteral(false)),
        "and" => Some(Token::And),
        "as" => Some(Token::As),
        "block" => Some(Token::Block),
        "boxed" => Some(Token::Boxed),
        "do" => Some(Token::Do),
        "effect" => Some(Token::Effect),
        "else" => Some(Token::Else),
        "extern" => Some(Token::Extern),
        "fn" => Some(Token::Fn),
        "given" => Some(Token::Given),
        "handle" => Some(Token::Handle),
        "if" => Some(Token::If),
        "impl" => Some(Token::Impl),
        "import" => Some(Token::Import),
        "in" => Some(Token::In),
        "loop" => Some(Token::Loop),
        "match" => Some(Token::Match),
        "methods" => Some(Token::Methods),
        "module" => Some(Token::Module),
        "not" => Some(Token::Not),
        "or" => Some(Token::Or),
        "owned" => Some(Token::Owned),
        "ref" => Some(Token::Ref),
        "return" => Some(Token::Return),
        "shared" => Some(Token::Shared),
        "then" => Some(Token::Then),
        "trait" => Some(Token::Trait),
        "type" => Some(Token::Type),
        "while" => Some(Token::While),
        "with" => Some(Token::With),
        _ => None,
    }
}
