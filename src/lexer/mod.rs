//! lexer/mod.rs - Contains the Lexer struct and implements the
//! first phase of the compiler: lexing.
//!
//! Lexing is the simplest compilation phase. Its goal is to convert
//! a stream of characters into a Vec<Token> that can then be fed
//! into the parser. Ante's lexer is somewhat more complex than other
//! language's lexers since it must also handle whitespace sensitivity.
//!
//! Aside from whitespace sensitivity, ante's lexer is fairly standard.
//! It implements `Iterator<Item = (Token, Location)>` and on each step
//! continues in the input until it can return the next full word, number,
//! operator, etc. as a Token. When reading this file it is recomended
//! to start with the Iterator impl as all Lexer methods are called from it.
//!
//! For whitespace, the lexer operates on a stack of indentation levels.
//! For each indentation level, whitespace is either ignored or not ignored
//! depending on which token came before the indent that started the block.
//! These ignored indent levels are how ante handles "semicolon inference".
//! In short, if you indent after an expression you continue the expression and
//! any newlines on that indent level are ignored. If you indent after a token
//! that expects an indent after it though, the indent is still issued and the
//! indentation level is not ignored. See `Lexer::should_expect_indent_after_token`
//! for a list of tokens after which indentation is not ignored.
//!
//! If indentation follows such a token:
//!     - The Lexer pushes an indent level that is not ignored.
//!     - An Indent token is issued and the lexer skips any subsequent empty
//!       lines until the first non-whitespace token.
//!     - Tokens are issued as normal, with Newline tokens being issued for
//!       each newline (multiple consecutive newlines will only have 1 Newline token).
//!     - An Unindent token is issued when the indentation level changes back down
//!       and the current indentation level is popped off of the Lexer's `indent_levels`
//!
//! If an indent is not preceeded by such a token:
//!     - The lexer pushes an ignored indent level.
//!     - A newline is not issued, nor are any Newline tokens. This is so the parser
//!       sees these tokens on this indent level as being on the same line. This
//!       is how expressions can be continued in ante despite most ending on a Newline.
pub mod token;

use crate::diagnostics::{Position, Span};
use std::{str::Chars, sync::Arc};
use token::{lookup_keyword, ClosingBracket, FloatKind, IntegerKind, LexerError, Token, F64};

#[derive(Clone)]
struct OpenBraces {
    parenthesis: usize,
    curly: usize,
    square: usize,
}

#[derive(Clone)]
pub struct Lexer<'contents> {
    current: char,
    next: char,
    file_contents: &'contents str,
    token_start_position: Position,
    current_position: Position,
    indent_levels: Vec<IndentLevel>,
    current_indent_level: usize,
    return_newline: bool, // Hack to always return a newline after an Unindent token
    previous_token_expects_indent: bool,
    chars: Chars<'contents>,
    open_braces: OpenBraces,
    pending_interpolations: Vec<usize>,
}

/// The lexer maintains a stack of IndentLevels to remember
/// how far each previous level was indented. An indent level
/// may be ignored (no indent, newline, or unindent tokens issued)
/// if an indentation is encountered that was not prefixed by a
/// token that expects an indent afterward (like `then`, `do` or `=`).
#[derive(Copy, Clone)]
struct IndentLevel {
    column: usize,
    ignored: bool,
}

impl IndentLevel {
    fn new(column: usize) -> IndentLevel {
        IndentLevel { column, ignored: false }
    }

    fn ignored(column: usize) -> IndentLevel {
        IndentLevel { column, ignored: true }
    }
}

type IterElem<'a> = Option<(Token, Span)>;

