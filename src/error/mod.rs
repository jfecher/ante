pub mod location;

use std::convert::From;
use colored::ColoredString;
use colored::*;

pub struct ErrorMessage(ColoredString);

impl From<String> for ErrorMessage {
    fn from(s: String) -> ErrorMessage {
        ErrorMessage(s.normal())
    }
}

impl From<&str> for ErrorMessage {
    fn from(s: &str) -> ErrorMessage {
        ErrorMessage(s.normal())
    }
}

impl From<ColoredString> for ErrorMessage {
    fn from(s: ColoredString) -> ErrorMessage {
        ErrorMessage(s)
    }
}
