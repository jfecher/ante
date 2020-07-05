pub mod token;

use std::str::Chars;
use std::path::Path;
use std::collections::HashMap;
use token::{ Token, LexerError };
use crate::error::location::{ Position, EndPosition, Location, Locatable };

#[derive(Clone)]
pub struct Lexer<'cache, 'contents> {
    current: char,
    next: char,
    filename: &'cache Path,
    file_contents: &'contents str,
    token_start_position: Position,
    current_position: Position,
    indent_levels: Vec<usize>,
    current_indent_level: usize,
    return_newline: bool, // Hack to always return a newline after an Unindent token
    chars: Chars<'contents>,
    keywords: HashMap<&'static str, Token>,
}

impl<'cache, 'contents> Locatable<'cache> for Lexer<'cache, 'contents> {
    fn locate(&self) -> Location<'cache> {
        let end = EndPosition::new(self.current_position.index);
        Location::new(self.filename, self.token_start_position, end)
    }
}

type IterElem<'a> = Option<(Token, Location<'a>)>;

impl<'cache, 'contents> Lexer<'cache, 'contents> {
    pub fn get_keywords() -> HashMap<&'static str, Token> {
        vec![
            ("int", Token::IntegerType),
            ("float", Token::FloatType),
            ("char", Token::CharType),
            ("string", Token::StringType),
            ("bool", Token::BooleanType),
            ("unit", Token::UnitType),
            ("ref", Token::Ref),
            ("mut", Token::Mut),

            ("true", Token::BooleanLiteral(true)),
            ("false", Token::BooleanLiteral(false)),

            ("and", Token::And),
            ("as", Token::As),
            ("block", Token::Block),
            ("break", Token::Break),
            ("continue", Token::Continue),
            ("do", Token::Do),
            ("else", Token::Else),
            ("for", Token::For),
            ("given", Token::Given),
            ("if", Token::If),
            ("impl", Token::Impl),
            ("import", Token::Import),
            ("in", Token::In),
            ("is", Token::Is),
            ("isnt", Token::Isnt),
            ("match", Token::Match),
            ("module", Token::Module),
            ("not", Token::Not),
            ("or", Token::Or),
            ("return", Token::Return),
            ("then", Token::Then),
            ("trait", Token::Trait),
            ("type", Token::Type),
            ("while", Token::While),
            ("with", Token::With),
        ].into_iter().collect()
    }

    pub fn new(filename: &'cache Path, file_contents: &'contents str) -> Lexer<'cache, 'contents> {
        let mut chars = file_contents.chars();
        let current = chars.next().unwrap_or('\0');
        let next = chars.next().unwrap_or('\0');
        Lexer {
            current,
            next,
            filename,
            file_contents,
            current_position: Position::begin(),
            token_start_position: Position::begin(),
            indent_levels: vec![0],
            current_indent_level: 0,
            return_newline: false,
            chars,
            keywords: Lexer::get_keywords(),
        }
    }

    fn at_end_of_input(&self) -> bool {
        self.current == '\0'
    }

    fn advance(&mut self) -> char {
        let ret = self.current;
        self.current = self.next;
        self.next = self.chars.next().unwrap_or('\0');
        self.current_position.advance(ret == '\n');
        ret
    }

    fn advance_with(&mut self, token: Token) -> IterElem<'cache> {
        self.advance();
        Some((token, self.locate()))
    }

    fn advance2_with(&mut self, token: Token) -> IterElem<'cache> {
        self.advance();
        self.advance_with(token)
    }

    fn get_slice_containing_current_token(&self) -> &'contents str {
        &self.file_contents[self.token_start_position.index .. self.current_position.index]
    }

    fn advance_while<F>(&mut self, mut f: F) -> &'contents str
        where F: FnMut(char, char) -> bool
    {
        while f(self.current, self.next) && !self.at_end_of_input() {
            self.advance();
        }
        self.get_slice_containing_current_token()
    }

    fn expect(&mut self, expected: char, token: Token) -> IterElem<'cache> {
        if self.current == expected {
            self.advance_with(token)
        } else {
            self.advance_with(Token::Invalid(LexerError::Expected(expected)))
        }
    }

    fn lex_number(&mut self) -> IterElem<'cache> {
        let integer_string = self.advance_while(|current, _| current.is_digit(10));

        if self.current == '.' && self.next.is_digit(10) {
            self.advance();
            self.advance_while(|current, _| current.is_digit(10));
            let float_string = self.get_slice_containing_current_token();
            let float = float_string.parse().unwrap();
            Some((Token::FloatLiteral(float), self.locate()))
        } else {
            let integer = integer_string.parse().unwrap();
            Some((Token::IntegerLiteral(integer), self.locate()))
        }
    }

    fn lex_alphanumeric(&mut self) -> IterElem<'cache> {
        let is_type = self.current.is_uppercase();
        let word = self.advance_while(|current, _| current.is_alphanumeric() || current == '_');

        if is_type {
            Some((Token::TypeName(word.to_owned()), self.locate()))
        } else {
            match self.keywords.get(word) {
                Some(keyword) => Some((keyword.clone(), self.locate())),
                None => Some((Token::Identifier(word.to_owned()), self.locate())),
            }
        }
    }

    fn lex_string(&mut self) -> IterElem<'cache> {
        self.advance();
        let mut contents = String::new();
        while self.current != '"' {
            let current_char = if self.current == '\\' {
                self.advance();
                match self.current {
                    '\\' | '\'' => self.current,
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '0' => '\0',
                    _ => {
                        let error = LexerError::InvalidEscapeSequence(self.current);
                        return self.advance2_with(Token::Invalid(error));
                    },
                }
            } else {
                self.current
            };
            contents.push(current_char);
            self.advance();
        }
        self.expect('"', Token::StringLiteral(contents))
    }

    fn lex_char_literal(&mut self) -> IterElem<'cache> {
        self.advance();
        let contents = if self.current == '\\' {
            self.advance();
            match self.current {
                '\\' | '\'' => self.current,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                _ => {
                    let error = LexerError::InvalidEscapeSequence(self.current);
                    return self.advance2_with(Token::Invalid(error));
                }
            }
        } else {
            self.current
        };

        self.advance();
        self.expect('\'', Token::CharLiteral(contents))
    }

    fn lex_newline(&mut self) -> IterElem<'cache> {
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
                self.advance_with(Token::Invalid(error))
            },
            ('/', '/') => self.lex_singleline_comment(),
            ('/', '*') => self.lex_multiline_comment(),
            _ if new_indent > self.current_indent_level => self.lex_indent(new_indent),
            _ if new_indent < self.current_indent_level => self.lex_unindent(new_indent),
            _ => Some((Token::Newline, self.locate())),
        }
    }

    fn lex_indent(&mut self, new_indent: usize) -> IterElem<'cache> {
        if new_indent == self.current_indent_level + 1 {
            self.indent_levels.push(new_indent);
            self.current_indent_level = new_indent;
            Some((Token::Invalid(LexerError::IndentChangeTooSmall), self.locate()))
        } else {
            debug_assert!(new_indent > self.current_indent_level);
            self.indent_levels.push(new_indent);
            self.current_indent_level = new_indent;
            Some((Token::Indent, self.locate()))
        }
    }

    fn lex_unindent(&mut self, new_indent: usize) -> IterElem<'cache> {
        if self.current_indent_level == new_indent + 1 {
            self.current_indent_level = new_indent;
            Some((Token::Invalid(LexerError::IndentChangeTooSmall), self.locate()))
        } else {
            debug_assert!(new_indent < self.current_indent_level);
            self.current_indent_level = new_indent;
            self.next()
        }
    }

    fn lex_singleline_comment(&mut self) -> IterElem<'cache> {
        self.advance_while(|current, _| current != '\n');
        self.next()
    }

    fn lex_multiline_comment(&mut self) -> IterElem<'cache> {
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
        self.next()
    }
}

