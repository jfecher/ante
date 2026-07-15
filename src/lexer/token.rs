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

use colored::Colorize;
use serde::{Deserialize, Serialize};

/// This constant is meant to match the "name" of the index operator when used as an identifier in
/// source code (the `Extract` ability's method). The surface index syntax is `a.[i]`, but the
/// operator referenced as a value / defined in an ability is written `(.[])`.
pub(crate) const INDEX_OPERATOR_FUNCTION_NAME: &str = ".[]";

/// This constant is meant to match the "name" of the index-assignment operator when used as an
/// identifier in source code (the `Insert` ability's method).
pub(crate) const INDEX_ASSIGN_OPERATOR_FUNCTION_NAME: &str = ".[]:=";

/// Lexing can fail with these errors, though the Lexer just
/// returns the LexerError inside of an Invalid token which
/// the parser will fail on. Currently the parser fails immediately
/// when it finds these tokens but in the future it may be able
/// to issue the error then continue on to output as many errors
/// as possible.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, Hash)]
pub enum LexerError {
    UnicodeWhitespaceCharacterInSignificantWhitespace(char),
    InvalidEscapeSequence(char),
    InvalidIntegerSuffx,
    InvalidFloatSuffx,
    IndentChangeTooSmall, // We require >= 2 spaces to indent or any number of tabs.
    UnindentToNewLevel,   // Unindented to a new indent level rather than returning to a previous one
    InconsistentIndentation { found: char, expected: char },
    Expected(char),
    UnknownChar(char),
    MismatchedBracketInQuote { expected: ClosingBracket },
    QuoteWithEndBracketAndNoStart { unexpected: ClosingBracket },
    FailedToParseNumber { integer_string: String },
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

impl IntegerKind {
    pub fn is_signed(&self) -> bool {
        use IntegerKind::*;
        matches!(self, I8 | I16 | I32 | I64 | Isz)
    }

    pub fn size_in_bytes(self, ptr_size: u32) -> u32 {
        use IntegerKind::*;
        match self {
            I8 | U8 => 1,
            I16 | U16 => 2,
            I32 | U32 => 4,
            I64 | U64 => 8,
            Isz | Usz => ptr_size,
        }
    }

    /// Maximum magnitude representable in this kind with the given sign.
    /// For signed kinds the negative side allows one more than the positive side
    /// (e.g. I8's max positive is 127, while its max negative is 128). For unsigned kinds
    /// a negative input is never representable, so `None` is returned.
    pub fn max_magnitude(self, negative: bool, ptr_size: u32) -> Option<u64> {
        use IntegerKind::*;
        let bits: u32 = match self {
            I8 | U8 => 8,
            I16 | U16 => 16,
            I32 | U32 => 32,
            I64 | U64 => 64,
            Isz | Usz => 8 * ptr_size,
        };
        match (self.is_signed(), negative) {
            (true, false) => Some((1u64 << (bits - 1)) - 1),
            (true, true) => Some(1u64 << (bits - 1)),
            (false, false) => Some(if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 }),
            (false, true) => None,
        }
    }
}

/// A parsed integer literal value. Keeps magnitude and sign separate so the lexer
/// never has to sign-extend a u64. Range checking and the "is too large" diagnostic
/// both operate on this struct directly so negative literals display as the user wrote them.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Integer {
    pub magnitude: u64,
    /// Invariant: when `negative` is true, `magnitude > 0`. Zero is always non-negative.
    pub negative: bool,
}

impl Integer {
    pub fn positive(magnitude: u64) -> Self {
        Self { magnitude, negative: false }
    }

    pub fn negated(self) -> Self {
        if self.magnitude == 0 { self } else { Self { magnitude: self.magnitude, negative: !self.negative } }
    }

    /// Sign-extended bit representation used by pattern matching and any other consumer
    /// that wants the in-memory bits rather than the abstract value.
    pub fn to_bits(self) -> u64 {
        if self.negative { (self.magnitude as i64).wrapping_neg() as u64 } else { self.magnitude }
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative { write!(f, "-{}", self.magnitude) } else { write!(f, "{}", self.magnitude) }
    }
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

impl FloatKind {
    pub fn size_in_bytes(self) -> u32 {
        match self {
            FloatKind::F32 => 4,
            FloatKind::F64 => 8,
        }
    }
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
    Invalid(char),
    Newline,
    Indent,
    Unindent,

