use std::fmt::Display;

pub mod trustme;

pub fn fmap<T, U, F>(array: &[T], mut f: F) -> Vec<U>
    where F: FnMut(&T) -> U
{
    array.iter().map(|x| f(x)).collect()
}

#[allow(dead_code)]
pub fn fmap_mut<T, U, F>(array: &mut [T], mut f: F) -> Vec<U>
    where F: FnMut(&mut T) -> U
{
    array.iter_mut().map(|x| f(x)).collect()
}

/// What a name! Iterate the array, mapping each element with a function that returns a pair
/// of a value and a vector. Accumulate the results in two separate vectors, the second of
/// which is merged from all the second-element vectors found so far.
pub fn fmap_mut_pair_merge_second<T, Ret1, Ret2, F>(array: &mut [T], mut f: F) -> (Vec<Ret1>, Vec<Ret2>)
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

pub fn contains<T: PartialEq>(array: &[T], element: &T) -> bool {
    array.iter().find(|&x| x == element).is_some()
}

pub fn join_with<T: Display>(vec: &[T], delimiter: &str) -> String {
    fmap(&vec, |t| format!("{}", t)).join(delimiter)
}