impl<'cache, 'contents> Iterator for Lexer<'cache, 'contents> {
    type Item = (Token, Location<'cache>);

    fn next(&mut self) -> Option<Self::Item> {
        let last_indent = self.indent_levels[self.indent_levels.len() - 1];
        self.token_start_position = self.current_position;

        match (self.current, self.next) {
            _ if self.return_newline => {
                self.return_newline = false;
                Some((Token::Newline, self.locate()))
            },
            _ if self.current_indent_level < last_indent => {
                self.indent_levels.pop();
                let last_indent = self.indent_levels[self.indent_levels.len() - 1];
                if self.current_indent_level > last_indent {
                    self.current_indent_level = last_indent;
                    Some((Token::Invalid(LexerError::UnindentToNewLevel), self.locate()))
                } else {
                    self.return_newline = true;
                    Some((Token::Unindent, self.locate()))
                }
            },
            ('\0', _) => {
                if self.current_position.index > self.file_contents.len() {
                    None
                } else {
                    self.advance_with(Token::EndOfInput)
                }
            },
            ('\r', _) => self.lex_newline(),
            ('\n', _) => self.lex_newline(),
            (c, _) if c.is_whitespace() => { self.advance(); self.next() }
            (c, _) if c.is_digit(10) => self.lex_number(),
            (c, _) if c.is_alphanumeric() || c == '_' => self.lex_alphanumeric(),
            ('"', _) => self.lex_string(),
            ('\'', _) => self.lex_char_literal(),
            ('/', '/') => self.lex_singleline_comment(),
            ('/', '*') => self.lex_multiline_comment(),
            (':', '=') => self.advance2_with(Token::Assignment),
            ('=', '=') => self.advance2_with(Token::EqualEqual),
            ('=', _) => self.advance_with(Token::Equal),
            ('!', '=') => self.advance2_with(Token::NotEqual),
            ('.', '.') => self.advance2_with(Token::Range),
            ('-', '>') => self.advance2_with(Token::RightArrow),
            ('<', '|') => self.advance2_with(Token::ApplyLeft),
            ('|', '>') => self.advance2_with(Token::ApplyRight),
            ('+', '+') => self.advance2_with(Token::Append),
            ('(', ')') => self.advance2_with(Token::UnitLiteral),
            ('<', '=') => self.advance2_with(Token::LessThanOrEqual),
            ('>', '=') => self.advance2_with(Token::GreaterThanOrEqual),
            ('#', _) => self.advance_with(Token::Index),
            ('%', _) => self.advance_with(Token::Modulus),
            ('*', _) => self.advance_with(Token::Multiply),
            ('(', _) => self.advance_with(Token::ParenthesisLeft),
            (')', _) => self.advance_with(Token::ParenthesisRight),
            ('-', _) => self.advance_with(Token::Subtract),
            ('+', _) => self.advance_with(Token::Add),
            ('[', _) => self.advance_with(Token::BracketLeft),
            (']', _) => self.advance_with(Token::BracketRight),
            ('|', _) => self.advance_with(Token::Pipe),
            (':', _) => self.advance_with(Token::Colon),
            (';', _) => self.advance_with(Token::Semicolon),
            (',', _) => self.advance_with(Token::Comma),
            ('.', _) => self.advance_with(Token::MemberAccess),
            ('<', _) => self.advance_with(Token::LessThan),
            ('>', _) => self.advance_with(Token::GreaterThan),
            ('/', _) => self.advance_with(Token::Divide),
            ('\\', _) => self.advance_with(Token::Backslash),
            ('&', _) => self.advance_with(Token::Ampersand),
            (c, _) => self.advance_with(Token::Invalid(LexerError::UnknownChar(c))),
        }
    }
}
