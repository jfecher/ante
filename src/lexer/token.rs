//! token.rs - Defines the Token type which represents
//! a single grammatical unit of ante source code. This
//! can be an identifier, type name, string literal, integer
//! literal, operator, etc.
//!
//! Converting a stream of characters into a Vec<Token> is the goal of
//! the lexing phase of the compiler. The resulting tokens are then
//! fed into the parser to verify the program's grammar and create
//! an abstract syntax tree.
use crate::types::TypeVariableId;
use std::fmt::{self, Display};

/// Lexing can fail with these errors, though the Lexer just
/// returns the LexerError inside of an Invalid token which
/// the parser will fail on. Currently the parser fails immediately
/// when it finds these tokens but in the future it may be able
/// to issue the error then continue on to output as many errors
/// as possible.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LexerError {
    InvalidCharacterInSignificantWhitespace(char), // Only spaces are allowed in significant whitespace
    InvalidEscapeSequence(char),
    InvalidIntegerSuffx,
    IndentChangeTooSmall, // All indentation changes must be >= 2 spaces in size difference relative to the previous level
    UnindentToNewLevel,   // Unindented to a new indent level rather than returning to a previous one
    Expected(char),
    UnknownChar(char),
}

/// Each Token::IntegerLiteral and Ast::LiteralKind::Integer has
/// an IntegerKind representing the size of the integer.
///
/// Integer literals in ante are polymorphic in the `Int a` trait. This
/// is represented by IntegerKind::Unknown at first until type inference
/// can give the Ast::LiteralKind::Integer a type variable to aid in
/// unifying its size. When this happens the IntegerKind::Unknown is
/// mutated into an IntegerKind::Inferred(id).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntegerKind {
    /// Unknown integer literals are mutated into Inferred integers
    /// after undergoing type checking and being assigned a type variable.
    Unknown,

    /// Inferred integers use a type variable with the `Int a` constraint
    /// to be generic over any of the below integer types
    Inferred(TypeVariableId),

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
    IntegerLiteral(u64, IntegerKind),
    FloatLiteral(f64),
    CharLiteral(char),
    BooleanLiteral(bool),
    UnitLiteral,

    // Types
    TypeName(String),
    IntegerType(IntegerKind),
    PolymorphicIntType,
    FloatType,
    CharType,
    StringType,
    PointerType,
    BooleanType,
    UnitType,
    Ref,
    Mut,

    // Keywords
    And,
    As,
    Block,
    Break,
    Continue,
    Do,
    Else,
    Extern,
    For,
    Fn,
    Given,
    If,
    Impl,
    Import,
    In,
    Is,
    Isnt,
    Match,
    Module,
    Not,
    Or,
    Return,
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
    Pipe,               // |
    Colon,              // :
    Semicolon,          // ;
    Comma,              // ,
    MemberAccess,       // .
    MemberReference,    // .&
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
    Divide,             // /
    Backslash,          // \
    Ampersand,          // &
    At,                 // @
}

impl Token {
    pub fn is_overloadable_operator(&self) -> bool {
        use Token::*;
        matches!(
            self,
            And | As
                | At
                | In
                | Is
                | Isnt
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
                | Ampersand
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
            InvalidIntegerSuffx => write!(f, "Invalid suffix after integer literal (expected an integer type like i32 or a non-alphanumeric character)"),
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
            I8 => write!(f, "i8"),
            I16 => write!(f, "i16"),
            I32 => write!(f, "i32"),
            I64 => write!(f, "i64"),
            Isz => write!(f, "isz"),
            U8 => write!(f, "u8"),
            U16 => write!(f, "u16"),
            U32 => write!(f, "u32"),
            U64 => write!(f, "u64"),
            Usz => write!(f, "usz"),
            Unknown => write!(f, "Int"),
            Inferred(_) => write!(f, "Int"),
        }
    }
}

