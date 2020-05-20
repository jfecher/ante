use std::str::CharIndices;

use crate::token::Token;

type Tokens<'a> = Vec<Token<'a>>;

struct Lexer<'a> {
    current: char,
    next: char,
    input: &'a str,
    current_index: usize,
    indices: CharIndices<'a>,
}

fn second<T, U>(tup: (T, U)) -> U {
    tup.1
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Lexer<'a> {
        let mut indices = input.char_indices();
        let current = indices.next().map_or('\0', second);
        let next = indices.next().map_or('\0', second);
        Lexer { current, next, input, current_index: 0, indices }
    }

    fn advance(&mut self) {
        self.current = self.next;
        self.next = self.indices.next().map_or('\0', second);
        self.current_index += 1;
    }

    fn at_end_of_input(&self) -> bool {
        self.current == '\0'
    }

    fn advance_with(&mut self, token: Token<'a>) -> Option<Token<'a>> {
        self.advance();
        Some(token)
    }

    fn advance_while<F>(&mut self, f: F) -> &'a str
        where F: Fn(&mut Lexer) -> bool
    {
        let start_index = self.current_index;
        while f(self) && !self.at_end_of_input() {
            self.advance();
        }
        &self.input[start_index .. self.current_index]
    }

    fn lex_integer(&mut self) -> Option<Token<'a>> {
        let integer_string = self.advance_while(|this| this.current.is_digit(10));
        let integer = integer_string.parse().unwrap();
        Some(Token::IntegerLiteral(integer))
    }

    fn lex_identifier(&mut self) -> Option<Token<'a>> {
        let identifier = self.advance_while(|this| this.current.is_alphanumeric());
        Some(Token::Identifier(identifier))
    }

    fn lex_string(&mut self) -> Option<Token<'a>> {
        self.advance();
        let string = self.advance_while(|this| this.current != '"');

        if self.current != '"' {
            Some(Token::Invalid)
        } else {
            self.advance();
            Some(Token::StringLiteral(string))
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current {
            '\0' => None,
            c if c.is_whitespace() => { self.advance(); self.next() }
            c if c.is_digit(10) => self.lex_integer(),
            c if c.is_alphanumeric() => self.lex_identifier(),
            '"' => self.lex_string(),
            '=' => self.advance_with(Token::Equal),
            _ => self.advance_with(Token::Invalid),
        }
    }
}

pub fn lex(input: &str) -> Tokens {
    let lexer = Lexer::new(input);
    lexer.collect()
}
