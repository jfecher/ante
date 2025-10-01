//! token.rs - Defines the Token type which represents
//! a single grammatical unit of ante source code. This
//! can be an identifier, type name, string literal, integer
//! literal, operator, etc.
//!
//! Converting a stream of characters into a Vec<Token> is the goal of
//! the lexing phase of the compiler. The resulting tokens are then
//! fed into the parser to verify the program's grammar and create
//! an abstract syntax tree.
use std::{
    fmt::{self, Display},
    str::FromStr,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

/// Lexing can fail with these errors, though the Lexer just
/// returns the LexerError inside of an Invalid token which
/// the parser will fail on. Currently the parser fails immediately
/// when it finds these tokens but in the future it may be able
/// to issue the error then continue on to output as many errors
/// as possible.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Hash)]
pub enum LexerError {
    InvalidCharacterInSignificantWhitespace(char), // Only spaces are allowed in significant whitespace
    InvalidEscapeSequence(char),
    InvalidIntegerSuffx,
    InvalidFloatSuffx,
    IndentChangeTooSmall, // All indentation changes must be >= 2 spaces in size difference relative to the previous level
    UnindentToNewLevel,   // Unindented to a new indent level rather than returning to a previous one
    Expected(char),
    UnknownChar(char),
    MismatchedBracketInQuote { expected: ClosingBracket },
    QuoteWithEndBracketAndNoStart { unexpected: ClosingBracket },
}

/// Each Token::IntegerLiteral and Ast::LiteralKind::Integer has
/// an IntegerKind representing the size of the integer.
///
/// Integer literals in ante are polymorphic in the `Int a` type. The 'a'
/// here is a type variable which will later resolve to an IntegerKind once
/// the integer size and sign are known.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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

#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Hash)]
pub enum ClosingBracket {
    /// `)`
    Paren,
    /// `]`
    Bracket,
    /// `}`
    Brace,
    /// `    `
    Unindent,
}

impl ClosingBracket {
    /// Return the corresponding token for this closing bracket
    pub fn token(self) -> Token {
        match self {
            ClosingBracket::Paren => Token::ParenthesisRight,
            ClosingBracket::Bracket => Token::BracketRight,
            ClosingBracket::Brace => Token::BraceRight,
            ClosingBracket::Unindent => Token::Unindent,
        }
    }

    pub fn from_token(token: &Token) -> Option<Self> {
        use Token::*;
        match token {
            Indent | Unindent => Some(ClosingBracket::Unindent),
            ParenthesisLeft | ParenthesisRight => Some(ClosingBracket::Paren),
            BracketLeft | BracketRight => Some(ClosingBracket::Bracket),
            BraceLeft | BraceRight => Some(ClosingBracket::Brace),
            _ => None,
        }
    }
}

impl Display for ClosingBracket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClosingBracket::Paren => write!(f, "`)`"),
            ClosingBracket::Bracket => write!(f, "`]`"),
            ClosingBracket::Brace => write!(f, "`}}`"),
            ClosingBracket::Unindent => write!(f, "an unindent"),
        }
    }
}

/// Each float literal is polymorphic over the `Float a` type. The `a` is the
/// specific FloatKind of the float which is later resolved to one of these
/// variants (or kept generic if the code allows).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FloatKind {
    F32,
    F64,
}

/// Wrapper for `f64` providing `Eq` - we don't care about NaN values
#[derive(Debug, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct F64(pub f64);
impl Eq for F64 {}
impl std::hash::Hash for F64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state)
    }
}

impl FromStr for F64 {
    type Err = <f64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        f64::from_str(s).map(F64)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Hash)]
pub enum Token {
    // Lexer sends an end of input token before stopping so we get a proper error location when
    // reporting parsing errors that expect a token but found the end of a file instead.
    EndOfInput,
    Error(LexerError),
    Newline,
    Indent,
    Unindent,

    LineComment(String),

    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(u64, Option<IntegerKind>),
    FloatLiteral(F64, Option<FloatKind>),
    CharLiteral(char),
    BooleanLiteral(bool),
    UnitLiteral,

    /// A quoted list of tokens
    Quoted(Arc<Vec<Token>>),

    // Types
    TypeName(String),
    IntegerType(IntegerKind),
    FloatType(FloatKind),
    Mut,

    // Keywords
    And,
    As,
    Block,
    Can,
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
    Is,
    Loop,
    Match,
    Module,
    Not,
    Or,
    Owned,
    Pure,
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
    Modulus,            // %
    Multiply,           // *
    ParenthesisLeft,    // (
    ParenthesisRight,   // )
    Subtract,           // -
    Add,                // +
    BracketLeft,        // [
    BracketRight,       // ]
    BraceLeft,          // {
    BraceRight,         // }
    Interpolate,        // ${
    Pipe,               // |
    Colon,              // :
    Semicolon,          // ;
    Comma,              // ,
    MemberAccess,       // .
    MemberRef,          // .&
    MemberMut,          // .!
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
    Index,              // .[]
    IndexRef,           // .&[]
    IndexMut,           // .![]
    Octothorpe,         // #
}

impl Token {
    #[allow(unused)]
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
                | Index
                | IndexRef
                | IndexMut
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
            MismatchedBracketInQuote { expected } => write!(f, "Mismatched bracket in quoted expression, expected `{expected}`"),
            QuoteWithEndBracketAndNoStart { unexpected } => write!(f, "Cannot quote a lone {unexpected}, all brackets and indentation must be matched"),
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
            Token::Error(error) => write!(f, "{:?}", error),
            Token::Newline => write!(f, "a newline"),
            Token::Indent => write!(f, "an indent"),
            Token::Unindent => write!(f, "an unindent"),

