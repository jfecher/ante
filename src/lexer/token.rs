use std::fmt::{ self, Display };

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

impl<'a> Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Token::*;
        match self {
            Invalid(error) => write!(f, "{:?}", error),
            Newline => write!(f, "Newline"),
            Indent => write!(f, "Indent"),
            Unindent => write!(f, "Unindent"),

            Identifier(name) => write!(f, "{}", name),
            StringLiteral(literal) => write!(f, "\"{}\"", literal),
            IntegerLiteral(i) => write!(f, "{}", i),
            FloatLiteral(x) => write!(f, "{}", x),
            CharLiteral(c) => write!(f, "'{}'", c),
            BooleanLiteral(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            UnitLiteral => write!(f, "()"),

            // Types
            TypeName(name) => write!(f, "{}", name),
            IntegerType => write!(f, "int"),
            FloatType => write!(f, "float"),
            CharType => write!(f, "char"),
            BooleanType => write!(f, "bool"),
            UnitType => write!(f, "unit"),
            Ref => write!(f, "ref"),
            Mut => write!(f, "mut"),

            // Keywords
            And => write!(f, "and"),
            As => write!(f, "as"),
            Block => write!(f, "block"),
            Break => write!(f, "break"),
            Continue => write!(f, "continue"),
            Do => write!(f, "do"),
            Else => write!(f, "else"),
            For => write!(f, "for"),
            Given => write!(f, "given"),
            If => write!(f, "if"),
            Impl => write!(f, "impl"),
            Import => write!(f, "import"),
            In => write!(f, "in"),
            Is => write!(f, "is"),
            Isnt => write!(f, "isnt"),
            Match => write!(f, "match"),
            Module => write!(f, "module"),
            Not => write!(f, "not"),
            Or => write!(f, "or"),
            Return => write!(f, "return"),
            Then => write!(f, "then"),
            Trait => write!(f, "trait"),
            Type => write!(f, "type"),
            While => write!(f, "while"),
            With => write!(f, "with"),
            
            // Operators
            Equal => write!(f, "="),
            Assignment => write!(f, ":="),
            EqualEqual => write!(f, "=="),
            NotEqual => write!(f, "!="),
            Range => write!(f, "..."),
            RightArrow => write!(f, "->"),
            ApplyLeft => write!(f, "<|"),
            ApplyRight => write!(f, "|>"),
            Append => write!(f, "++"),
            Index => write!(f, "#"),
            Modulus => write!(f, "%"),
            Multiply => write!(f, "*"),
            ParenthesisLeft => write!(f, "("),
            ParenthesisRight => write!(f, ")"),
            Subtract => write!(f, "-"),
            Add => write!(f, "+"),
            BracketLeft => write!(f, "["),
            BracketRight => write!(f, "]"),
            Pipe => write!(f, "|"),
            Colon => write!(f, ":"),
            Semicolon => write!(f, ";"),
            Comma => write!(f, ","),
            MemberAccess => write!(f, "."),
            LessThan => write!(f, "<"),
            GreaterThan => write!(f, ">"),
            LessThanOrEqual => write!(f, "<="),
            GreaterThanOrEqual => write!(f, ">="),
            Divide => write!(f, "/"),
        }
    }
}
