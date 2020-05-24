pub mod token;

use std::str::CharIndices;
use std::collections::HashMap;
use token::{Token, LexerError};

#[derive(Clone)]
pub struct Lexer<'a> {
    current: char,
    next: char,
    input: &'a str,
    current_index: usize,
    indent_levels: Vec<usize>,
    current_indent_level: usize,
    indices: CharIndices<'a>,
    keywords: &'a HashMap<&'a str, Token<'a>>,
}

fn second<T, U>(tup: (T, U)) -> U {
    tup.1
}

impl<'a> Lexer<'a> {
    pub fn get_keywords() -> HashMap<&'a str, Token<'a>> {
        vec![
            ("int", Token::IntegerType),
            ("float", Token::FloatType),
            ("char", Token::CharType),
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

    pub fn new(input: &'a str, keywords: &'a HashMap<&'a str, Token<'a>>) -> Lexer<'a> {
        let mut indices = input.char_indices();
        let current = indices.next().map_or('\0', second);
        let next = indices.next().map_or('\0', second);
        Lexer {
            current,
            next,
            input,
            current_index: 0,
            indent_levels: vec![0],
            current_indent_level: 0,
            indices,
            keywords,
        }
    }

    fn at_end_of_input(&self) -> bool {
        self.current == '\0'
    }

    fn advance(&mut self) -> char {
        let ret = self.current;
        self.current = self.next;
        self.next = self.indices.next().map_or('\0', second);
        self.current_index += 1;
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

    fn advance_while<F>(&mut self, mut f: F) -> &'a str
        where F: FnMut(char, char) -> bool
    {
        let start_index = self.current_index;
        while f(self.current, self.next) && !self.at_end_of_input() {
            self.advance();
        }
        &self.input[start_index .. self.current_index]
    }

    fn expect(&mut self, expected: char, token: Token<'a>) -> Option<Token<'a>> {
        if self.current == expected {
            self.advance_with(token)
        } else {
            self.advance_with(Token::Invalid(LexerError::Expected(expected)))
        }
    }

    fn lex_number(&mut self) -> Option<Token<'a>> {
        let start_index = self.current_index;
        let integer_string = self.advance_while(|current, _| current.is_digit(10));

        if self.current == '.' && self.next.is_digit(10) {
            self.advance();
            self.advance_while(|current, _| current.is_digit(10));
            let float_string = &self.input[start_index .. self.current_index];
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

        match (self.current, self.next) {
            _ if self.current_indent_level < last_indent => {
                self.indent_levels.pop();
                let last_indent = self.indent_levels[self.indent_levels.len() - 1];
                if self.current_indent_level > last_indent {
                    self.current_indent_level = last_indent;
                    Some(Token::Invalid(LexerError::UnindentToNewLevel))
                } else {
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
            (c, _) => self.advance_with(Token::Invalid(LexerError::UnknownChar(c))),
        }
    }
}
