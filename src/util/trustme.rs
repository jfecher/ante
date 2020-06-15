// This module is the home for any vastly unsafe function.
// This module should only be used as a last resort.

pub fn extend_lifetime_mut<'a, 'b, T>(x: &'a mut T) -> &'b mut T {
    unsafe { std::mem::transmute(x) }
}

pub fn make_mut<'a, 'b, T>(x: *const T) -> &'a mut T {
    unsafe { std::mem::transmute(x) }
}
