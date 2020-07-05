pub mod location;
use crate::error::location::Location;

use std::cmp::{min, max};
use std::fmt::{ Display, Formatter };
use std::fs::File;
use std::io::{ BufReader, Read };
use std::path::Path;
use std::sync::atomic::{ AtomicBool, AtomicUsize };
use std::sync::atomic::Ordering::SeqCst;
use colored::ColoredString;
use colored::*;

static COLORED_OUTPUT: AtomicBool = AtomicBool::new(true);

static ERROR_COUNT: AtomicUsize = AtomicUsize::new(0);

macro_rules! make_error {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        crate::error::ErrorMessage::error(&message[..], $location)
    });
}

macro_rules! error {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        println!("{}", make_error!($location, $fmt_string $( , $($msg)* )?));
    });
}

macro_rules! make_warning {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        crate::error::ErrorMessage::warning(&message[..], $location)
    });
}

macro_rules! warning {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        println!("{}", make_warning!($location, $fmt_string $( , $($msg)* )?));
    });
}

macro_rules! make_note {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        crate::error::ErrorMessage::note(&message[..], $location)
    });
}

macro_rules! note {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        println!("{}", make_note!($location, $fmt_string $( , $($msg)* )?));
    });
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorType {
    Error,
    Warning,
    Note,
}

#[derive(PartialEq, Eq)]
pub struct ErrorMessage<'a> {
    msg: ColoredString,
    error_type: ErrorType,
    location: Location<'a>,
}

impl<'a> Ord for ErrorMessage<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::ops::Deref;
        (self.location, self.error_type, self.msg.deref()).cmp(&(other.location, other.error_type, &other.msg))
    }
}

impl<'a> PartialOrd for ErrorMessage<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>{
        self.location.partial_cmp(&other.location)
    }
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

    fn marker(&self) -> ColoredString {
        match self.error_type {
            ErrorType::Error => self.color("error:"),
            ErrorType::Warning => self.color("warning:"),
            ErrorType::Note => self.color("note:"),
        }
    }

    fn color(&self, msg: &str) -> ColoredString {
        match (COLORED_OUTPUT.load(SeqCst), self.error_type) {
            (false, _) => msg.normal(),
            (_, ErrorType::Error) => msg.red(),
            (_, ErrorType::Warning) => msg.yellow(),
            (_, ErrorType::Note) => msg.purple(),
        }
    }
}

fn read_file_or_panic(path: &Path) -> String {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    contents
}

pub fn color_output(should_color: bool) {
    COLORED_OUTPUT.store(should_color, SeqCst);
}

pub fn get_error_count() -> usize {
    ERROR_COUNT.load(SeqCst)
}

// Format the path in an OS-agnostic way. By default rust uses "/" on Unix
// and "\" on windows as the path separator. This makes testing more
// difficult and isn't needed for error reporting so we implement our own
// path-Displaying here that is roughly the same as printing Unix paths.
fn os_agnostic_display_path(path: &Path) -> ColoredString {
    let mut ret = String::new();

    for (i, component) in path.components().enumerate() {
        use std::path::Component;

        // Use / as the separator regardless of the host OS so
        // we can use the same tests for Linux/Mac/Windows
        if i != 0 && ret != "/" {
            ret += "/";
        }

        ret += match component {
            Component::CurDir => ".",
            Component::Normal(s) => s.to_str().expect("Path contains invalid utf-8"),
            Component::ParentDir => "..",
            Component::Prefix(_) => "",
            Component::RootDir => "/",
        }
    }

    if COLORED_OUTPUT.load(SeqCst) {
        ret.italic()
    } else {
        ret.normal()
    }
}

impl<'a> Display for ErrorMessage<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let start = self.location.start;

        // An error isn't considered an error until it is actually printed out.
        // That's why ERROR_COUNT is incremented here and not when ErrorMessage is constructed.
        if self.error_type == ErrorType::Error {
            ERROR_COUNT.fetch_add(1, SeqCst);
        }

        let filename = os_agnostic_display_path(&self.location.filename);

        writeln!(f, "{}: {},{}\t{} {}", filename,
            start.line, start.column, self.marker(), self.msg)?;

        let file_contents = read_file_or_panic(self.location.filename);
        let line = file_contents.lines().nth(start.line as usize - 1).unwrap();

        let start_column = start.column as usize - 1;
        let actual_len = min(self.location.len(), line.len() - start_column);

        // In case we have an odd Location that has start.index = end.index,
        // we show a minimum of one indicator (^) to show where the error is.
        // let adjusted_len = max(1, actual_len);

        // write the first part of the line, then the erroring part in red, then the rest
        write!(f, "{}", &line[0 .. start_column])?;
        write!(f, "{}", self.color(&line[start_column .. start_column + actual_len]))?;
        writeln!(f, "{}", &line[start_column + actual_len ..])?;

        if !COLORED_OUTPUT.load(SeqCst) {
            let padding = " ".repeat(start_column);
            let indicator = self.color(&"^".repeat(max(1, actual_len)));
            writeln!(f, "{}{}", padding, indicator)?;
        }
        Ok(())
    }
}