impl Display for Token {
    /// This formatting is shown when the parser outputs its
    /// "expected one of ..." tokens list after finding a syntax error.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;
        match self {
            EndOfInput => write!(f, "end of input"),
            Invalid(error) => write!(f, "{:?}", error),
            Newline => write!(f, "a newline"),
            Indent => write!(f, "an indent"),
            Unindent => write!(f, "an unindent"),

            Identifier(_) => write!(f, "an identifier"),
            StringLiteral(_) => write!(f, "a string literal"),
            IntegerLiteral(_, _) => write!(f, "an integer literal"),
            FloatLiteral(_) => write!(f, "a float literal"),
            CharLiteral(_) => write!(f, "a char literal"),
            BooleanLiteral(_) => write!(f, "a boolean literal"),
            UnitLiteral => write!(f, "'()'"),

            // Types
            TypeName(_) => write!(f, "a typename"),
            IntegerType(kind) => write!(f, "'{}'", kind),
            PolymorphicIntType => write!(f, "'Int'"),
            FloatType => write!(f, "'float'"),
            CharType => write!(f, "'char'"),
            StringType => write!(f, "'string'"),
            PointerType => write!(f, "'Ptr'"),
            BooleanType => write!(f, "'bool'"),
            UnitType => write!(f, "'unit'"),
            Ref => write!(f, "'ref'"),
            Mut => write!(f, "'mut'"),

            // Keywords
            And => write!(f, "'and'"),
            As => write!(f, "'as'"),
            Block => write!(f, "'block'"),
            Break => write!(f, "'break'"),
            Continue => write!(f, "'continue'"),
            Do => write!(f, "'do'"),
            Else => write!(f, "'else'"),
            Extern => write!(f, "'extern'"),
            For => write!(f, "'for'"),
            Fn => write!(f, "'fn'"),
            Given => write!(f, "'given'"),
            If => write!(f, "'if'"),
            Impl => write!(f, "'impl'"),
            Import => write!(f, "'import'"),
            In => write!(f, "'in'"),
            Is => write!(f, "'is'"),
            Isnt => write!(f, "'isnt'"),
            Match => write!(f, "'match'"),
            Module => write!(f, "'module'"),
            Not => write!(f, "'not'"),
            Or => write!(f, "'or'"),
            Return => write!(f, "'return'"),
            Then => write!(f, "'then'"),
            Trait => write!(f, "'trait'"),
            Type => write!(f, "'type'"),
            While => write!(f, "'while'"),
            With => write!(f, "'with'"),

            // Operators
            Equal => write!(f, "'='"),
            Assignment => write!(f, "':='"),
            EqualEqual => write!(f, "'=='"),
            NotEqual => write!(f, "'!='"),
            Range => write!(f, "'..'"),
            RightArrow => write!(f, "'->'"),
            ApplyLeft => write!(f, "'<|'"),
            ApplyRight => write!(f, "'|>'"),
            Append => write!(f, "'++'"),
            Index => write!(f, "'#'"),
            Modulus => write!(f, "'%'"),
            Multiply => write!(f, "'*'"),
            ParenthesisLeft => write!(f, "'('"),
            ParenthesisRight => write!(f, "')'"),
            Subtract => write!(f, "'-'"),
            Add => write!(f, "'+'"),
            BracketLeft => write!(f, "'['"),
            BracketRight => write!(f, "']'"),
            Pipe => write!(f, "'|'"),
            Colon => write!(f, "':'"),
            Semicolon => write!(f, "';'"),
            Comma => write!(f, "','"),
            MemberAccess => write!(f, "'.'"),
            MemberReference => write!(f, "'.&'"),
            LessThan => write!(f, "'<'"),
            GreaterThan => write!(f, "'>'"),
            LessThanOrEqual => write!(f, "'<='"),
            GreaterThanOrEqual => write!(f, "'>='"),
            Divide => write!(f, "'/'"),
            Backslash => write!(f, "'\\'"),
            Ampersand => write!(f, "'&'"),
            At => write!(f, "'@'"),
        }
    }
}
