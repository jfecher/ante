use std::fmt::Formatter;
use std::path::Path;
use super::ErrorMessage;
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
    pub index: usize,
}

impl EndPosition {
    pub fn new(index: usize) -> EndPosition {
        EndPosition { index }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct File<'a> {
    pub filename: &'a Path,
    pub contents: &'a str,
}

#[derive(Debug, Copy, Clone)]
pub struct Location<'a> {
    pub filename: &'a Path,
    pub file_contents: &'a str,
    pub start: Position,
    pub end: EndPosition,
}

impl<'a> Location<'a> {
    pub fn new(filename: &'a Path, file_contents: &'a str, start: Position, end: EndPosition) -> Location<'a> {
        Location { filename, file_contents, start, end }
    }

    pub fn len(&self) -> usize {
        self.end.index - self.start.index
    }

    pub fn union(&self, other: Location<'a>) -> Location<'a> {
        let start = if self.start.index < other.start.index { self.start } else { other.start };
        let end = if self.end.index < other.end.index { self.end } else { other.end };

        Location {
            filename: self.filename,
            file_contents: self.file_contents,
            start,
            end
        }
    }

    pub fn fmt_error<Msg>(&self, f: &mut Formatter, msg: Msg) -> Result<(), std::fmt::Error>
        where Msg: Into<ErrorMessage>
    {
        use std::cmp::{ min, max };

        writeln!(f, "{}: {},{}\t{}: {}", self.filename.to_string_lossy().italic(), self.start.line, self.start.column, "error".red(), msg.into().0)?;
        let line = self.file_contents.lines().nth(self.start.line as usize - 1).unwrap();

        let start_column = self.start.column as usize - 1;
        let actual_len = min(self.len(), line.len() - start_column);

        // In case we have an odd Location that has start.index = end.index,
        // we show a minimum of one indicator (^) to show where the error is.
        let adjusted_len = max(1, actual_len);

        // write the first part of the line, then the erroring part in red, then the rest
        write!(f, "{}", &line[0 .. start_column])?;
        write!(f, "{}", &line[start_column .. start_column + actual_len].red())?;
        writeln!(f, "{}", &line[start_column + actual_len ..])?;

        let padding = " ".repeat(start_column);
        let indicator = "^".repeat(adjusted_len).red();
        writeln!(f, "{}{}", padding, indicator)
    }
}

pub trait Locatable<'a> {
    fn locate(&self) -> Location<'a>;
}
