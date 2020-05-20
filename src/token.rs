#[derive(Debug, PartialEq)]
pub enum IntegerSize {
    I8, I16, I32, I64, Isz,
}

#[derive(Debug, PartialEq)]
pub enum FloatSize {
    F16, F32, F64,
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Invalid,
    EndOfFile,
    Newline,
    Indent,
    Unindent,

    Identifier(&'a str),
    StringLiteral(&'a str),
    IntegerLiteral(u64),
    FloatLiteral(f64),
    CharLiteral(char),
    BooleanLiteral(bool),
    UnitLiteral,

    // Types
    TypeName(&'a str),
    TypeVariable(&'a str),
    SignedType(IntegerSize),
    UnsignedType(IntegerSize),
    FloatType(FloatSize),
    CharType,
    BooleanType,
    UnitType,
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
    Ref,
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
    Comma,              // ,
    MemberAccess,       // .
    LessThan,           // <
    GreaterThan,        // >
    LessThanOrEqual,    // <=
    GreaterThanOrEqual, // >=
    Divide,             // /
}
