//! error/mod.rs - Defines the error, warning, and note macros
//! used to issue compiler errors. There is also an ErrorMessage type
//! for storing messages that may be issued later. Note that all issuing
//! an error does is print it to stderr and update the global ERROR_COUNT.
//!
//! Compiler passes are expected to continue even after issuing errors so
//! that as many can be issued as possible. A possible future improvement
//! would be to implement poisoning so that repeated errors are hidden.
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

/// Return an error which may be issued later
macro_rules! make_error {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        $crate::error::ErrorMessage::error(&message[..], $location)
    });
}

/// Issue an error message to stderr and increment the error count
macro_rules! error {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        eprintln!("{}", make_error!($location, $fmt_string $( , $($msg)* )?));
    });
}

/// Return a warning which may be issued later
macro_rules! make_warning {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        $crate::error::ErrorMessage::warning(&message[..], $location)
    });
}

/// Issues a warning to stderr
macro_rules! warning {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        eprintln!("{}", make_warning!($location, $fmt_string $( , $($msg)* )?));
    });
}

/// Return a note which may be issued later
macro_rules! make_note {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        let message = format!($fmt_string $( , $($msg)* )? );
        $crate::error::ErrorMessage::note(&message[..], $location)
    });
}

/// Issues a note to stderr
macro_rules! note {
    ( $location:expr , $fmt_string:expr $( , $($msg:tt)* )? ) => ({
        eprintln!("{}", make_note!($location, $fmt_string $( , $($msg)* )?));
    });
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorType {
    Error,
    Warning,
    Note,
}

/// An error (or warning/note) message to be printed out on screen.
#[derive(Debug, PartialEq, Eq)]
pub struct ErrorMessage<'a> {
    msg: ColoredString,
    error_type: ErrorType,
    location: Location<'a>,
}

/// ErrorMessages are ordered so we can issue them in a
/// deterministic order for the golden tests.
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

    /// Color the given string in either the error, warning, or note color
    fn color(&self, msg: &str) -> ColoredString {
        match (COLORED_OUTPUT.load(SeqCst), self.error_type) {
            (false, _) => msg.normal(),
            (_, ErrorType::Error) => msg.red(),
            (_, ErrorType::Warning) => msg.yellow(),
            (_, ErrorType::Note) => msg.purple(),
        }
    }
}

/// Reads the given file, returning all of its contents
fn read_file_or_panic(path: &Path) -> String {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader.read_to_string(&mut contents).unwrap();
    contents
}

/// Sets whether error message output should be colored or not
pub fn color_output(should_color: bool) {
    COLORED_OUTPUT.store(should_color, SeqCst);
}

pub fn get_error_count() -> usize {
    ERROR_COUNT.load(SeqCst)
}

/// Format the path in an OS-agnostic way. By default rust uses "/" on Unix
/// and "\" on windows as the path separator. This makes testing more
/// difficult and isn't needed for error reporting so we implement our own
/// path-Displaying here that is roughly the same as printing Unix paths.
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

    // An arbitrary length to start truncating filenames at.
    // Chosen to match a good length for the default prelude path.
    let arbitrary_big_len = 26;

    if ret.len() > arbitrary_big_len {
        let cutoff = ret.len() - arbitrary_big_len + 3;
        let mut shortened = "...".to_owned();
        shortened += &ret[cutoff ..];
        ret = shortened;
    }

    if COLORED_OUTPUT.load(SeqCst) {
        ret.italic()
    } else {
        ret.normal()
    }
}

impl<'a> Display for Location<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let filename = os_agnostic_display_path(self.filename);
        write!(f, "{}: {},{}", filename, self.start.line, self.start.column)
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

        writeln!(f, "{}\t{} {}", self.location, self.marker(), self.msg)?;

        let file_contents = read_file_or_panic(self.location.filename);
        let line = file_contents.lines().nth(max(1, start.line) as usize - 1).unwrap();

        let start_column = max(1, start.column) as usize - 1;
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
