use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub enum TestError {
    ExpectedOutputDiffers,
}

// This isn't displayed when running tests via 'cargo test' anyway
impl fmt::Display for TestError {
    fn fmt(&self, _: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        Ok(())
    }
}

impl Error for TestError { } 
