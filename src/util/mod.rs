//! util/mod.rs - Various utility functions used throughout the compiler.
//! Mostly consists of convenience functions for iterators such as `fmap`.
use std::fmt::Display;
use std::sync::atomic::AtomicBool;

#[macro_use]
pub mod logging;
pub mod trustme;

pub static TIME_PASSES: AtomicBool = AtomicBool::new(false);

/// Equivalent to .iter().map(f).collect()
pub fn fmap<T, U, F>(array: &[T], f: F) -> Vec<U>
    where F: FnMut(&T) -> U
{
    array.iter().map(f).collect()
}

/// What a name! Iterate the array, mapping each element with a function that returns a pair
/// of a value and a vector. Accumulate the results in two separate vectors, the second of
/// which is flattened from all the second-element vectors found so far.
pub fn fmap_mut_pair_flatten_second<T, Ret1, Ret2, F>(array: &mut [T], mut f: F) -> (Vec<Ret1>, Vec<Ret2>)
    where F: FnMut(&mut T) -> (Ret1, Vec<Ret2>)
{
    let mut ret1 = Vec::with_capacity(array.len());
    let mut ret2 = Vec::with_capacity(array.len());
    for elem in array.iter_mut() {
        let (elem1, mut vec) = f(elem);
        ret1.push(elem1);
        ret2.append(&mut vec);
    }
    (ret1, ret2)
}

/// Equivalent to option.as_ref().unwrap().clone()
pub fn unwrap_clone<T: Clone>(option: &Option<T>) -> T {
    option.as_ref().unwrap().clone()
}

/// Transmute a f64 to a u64 so it can be hashed
pub fn reinterpret_as_bits(x: f64) -> u64 {
    unsafe { std::mem::transmute(x) }
}

/// Transmute a u64 back into a f64 to get the value of a FloatLiteral
pub fn reinterpret_from_bits(x: u64) -> f64 {
    unsafe { std::mem::transmute(x) }
}

/// Convert each element to a String and join them with the given delimiter
pub fn join_with<T: Display>(vec: &[T], delimiter: &str) -> String {
    fmap(&vec, |t| format!("{}", t)).join(delimiter)
}

/// Set whether the time! macro should print out the timings of each pass or not
pub fn time_passes(should_time: bool) {
    TIME_PASSES.store(should_time, std::sync::atomic::Ordering::SeqCst);
}

macro_rules! time {( $pass_name:expr, $pass:expr ) => ({
    if $crate::util::TIME_PASSES.load(std::sync::atomic::Ordering::SeqCst) {
        let start = std::time::Instant::now();
        let result = $pass;
        println!("{: <23} - {}us", $pass_name, start.elapsed().as_micros());
        result
    } else {
        $pass
    }
});}