impl<'contents> Lexer<'contents> {
    pub fn new(file_contents: &'contents str) -> Lexer<'contents> {
        let mut chars = file_contents.chars();
        let current = chars.next().unwrap_or('\0');
        let next = chars.next().unwrap_or('\0');
        Lexer {
            current,
            next,
            file_contents,
            current_position: Position::start(),
            token_start_position: Position::start(),
            indent_levels: vec![IndentLevel { column: 0, ignored: false }],
            current_indent_level: 0,
            return_newline: false,
            previous_token_expects_indent: false,
            chars,
            open_braces: OpenBraces { parenthesis: 0, curly: 0, square: 0 },
            pending_interpolations: Vec::new(),
        }
    }

    fn should_expect_indent_after_token(token: &Token) -> bool {
        matches!(
            token,
            Token::Block
                | Token::Do
                | Token::Else
                | Token::Extern
                | Token::Handle
                | Token::If
                | Token::Match
                | Token::Then
                | Token::While
                | Token::With
                | Token::Equal
                | Token::RightArrow
        )
    }

    fn at_end_of_input(&self) -> bool {
        self.current == '\0'
    }

    fn advance(&mut self) -> char {
        let ret = self.current;
        self.current = self.next;
        self.next = self.chars.next().unwrap_or('\0');
        self.current_position.byte_index += ret.len_utf8();
        self.current_position.column_number += 1;

        if ret == '\n' {
            self.current_position.column_number = 0;
            self.current_position.line_number += 1;
        }

        ret
    }

    fn locate(&self) -> Span {
        let mut end = self.current_position;
        // end is exclusive so we have to increment 1.
        // no token ends in a `\n` so we can ignore the line number
        end.column_number += 1;
        end.byte_index += 1;
        Span { start: self.token_start_position, end }
    }

    fn advance_with(&mut self, token: Token) -> IterElem {
        self.advance();
        Some((token, self.locate()))
    }

    fn advance2_with(&mut self, token: Token) -> IterElem {
        self.advance();
        self.advance_with(token)
    }

    fn get_slice_containing_current_token(&self) -> &'contents str {
        &self.file_contents[self.token_start_position.byte_index..self.current_position.byte_index]
    }

    fn expect(&mut self, expected: char, token: Token) -> IterElem {
        if self.current == expected {
            self.advance_with(token)
        } else {
            self.advance_with(Token::Error(LexerError::Expected(expected)))
        }
    }

    fn advance_while<F>(&mut self, mut f: F) -> &'contents str
    where
        F: FnMut(char, char) -> bool,
    {
        while f(self.current, self.next) && !self.at_end_of_input() {
            self.advance();
        }
        self.get_slice_containing_current_token()
    }

    fn lex_integer(&mut self) -> String {
        let start = self.current_position.byte_index;

        while !self.at_end_of_input() && (self.current.is_ascii_digit() || self.current == '_') {
            self.advance();
        }

        let end = self.current_position.byte_index;
        self.file_contents[start..end].replace('_', "")
    }

    fn lex_integer_suffix(&mut self) -> Result<Option<IntegerKind>, Token> {
        let start = self.current_position.byte_index;
        while self.current.is_alphanumeric() || self.current == '_' {
            self.advance();
        }

        let word = &self.file_contents[start..self.current_position.byte_index];
        Ok(Some(match word {
            "i8" => IntegerKind::I8,
            "u8" => IntegerKind::U8,
            "i16" => IntegerKind::I16,
            "u16" => IntegerKind::U16,
            "i32" => IntegerKind::I32,
            "u32" => IntegerKind::U32,
            "i64" => IntegerKind::I64,
            "u64" => IntegerKind::U64,
            "isz" => IntegerKind::Isz,
            "usz" => IntegerKind::Usz,
            "" => return Ok(None),
            _ => return Err(Token::Error(LexerError::InvalidIntegerSuffx)),
        }))
    }

    fn lex_float_suffix(&mut self) -> Result<Option<FloatKind>, Token> {
        let start = self.current_position.byte_index;
        while self.current.is_alphanumeric() || self.current == '_' {
            self.advance();
        }

        let word = &self.file_contents[start..self.current_position.byte_index];
        match word {
            "f32" => Ok(Some(FloatKind::F32)),
            "f64" => Ok(Some(FloatKind::F64)),
            "" => Ok(None),
            _ => Err(Token::Error(LexerError::InvalidFloatSuffx)),
        }
    }

    fn lex_number(&mut self) -> IterElem {
        let integer_string = self.lex_integer();

        if self.current == '.' && self.next.is_ascii_digit() {
            self.advance();
            let float_string = integer_string + "." + &self.lex_integer();
            let float = float_string.parse().unwrap();
            let location = self.locate();

            match self.lex_float_suffix() {
                Ok(suffix) => Some((Token::FloatLiteral(float, suffix), location)),
                Err(lexer_error) => Some((lexer_error, location)),
            }
        } else {
            let integer = integer_string.parse().unwrap();
            let location = self.locate();
            match self.lex_integer_suffix() {
                Ok(suffix) => Some((Token::IntegerLiteral(integer, suffix), location)),
                Err(lexer_error) => Some((lexer_error, location)),
            }
        }
    }

