//! logging.rs - Simple tree-based logging for debugging.
//! Useful to trace the compiler's control flow through various programs.
#![allow(dead_code)]
#![allow(unused_macros)]

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

pub static LOG_LEVEL: AtomicUsize = AtomicUsize::new(0);

pub struct Logger;

/// Prints out a log line prepended with the current indent level
macro_rules! log { ( $fmt_string:expr $( , $($msg:tt)* )? ) => ({
    let seq_cst = std::sync::atomic::Ordering::SeqCst;

    print!("{}", " ".repeat($crate::util::logging::LOG_LEVEL.load(seq_cst)));
    println!($fmt_string $( , $($msg)* )?);
    $crate::util::logging::Logger
});}

impl Logger {
    /// Starts a log block, causing all logs within the given function to be
    /// indented more than the logs outside of it. Useful for tracing control
    /// flow for recursive functions.
    pub fn block<F, T>(self, f: F) -> T
        where F: FnOnce() -> T
    {
        LOG_LEVEL.fetch_add(2, SeqCst);
        let result = f();
        LOG_LEVEL.fetch_sub(2, SeqCst);
        result
    }
}
