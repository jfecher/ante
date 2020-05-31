pub mod location;
use crate::error::location::Location;

use std::fmt::{ Display, Formatter };
use colored::ColoredString;
use colored::*;

macro_rules! error {
    ( $location:expr , $fmt_string:expr , $($msg:tt)* ) => ({
        let message = format!($fmt_string, $($msg),*);
        crate::error::ErrorMessage::error(&message[..], $location)
    });
}

macro_rules! warning {
    ( $location:expr , $fmt_string:expr , $($msg:tt)* ) => ({
        let message = format!($fmt_string, $($msg),*);
        crate::error::ErrorMessage::warning(&message[..], $location)
    });
}

macro_rules! note {
    ( $location:expr , $fmt_string:expr , $($msg:tt)* ) => ({
        let message = format!($fmt_string, $($msg),*);
        crate::error::ErrorMessage::note(&message[..], $location)
    });
}

pub enum ErrorType {
    Error,
    Warning,
    Note,
}

impl ErrorType {
    fn marker(&self) -> ColoredString {
        match self {
            ErrorType::Error => self.color("error"),
            ErrorType::Warning => self.color("warning"),
            ErrorType::Note => self.color("note"),
        }
    }

    fn color(&self, msg: &str) -> ColoredString {
        match self {
            ErrorType::Error => msg.red(),
            ErrorType::Warning => msg.yellow(),
            ErrorType::Note => msg.purple(),
        }
    }
}

pub struct ErrorMessage<'a> {
    msg: ColoredString,
    error_type: ErrorType,
    location: Location<'a>,
}

impl<'a> ErrorMessage<'a> {
    pub fn error<T: Into<ColoredString>>(msg: T, location: Location<'a>) -> ErrorMessage<'a> {
        ErrorMessage { msg: msg.into(), location, error_type: ErrorType::Error }
    }

    pub fn warning<T: Into<ColoredString>>(msg: T, location: Location<'a>) -> ErrorMessage<'a> {
        ErrorMessage { msg: msg.into(), location, error_type: ErrorType::Warning }
    }

    pub fn note<T: Into<ColoredString>>(msg: T, location: Location<'a>) -> ErrorMessage<'a> {
        ErrorMessage { msg: msg.into(), location, error_type: ErrorType::Note }
    }
}

impl<'a> Display for ErrorMessage<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        use std::cmp::{ min, max };

        let start = self.location.start;

        writeln!(f, "{}: {},{}\t{}: {}", self.location.filename.to_string_lossy().italic(),
            start.line, start.column,
            self.error_type.marker(), self.msg)?;

        let line = self.location.file_contents.lines().nth(start.line as usize - 1).unwrap();

        let start_column = start.column as usize - 1;
        let actual_len = min(self.location.len(), line.len() - start_column);

        // In case we have an odd Location that has start.index = end.index,
        // we show a minimum of one indicator (^) to show where the error is.
        let adjusted_len = max(1, actual_len);

        // write the first part of the line, then the erroring part in red, then the rest
        write!(f, "{}", &line[0 .. start_column])?;
        write!(f, "{}", &line[start_column .. start_column + actual_len].red())?;
        writeln!(f, "{}", &line[start_column + actual_len ..])?;

        let padding = " ".repeat(start_column);
        let indicator = self.error_type.color(&"^".repeat(adjusted_len));
        writeln!(f, "{}{}", padding, indicator)
    }
}