    DocComment(String),

    Identifier(String),
    StringLiteral(String),
    IntegerLiteral(Integer, Option<IntegerKind>),
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
    Break,
    Can,
    Continue,
    Do,
    Effect,
    Else,
    Exists,
    Export,
    Extern,
    Fn,
    For,
    Forall,
    Freeze,
    Given,
    Handler,
    If,
    Imm,
    Impl,
    Implicit,
    Import,
    In,
    Is,
    Loop,
    Match,
    Module,
    Move,
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
    Uniq,
    Var,
    While,
    With,

    // Operators
    Equal,              // =
    Assignment,         // :=
    AddAssign,          // +=
    SubAssign,          // -=
    MulAssign,          // *=
    DivAssign,          // /=
    ModAssign,          // %=
    EqualEqual,         // ==
    NotEqual,           // !=
    Range,              // ...
    RightArrow,         // ->
    FatArrow,           // =>
    TildeArrow,         // ~>
    LeftArrow,          // <-
    ApplyLeft,          // <|
    ApplyRight,         // |>
    Append,             // ++
    Modulus,            // %
    Divides,            // %%
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
    Index,              // .[   (postfix index syntax `a.[i]`)
    IndexBrackets,      // .[]  (the `Extract` operator name `(.[])`)
    IndexAssign,        // .[]:=
    Copy,               // .*
    Octothorpe,         // #
    Apostrophe,         // '
}

impl Token {
    pub fn is_overloadable_operator(&self) -> bool {
        use Token::*;
        matches!(
            self,
            In | Not
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
                | Divides
                | Range
                | IndexBrackets
                | IndexAssign
                | Copy
        )
    }
}

impl Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LexerError::*;
        match self {
            UnicodeWhitespaceCharacterInSignificantWhitespace(c) => {
                write!(
                    f,
                    "Only spaces or tabs are allowed in significant whitespace, U+{:x} is not allowed here",
                    *c as u32
                )
            },
            InvalidEscapeSequence(c) => write!(f, "Invalid character in escape sequence: '{}' (U+{:x})", c, *c as u32),
            InvalidIntegerSuffx => write!(
                f,
                "Invalid suffix after integer literal. Expected an integer type like i32 or a space to separate the two tokens"
            ),
            InvalidFloatSuffx => write!(
                f,
                "Invalid suffix after float literal. Expected either 'f', 'f32', 'f64', or a space to separate the two tokens"
            ),
            IndentChangeTooSmall => write!(
                f,
                "This indent/unindent is too small, it should be at least 2 spaces apart from the previous indentation level"
            ),
            UnindentToNewLevel => write!(f, "This unindent doesn't return to any previous indentation level"),
            InconsistentIndentation { found, expected } => {
                let name = |c: char| if c == '\t' { "tab" } else { "space" };
                write!(f, "{} found in a file using {}s in its significant indentation", name(*found), name(*expected))
            },
            Expected(c) => write!(f, "Expected {} (U+{:x}) while lexing", *c, *c as u32),
            UnknownChar(c) => write!(f, "Unknown character '{}' (U+{:x}) in file", *c, *c as u32),
            MismatchedBracketInQuote { expected } => {
                write!(f, "Mismatched bracket in quoted expression, expected `{expected}`")
            },
            QuoteWithEndBracketAndNoStart { unexpected } => {
                write!(f, "Cannot quote a lone {unexpected}, all brackets and indentation must be matched")
            },
            FailedToParseNumber { integer_string } => {
                write!(f, "Integer is too large: {}", integer_string.purple())
            },
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
            Token::Invalid(char) => write!(f, "`{}`", char),
            Token::Newline => write!(f, "a newline"),
            Token::Indent => write!(f, "an indent"),
            Token::Unindent => write!(f, "an unindent"),

            Token::DocComment(s) => write!(f, "///{s}"),

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
            Token::Break => write!(f, "break"),
            Token::Can => write!(f, "can"),
            Token::Continue => write!(f, "continue"),
            Token::Do => write!(f, "do"),
            Token::Effect => write!(f, "effect"),
            Token::Else => write!(f, "else"),
            Token::Exists => write!(f, "exists"),
            Token::Export => write!(f, "export"),
            Token::Extern => write!(f, "extern"),
            Token::Fn => write!(f, "fn"),
            Token::For => write!(f, "for"),
            Token::Forall => write!(f, "forall"),
            Token::Freeze => write!(f, "freeze"),
            Token::Given => write!(f, "given"),
            Token::Handler => write!(f, "handler"),
            Token::If => write!(f, "if"),
            Token::Imm => write!(f, "imm"),
            Token::Impl => write!(f, "impl"),
            Token::Implicit => write!(f, "implicit"),
            Token::Import => write!(f, "import"),
            Token::In => write!(f, "in"),
            Token::Is => write!(f, "is"),
            Token::Loop => write!(f, "loop"),
            Token::Match => write!(f, "match"),
            Token::Module => write!(f, "module"),
            Token::Move => write!(f, "move"),
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
            Token::Uniq => write!(f, "uniq"),
            Token::Var => write!(f, "var"),
            Token::While => write!(f, "while"),
            Token::With => write!(f, "with"),

            // Operators
            Token::Equal => write!(f, "="),
            Token::Assignment => write!(f, ":="),
            Token::AddAssign => write!(f, "+="),
            Token::SubAssign => write!(f, "-="),
            Token::MulAssign => write!(f, "*="),
            Token::DivAssign => write!(f, "/="),
            Token::ModAssign => write!(f, "%="),
            Token::EqualEqual => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::Range => write!(f, ".."),
            Token::RightArrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::TildeArrow => write!(f, "~>"),
            Token::LeftArrow => write!(f, "<-"),
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
            Token::LessThan => write!(f, "<"),
            Token::GreaterThan => write!(f, ">"),
            Token::LessThanOrEqual => write!(f, "<="),
            Token::GreaterThanOrEqual => write!(f, ">="),
            Token::Divide => write!(f, "/"),
            Token::Divides => write!(f, "%%"),
            Token::Backslash => write!(f, "\\"),
            Token::Ampersand => write!(f, "&"),
            Token::At => write!(f, "@"),
            Token::ExclamationMark => write!(f, "!"),
            Token::QuestionMark => write!(f, "?"),
            Token::Index => write!(f, ".["),
            Token::IndexBrackets => write!(f, "{INDEX_OPERATOR_FUNCTION_NAME}"),
            Token::IndexAssign => write!(f, "{INDEX_ASSIGN_OPERATOR_FUNCTION_NAME}"),
            Token::Copy => write!(f, ".*"),
            Token::Octothorpe => write!(f, "#"),
            Token::Apostrophe => write!(f, "'"),
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
        if self.0.fract() == 0.0 { write!(f, "{}.0", self.0) } else { write!(f, "{}", self.0) }
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
        "break" => Some(Token::Break),
        "can" => Some(Token::Can),
        "continue" => Some(Token::Continue),
        "do" => Some(Token::Do),
        "effect" => Some(Token::Effect),
        "else" => Some(Token::Else),
        "exists" => Some(Token::Exists),
        "export" => Some(Token::Export),
        "extern" => Some(Token::Extern),
        "fn" => Some(Token::Fn),
        "for" => Some(Token::For),
        "forall" => Some(Token::Forall),
        "freeze" => Some(Token::Freeze),
        "given" => Some(Token::Given),
        "handler" => Some(Token::Handler),
        "if" => Some(Token::If),
        "imm" => Some(Token::Imm),
        "impl" => Some(Token::Impl),
        "implicit" => Some(Token::Implicit),
        "import" => Some(Token::Import),
        "in" => Some(Token::In),
        "is" => Some(Token::Is),
        "loop" => Some(Token::Loop),
        "match" => Some(Token::Match),
        "module" => Some(Token::Module),
        "move" => Some(Token::Move),
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
        "uniq" => Some(Token::Uniq),
        "var" => Some(Token::Var),
        "while" => Some(Token::While),
        "with" => Some(Token::With),
        _ => None,
    }
}