            Token::LineComment(s) => write!(f, "//{s}"),

            Token::Identifier(s) => write!(f, "{s}"),
            Token::StringLiteral(s) => write!(f, "\"{s}\""),
            Token::IntegerLiteral(x, None) => write!(f, "{x}"),
            Token::IntegerLiteral(x, Some(kind)) => write!(f, "{x}_{kind}"),
            Token::FloatLiteral(x, None) => write!(f, "{x}"),
            Token::FloatLiteral(x, Some(kind)) => write!(f, "{x}_{kind}"),
            Token::CharLiteral(c) => write!(f, "c\"{c}\""),
            Token::BooleanLiteral(b) => write!(f, "{b}"),
            Token::UnitLiteral => write!(f, "()"),

            // Types
            Token::TypeName(n) => write!(f, "{n}"),
            Token::IntegerType(kind) => write!(f, "{}", kind),
            Token::FloatType(kind) => write!(f, "{}", kind),
            Token::Mut => write!(f, "mut"),

            // Keywords
            Token::And => write!(f, "and"),
            Token::As => write!(f, "as"),
            Token::Block => write!(f, "block"),
            Token::Can => write!(f, "can"),
            Token::Do => write!(f, "do"),
            Token::Effect => write!(f, "effect"),
            Token::Else => write!(f, "else"),
            Token::Extern => write!(f, "extern"),
            Token::Fn => write!(f, "fn"),
            Token::Given => write!(f, "given"),
            Token::Handle => write!(f, "handle"),
            Token::If => write!(f, "if"),
            Token::Impl => write!(f, "impl"),
            Token::Import => write!(f, "import"),
            Token::In => write!(f, "in"),
            Token::Is => write!(f, "is"),
            Token::Loop => write!(f, "loop"),
            Token::Match => write!(f, "match"),
            Token::Module => write!(f, "module"),
            Token::Not => write!(f, "not"),
            Token::Or => write!(f, "or"),
            Token::Owned => write!(f, "owned"),
            Token::Pure => write!(f, "pure"),
            Token::Return => write!(f, "return"),
            Token::Ref => write!(f, "ref"),
            Token::Shared => write!(f, "shared"),
            Token::Then => write!(f, "then"),
            Token::Trait => write!(f, "trait"),
            Token::Type => write!(f, "type"),
            Token::While => write!(f, "while"),
            Token::With => write!(f, "with"),

            // Operators
            Token::Equal => write!(f, "="),
            Token::Assignment => write!(f, ":="),
            Token::EqualEqual => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Range => write!(f, ".."),
            Token::RightArrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::ApplyLeft => write!(f, "<|"),
            Token::ApplyRight => write!(f, "|>"),
            Token::Append => write!(f, "++"),
            Token::Modulus => write!(f, "%"),
            Token::Multiply => write!(f, "*"),
            Token::ParenthesisLeft => write!(f, "("),
            Token::ParenthesisRight => write!(f, ")"),
            Token::Subtract => write!(f, "-"),
            Token::Add => write!(f, "+"),
            Token::BracketLeft => write!(f, "["),
            Token::BracketRight => write!(f, "]"),
            Token::BraceLeft => write!(f, "{{"),
            Token::BraceRight => write!(f, "}}"),
            Token::Interpolate => write!(f, "${{"),
            Token::Pipe => write!(f, "|"),
            Token::Colon => write!(f, ":"),
            Token::Semicolon => write!(f, ";"),
            Token::Comma => write!(f, ","),
            Token::MemberAccess => write!(f, "."),
            Token::MemberRef => write!(f, ".&"),
            Token::MemberMut => write!(f, ".!"),
            Token::LessThan => write!(f, "<"),
            Token::GreaterThan => write!(f, ">"),
            Token::LessThanOrEqual => write!(f, "<="),
            Token::GreaterThanOrEqual => write!(f, ">="),
            Token::Divide => write!(f, "/"),
            Token::Backslash => write!(f, "\\"),
            Token::Ampersand => write!(f, "&"),
            Token::At => write!(f, "@"),
            Token::ExclamationMark => write!(f, "!"),
            Token::QuestionMark => write!(f, "?"),
            Token::Index => write!(f, ".[]"),
            Token::IndexRef => write!(f, ".&[]"),
            Token::IndexMut => write!(f, ".![]"),
            Token::Octothorpe => write!(f, "#"),
            Token::Quoted(tokens) => {
                write!(f, "'")?;
                for token in tokens.iter() {
                    write!(f, "{token}")?;
                }
                Ok(())
            },
        }
    }
}

impl std::fmt::Display for F64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
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
        "mut" => Some(Token::Mut),
        "true" => Some(Token::BooleanLiteral(true)),
        "false" => Some(Token::BooleanLiteral(false)),
        "and" => Some(Token::And),
        "as" => Some(Token::As),
        "block" => Some(Token::Block),
        "can" => Some(Token::Can),
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
        "is" => Some(Token::Is),
        "loop" => Some(Token::Loop),
        "match" => Some(Token::Match),
        "module" => Some(Token::Module),
        "not" => Some(Token::Not),
        "or" => Some(Token::Or),
        "owned" => Some(Token::Owned),
        "pure" => Some(Token::Pure),
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
