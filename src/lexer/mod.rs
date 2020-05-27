pub mod token;

use std::str::Chars;
use std::collections::HashMap;
use token::{ Token, LexerError };
use crate::error::location::{ Position, EndPosition, Location, Locatable };

#[derive(Clone)]
pub struct Lexer<'a> {
    current: char,
    next: char,
    filename: &'a str,
    file_contents: &'a str,
    token_start_position: Position,
    current_position: Position,
    indent_levels: Vec<usize>,
    current_indent_level: usize,
    return_newline: bool, // Hack to always return a newline after an Unindent token
    chars: Chars<'a>,
    keywords: &'a HashMap<&'a str, Token<'a>>,
}

impl<'a> Locatable<'a> for Lexer<'a> {
    fn locate(&self) -> Location<'a> {
        let end = EndPosition::new(self.current_position.index);
        Location::new(self.filename, self.file_contents, self.token_start_position, end)
    }
}

impl<'a> Lexer<'a> {
    pub fn get_keywords() -> HashMap<&'a str, Token<'a>> {
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

    pub fn new(filename: &'a str, file_contents: &'a str, keywords: &'a HashMap<&'a str, Token<'a>>) -> Lexer<'a> {
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
            keywords,
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

    fn advance_with(&mut self, token: Token<'a>) -> Option<Token<'a>> {
        self.advance();
        Some(token)
    }

    fn advance2_with(&mut self, token: Token<'a>) -> Option<Token<'a>> {
        self.advance();
        self.advance_with(token)
    }

    fn get_slice_containing_current_token(&self) -> &'a str {
        &self.file_contents[self.token_start_position.index .. self.current_position.index]
    }

    fn advance_while<F>(&mut self, mut f: F) -> &'a str
        where F: FnMut(char, char) -> bool
    {
        while f(self.current, self.next) && !self.at_end_of_input() {
            self.advance();
        }
        self.get_slice_containing_current_token()
    }

    fn expect(&mut self, expected: char, token: Token<'a>) -> Option<Token<'a>> {
        if self.current == expected {
            self.advance_with(token)
        } else {
            self.advance_with(Token::Invalid(LexerError::Expected(expected)))
        }
    }

    fn lex_number(&mut self) -> Option<Token<'a>> {
        let integer_string = self.advance_while(|current, _| current.is_digit(10));

        if self.current == '.' && self.next.is_digit(10) {
            self.advance();
            self.advance_while(|current, _| current.is_digit(10));
            let float_string = self.get_slice_containing_current_token();
            let float = float_string.parse().unwrap();
            Some(Token::FloatLiteral(float))
        } else {
            let integer = integer_string.parse().unwrap();
            Some(Token::IntegerLiteral(integer))
        }
    }

    fn lex_alphanumeric(&mut self) -> Option<Token<'a>> {
        let is_type = self.current.is_uppercase();
        let word = self.advance_while(|current, _| current.is_alphanumeric());

        if is_type {
            Some(Token::TypeName(word))
        } else {
            match self.keywords.get(word) {
                Some(keyword) => Some(keyword.clone()),
                None => Some(Token::Identifier(word)),
            }
        }
    }

    fn lex_string(&mut self) -> Option<Token<'a>> {
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

    fn lex_char_literal(&mut self) -> Option<Token<'a>> {
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

    fn lex_newline(&mut self) -> Option<Token<'a>> {
        self.advance();

        // Must advance start_position otherwise the slice returned by advance_while
        // in recursive calls to lex_newline will be longer than it should be
        self.token_start_position = self.current_position;
        let new_indent = self.advance_while(|current, _| current == ' ').len();

        match (self.current, self.next) {
            ('\n', _) => self.lex_newline(),
            (c, _) if c.is_whitespace() => {
                let error = LexerError::InvalidCharacterInSignificantWhitespace(self.current);
                self.advance_with(Token::Invalid(error))
            },
            ('/', '/') => self.lex_singleline_comment(),
            ('/', '*') => self.lex_multiline_comment(),
            _ if new_indent > self.current_indent_level => self.lex_indent(new_indent),
            _ if new_indent < self.current_indent_level => self.lex_unindent(new_indent),
            _ => Some(Token::Newline)
        }
    }

    fn lex_indent(&mut self, new_indent: usize) -> Option<Token<'a>> {
        if new_indent == self.current_indent_level + 1 {
            self.indent_levels.push(new_indent);
            self.current_indent_level = new_indent;
            Some(Token::Invalid(LexerError::IndentChangeTooSmall))
        } else {
            debug_assert!(new_indent > self.current_indent_level);
            self.indent_levels.push(new_indent);
            self.current_indent_level = new_indent;
            Some(Token::Indent)
        }
    }

    fn lex_unindent(&mut self, new_indent: usize) -> Option<Token<'a>> {
        if self.current_indent_level == new_indent + 1 {
            self.current_indent_level = new_indent;
            Some(Token::Invalid(LexerError::IndentChangeTooSmall))
        } else {
            debug_assert!(new_indent < self.current_indent_level);
            self.current_indent_level = new_indent;
            self.next()
        }
    }

    fn lex_singleline_comment(&mut self) -> Option<Token<'a>> {
        self.advance_while(|current, _| current != '\n');
        self.next()
    }

    fn lex_multiline_comment(&mut self) -> Option<Token<'a>> {
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

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let last_indent = self.indent_levels[self.indent_levels.len() - 1];
        self.token_start_position = self.current_position;

        match (self.current, self.next) {
            _ if self.return_newline => {
                self.return_newline = false;
                Some(Token::Newline)
            },
            _ if self.current_indent_level < last_indent => {
                self.indent_levels.pop();
                let last_indent = self.indent_levels[self.indent_levels.len() - 1];
                if self.current_indent_level > last_indent {
                    self.current_indent_level = last_indent;
                    Some(Token::Invalid(LexerError::UnindentToNewLevel))
                } else {
                    self.return_newline = true;
                    Some(Token::Unindent)
                }
            },
            ('\0', _) => None,
            ('\n', _) => self.lex_newline(),
            (c, _) if c.is_whitespace() => { self.advance(); self.next() }
            (c, _) if c.is_digit(10) => self.lex_number(),
            (c, _) if c.is_alphanumeric() => self.lex_alphanumeric(),
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
            (c, _) => self.advance_with(Token::Invalid(LexerError::UnknownChar(c))),
        }
    }
}
