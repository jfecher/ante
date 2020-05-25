use crate::lexer::{ File, Lexer };
use std::fmt::Formatter;
use colored::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Position {
    pub index: usize,
    line: u32,
    column: u16,
}

impl Position {
    pub fn begin() -> Position {
        Position {
            index: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn advance(&mut self, passed_newline: bool) {
        if passed_newline {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        self.index += 1;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EndPosition {
    index: usize,
}

impl EndPosition {
    pub fn new(index: usize) -> EndPosition {
        EndPosition { index }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Location<'a> {
    file: File<'a>,
    start: Position,
    end: EndPosition,
}

impl<'a> Location<'a> {
    pub fn new(lexer: &Lexer<'a>, start: Position, end: EndPosition) -> Location<'a> {
        Location { file: lexer.file, start, end }
    }

    pub fn len(&self) -> usize {
        self.end.index - self.start.index
    }

    pub fn union(&self, other: Location<'a>) -> Location<'a> {
        let start = if self.start.index < other.start.index { self.start } else { other.start };
        let end = if self.end.index < other.end.index { self.end } else { other.end };

        Location { file: self.file, start, end }
    }
    
    pub fn fmt_error(&self, fmt: &mut Formatter, msg: &'a str) -> Result<(), std::fmt::Error> {
        use std::cmp::{ min, max };

        writeln!(fmt, "{}: {},{}\t{}: {}", self.file.filename.italic(), self.start.line, self.start.column, "error".red(), msg)?;
        let line = self.file.contents.lines().nth(self.start.line as usize - 1).unwrap();
        writeln!(fmt, "{}", line)?;
        let padding = " ".repeat(self.start.column as usize - 1);
        let remaining_columns_after_padding = line.len() - padding.len();
        let indicator = "^".repeat(max(1, min(self.len(), remaining_columns_after_padding))).red();
        writeln!(fmt, "{}{}", padding, indicator)
    }
}

pub trait Locatable<'a> {
    fn locate(&self) -> Location<'a>;
}
