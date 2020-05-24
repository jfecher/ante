#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LexerError {
    InvalidCharacterInSignificantWhitespace(char), // Only spaces are allowed in significant whitespace
    InvalidEscapeSequence(char),
    IndentChangeTooSmall, // All indentation changes must be >= 2 spaces in size difference relative to the previous level
    UnindentToNewLevel, // Unindented to a new indent level rather than returning to a previous one
    Expected(char),
    UnknownChar(char),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    Invalid(LexerError),
    Newline,
    Indent,
    Unindent,

    Identifier(&'a str),
    StringLiteral(String),
    IntegerLiteral(u64),
    FloatLiteral(f64),
    CharLiteral(char),
    BooleanLiteral(bool),
    UnitLiteral,

    // Types
    TypeName(&'a str),
    IntegerType,
    FloatType,
    CharType,
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
    For,
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
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
    Divide,             // /
}
