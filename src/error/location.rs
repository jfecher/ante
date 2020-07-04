use std::path::Path;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub index: usize,
    pub line: u32,
    pub column: u16,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Location<'a> {
    pub filename: &'a Path,
    pub start: Position,
    pub end: EndPosition,
}

impl<'a> Ord for Location<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.start, self.end).cmp(&(other.start, other.end))
    }
}

impl<'a> PartialOrd for Location<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Location<'a> {
    pub fn new(filename: &'a Path, start: Position, end: EndPosition) -> Location<'a> {
        Location { filename, start, end }
    }

    pub fn len(&self) -> usize {
        self.end.index - self.start.index
    }

    pub fn union(&self, other: Location<'a>) -> Location<'a> {
        let start = if self.start.index < other.start.index { self.start } else { other.start };
        let end = if self.end.index < other.end.index { self.end } else { other.end };

        Location {
            filename: self.filename,
            start,
            end
        }
    }
}

pub trait Locatable<'a> {
    fn locate(&self) -> Location<'a>;
}
