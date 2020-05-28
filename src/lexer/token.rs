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
    // Lexer sends an end of input token before stopping so we get a proper error location when
    // reporting parsing errors that expect a token but found the end of a file instead.
    EndOfInput, 
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
    StringType,
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
    Backslash,          // \
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
            IndentChangeTooSmall => write!(f, "This indent/unindent is too small, it should be at least 2 spaces apart from the previous indentation level"),
            UnindentToNewLevel => write!(f, "This unindent doesn't return to any previous indentation level"),
            Expected(c) => write!(f, "Expected {} (U+{:x}) while lexing", *c, *c as u32),
            UnknownChar(c) => write!(f, "Unknown character '{}' (U+{:x}) in file", *c, *c as u32),
        }
    }
}

impl<'a> Display for Token<'a> {
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
            IntegerLiteral(_) => write!(f, "an integer literal"),
            FloatLiteral(_) => write!(f, "a float literal"),
            CharLiteral(_) => write!(f, "a char literal"),
            BooleanLiteral(_) => write!(f, "a boolean literal"),
            UnitLiteral => write!(f, "'()'"),

            // Types
            TypeName(_) => write!(f, "a typename"),
            IntegerType => write!(f, "'int'"),
            FloatType => write!(f, "'float'"),
            CharType => write!(f, "'char'"),
            StringType => write!(f, "'string'"),
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
            For => write!(f, "'for'"),
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
            Range => write!(f, "'...'"),
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
            LessThan => write!(f, "'<'"),
            GreaterThan => write!(f, "'>'"),
            LessThanOrEqual => write!(f, "'<='"),
            GreaterThanOrEqual => write!(f, "'>='"),
            Divide => write!(f, "'/'"),
            Backslash => write!(f, "'\\'"),
        }
    }
}
