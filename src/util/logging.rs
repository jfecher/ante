#![allow(dead_code)]
#![allow(unused_macros)]

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

pub static LOG_LEVEL: AtomicUsize = AtomicUsize::new(0);

pub struct Logger;

macro_rules! log { ( $fmt_string:expr $( , $($msg:tt)* )? ) => ({
    let seq_cst = std::sync::atomic::Ordering::SeqCst;

    print!("{}", " ".repeat($crate::util::logging::LOG_LEVEL.load(seq_cst)));
    println!($fmt_string $( , $($msg)* )?);
    $crate::util::logging::Logger
});}

impl Logger {
    pub fn block<F, T>(self, f: F) -> T
        where F: FnOnce() -> T
    {
        LOG_LEVEL.fetch_add(2, SeqCst);
        let result = f();
        LOG_LEVEL.fetch_sub(2, SeqCst);
        result
    }
}