    fn lex_negative(&mut self) -> IterElem {
        self.advance(); // consume '-'

        if self.current.is_numeric() {
            self.lex_number().map(|(token, location)| {
                let token = match token {
                    Token::IntegerLiteral(x, kind) => {
                        let x = format!("-{}", x).parse::<i64>().unwrap();
                        Token::IntegerLiteral(x as u64, kind)
                    },
                    Token::FloatLiteral(x, kind) => Token::FloatLiteral(F64(-x.0), kind),
                    _ => unreachable!(),
                };
                (token, location)
            })
        } else {
            Some((Token::Subtract, self.locate()))
        }
    }

    fn lex_alphanumeric(&mut self) -> IterElem {
        let is_type = self.current.is_uppercase();
        let word = self.advance_while(|current, _| current.is_alphanumeric() || current == '_');
        let location = self.locate();

        match lookup_keyword(word) {
            Some(keyword) => {
                self.previous_token_expects_indent = Lexer::should_expect_indent_after_token(&keyword);
                Some((keyword, location))
            },
            None if is_type => Some((Token::TypeName(word.to_owned()), location)),
            None => Some((Token::Identifier(word.to_owned()), location)),
        }
    }

    fn lex_string(&mut self) -> IterElem {
        self.advance();
        let mut contents = String::new();
        while !(self.current == '"' || self.at_end_of_input()) {
            let current_char = match (self.current, self.next) {
                ('$', '{') => {
                    return Some((Token::StringLiteral(contents), self.locate()));
                },
                ('\\', c) => {
                    self.advance();
                    match c {
                        '\\' | '"' | '$' => c,
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '0' => '\0',
                        _ => {
                            let error = LexerError::InvalidEscapeSequence(self.current);
                            return self.advance2_with(Token::Error(error));
                        },
                    }
                },
                (c, _) => c,
            };
            contents.push(current_char);
            self.advance();
        }
        self.expect('"', Token::StringLiteral(contents))
    }

    fn lex_quoted(&mut self) -> IterElem {
        // skip the single quote
        let start = self.current_position;
        let mut span = Span { start, end: start };
        self.advance();

        let mut tokens = Vec::new();
        let mut bracket_stack = Vec::new();

        // Keep track of the stack of brackets so that we always match them.
        // This includes quoted blocks
        while {
            let (token, token_span) = self.next()?;
            span.end = token_span.end;

            match token {
                token @ (Token::Indent | Token::ParenthesisLeft | Token::BracketLeft | Token::BraceLeft) => {
                    let bracket = ClosingBracket::from_token(&token).unwrap();
                    bracket_stack.push(bracket);
                    tokens.push(token)
                },
                token @ (Token::Unindent | Token::ParenthesisRight | Token::BracketRight | Token::BraceRight) => {
                    match bracket_stack.pop() {
                        Some(matching) if matching.token() == token => tokens.push(token),
                        Some(expected) => {
                            let error = LexerError::MismatchedBracketInQuote { expected };
                            return Some((Token::Error(error), span));
                        },
                        None => {
                            let unexpected = ClosingBracket::from_token(&token).unwrap();
                            let error = LexerError::QuoteWithEndBracketAndNoStart { unexpected };
                            return Some((Token::Error(error), span));
                        },
                    }
                },
                other => {
                    tokens.push(other);
                },
            }

            !bracket_stack.is_empty()
        } {}

        Some((Token::Quoted(Arc::new(tokens)), span))
    }

    /// The char literal syntax is: c"_" where _ is an arbitrary character
    fn lex_char_literal(&mut self) -> IterElem {
        // Skip c"
        self.advance();
        self.advance();

        let contents = if self.current == '\\' {
            self.advance();
            match self.current {
                '\\' | '"' => self.current,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                _ => {
                    let error = LexerError::InvalidEscapeSequence(self.current);
                    return self.advance2_with(Token::Error(error));
                },
            }
        } else {
            self.current
        };

        self.advance();
        self.expect('"', Token::CharLiteral(contents))
    }

    fn lex_newline(&mut self) -> IterElem {
        self.advance();

        // Must advance start_position otherwise the slice returned by advance_while
        // in recursive calls to lex_newline will be longer than it should be
        self.token_start_position = self.current_position;
        let new_indent = self.advance_while(|current, _| current == ' ').len();

        match (self.current, self.next) {
            ('\r', _) => self.lex_newline(),
            ('\n', _) => self.lex_newline(),

            (c, _) if c.is_whitespace() => {
                let error = LexerError::InvalidCharacterInSignificantWhitespace(self.current);
                self.advance_with(Token::Error(error))
            },

            ('/', '*') => self.lex_multiline_comment(),

            _ if new_indent > self.current_indent_level => self.lex_indent(new_indent),
            _ if new_indent < self.current_indent_level => self.lex_unindent(new_indent),

            _ if self.newlines_ignored() => self.next(),
            _ => Some((Token::Newline, self.locate())),
        }
    }

    fn newlines_ignored(&self) -> bool {
        self.indent_levels.last().unwrap().ignored
    }

    fn lex_indent(&mut self, new_indent: usize) -> IterElem {
        if new_indent == self.current_indent_level + 1 {
            self.indent_levels.push(IndentLevel::new(new_indent));
            self.current_indent_level = new_indent;
            Some((Token::Error(LexerError::IndentChangeTooSmall), self.locate()))
        } else if self.previous_token_expects_indent {
            self.indent_levels.push(IndentLevel::new(new_indent));
            self.current_indent_level = new_indent;
            Some((Token::Indent, self.locate()))
        } else {
            self.indent_levels.push(IndentLevel::ignored(new_indent));
            self.current_indent_level = new_indent;
            self.next()
        }
    }

    fn lex_unindent(&mut self, new_indent: usize) -> IterElem {
        let last_indent = self.indent_levels.pop().unwrap();
        self.current_indent_level = new_indent;

        // The newline returned after an unindent 'belongs' to the
        // previous indent level which is why we need to check if the
        // now-current indent level has newlines ignored here instead
        // of checking the last_indent level that was just popped.
        self.return_newline = !self.newlines_ignored();

        if new_indent > last_indent.column {
            Some((Token::Error(LexerError::UnindentToNewLevel), self.locate()))
        } else if last_indent.ignored {
            self.next()
        } else {
            Some((Token::Unindent, self.locate()))
        }
    }

    fn lex_singleline_comment(&mut self) -> IterElem {
        // Skip the leading `//`
        self.advance();
        self.advance();

        let comment = self.advance_while(|current, _| current != '\n').to_string();
        Some((Token::LineComment(comment), self.locate()))
    }

    fn lex_multiline_comment(&mut self) -> IterElem {
        self.advance();
        self.advance();
        let mut comment_level = 1;
        self.advance_while(|current, next| {
            match (current, next) {
                ('/', '*') => comment_level += 1,
                ('*', '/') => comment_level -= 1,
                _ => (),
            }
            comment_level != 0
        });
        self.advance();
        self.advance();
        self.next()
    }
}

impl<'contents> Iterator for Lexer<'contents> {
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        let last_indent = *self.indent_levels.last().unwrap();
        self.token_start_position = self.current_position;

        if self.return_newline {
            self.return_newline = false;
            return Some((Token::Newline, self.locate()));
        }

        // May have to issue several consecutive unindent tokens, so check first
        if self.current_indent_level < last_indent.column {
            return self.lex_unindent(self.current_indent_level);
        }

        // Must check for whitespace changes before previous_token_expects_indent is reset.
        if self.current == '\r' || self.current == '\n' {
            return self.lex_newline();
        } else if self.current.is_whitespace() {
            self.advance();
            return self.next();
        }

        self.previous_token_expects_indent = false;

        // Checks if there is the same number of open parenthesis as when interpolation last began
        let matched_interpolation = match self.pending_interpolations.last() {
            Some(open_braces) => open_braces == &self.open_braces.curly,
            None => false,
        };

        match (self.current, self.next) {
            (c, _) if c.is_ascii_digit() => self.lex_number(),
            ('c', '"') => self.lex_char_literal(),
            (c, _) if c.is_alphanumeric() || c == '_' => self.lex_alphanumeric(),
            ('\0', _) => {
                if self.current_indent_level != 0 {
                    // Issue any pending unindent tokens before EndOfInput
                    self.lex_unindent(0)
                } else if self.current_position.byte_index > self.file_contents.len() {
                    None
                } else {
                    self.advance_with(Token::EndOfInput)
                }
            },
            ('"', _) => self.lex_string(),
            ('}', _) if matched_interpolation => {
                self.current = '"';
                self.pending_interpolations.pop();
                Some((Token::BraceRight, self.locate()))
            },
            ('$', '{') => {
                self.pending_interpolations.push(self.open_braces.curly);
                self.advance2_with(Token::Interpolate)
            },
            ('\'', _) => self.lex_quoted(),
            ('/', '/') => self.lex_singleline_comment(),
            ('/', '*') => self.lex_multiline_comment(),
            ('=', '=') => self.advance2_with(Token::EqualEqual),
            ('=', '>') => self.advance2_with(Token::FatArrow),
            ('.', '.') => self.advance2_with(Token::Range),
            (':', '=') => {
                self.previous_token_expects_indent = true;
                self.advance2_with(Token::Assignment)
            },
            ('=', _) => {
                self.previous_token_expects_indent = true;
                self.advance_with(Token::Equal)
            },
            ('-', '>') => {
                self.previous_token_expects_indent = true;
                self.advance2_with(Token::RightArrow)
            },
            ('.', '&') => {
                self.advance();
                self.advance();
                if self.current == '[' {
                    self.advance_with(Token::IndexRef)
                } else {
                    Some((Token::MemberRef, self.locate()))
                }
            },
            ('.', '!') => {
                self.advance();
                self.advance();
                if self.current == '[' {
                    self.advance_with(Token::IndexMut)
                } else {
                    Some((Token::MemberMut, self.locate()))
                }
            },
            ('.', '[') => self.advance2_with(Token::Index),
            ('.', _) => self.advance_with(Token::MemberAccess),
            ('-', _) => self.lex_negative(),
            ('!', '=') => self.advance2_with(Token::NotEqual),
            ('<', '|') => self.advance2_with(Token::ApplyLeft),
            ('|', '>') => self.advance2_with(Token::ApplyRight),
            ('+', '+') => self.advance2_with(Token::Append),
            ('(', ')') => self.advance2_with(Token::UnitLiteral),
            ('<', '=') => self.advance2_with(Token::LessThanOrEqual),
            ('>', '=') => self.advance2_with(Token::GreaterThanOrEqual),
            ('%', _) => self.advance_with(Token::Modulus),
            ('*', _) => self.advance_with(Token::Multiply),
            ('(', _) => {
                self.open_braces.parenthesis += 1;
                self.advance_with(Token::ParenthesisLeft)
            },
            (')', _) => {
                // This will overflow if there are mismatched parenthesis,
                // should we handle this inside the lexer,
                // or leave that to the parsing stage?
                self.open_braces.parenthesis = self.open_braces.parenthesis.saturating_sub(1);
                self.advance_with(Token::ParenthesisRight)
            },
            ('+', _) => self.advance_with(Token::Add),
            ('[', _) => {
                self.open_braces.square += 1;
                self.advance_with(Token::BracketLeft)
            },
            (']', _) => {
                self.open_braces.square = self.open_braces.square.saturating_sub(1);
                self.advance_with(Token::BracketRight)
            },
            ('{', _) => {
                self.open_braces.curly += 1;
                self.advance_with(Token::BraceLeft)
            },
            ('}', _) => {
                self.open_braces.curly = self.open_braces.curly.saturating_sub(1);
                self.advance_with(Token::BraceRight)
            },
            ('|', _) => self.advance_with(Token::Pipe),
            (':', _) => self.advance_with(Token::Colon),
            (';', _) => self.advance_with(Token::Semicolon),
            (',', _) => self.advance_with(Token::Comma),
            ('<', _) => self.advance_with(Token::LessThan),
            ('>', _) => self.advance_with(Token::GreaterThan),
            ('/', _) => self.advance_with(Token::Divide),
            ('\\', _) => self.advance_with(Token::Backslash),
            ('&', _) => self.advance_with(Token::Ampersand),
            ('@', _) => self.advance_with(Token::At),
            ('!', _) => self.advance_with(Token::ExclamationMark),
            ('?', _) => self.advance_with(Token::QuestionMark),
            ('#', _) => self.advance_with(Token::Octothorpe),
            (c, _) => self.advance_with(Token::Error(LexerError::UnknownChar(c))),
        }
    }
}
