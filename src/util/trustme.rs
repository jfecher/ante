//! This module is the home for any vastly unsafe function.
//! This module should only be used as a last resort.
//!
//! Currently, these functions are only used when retrieving the
//! definition of variables from the cache to separate the mutability/lifetime
//! of the definition from that of the cache. These should be able
//! to be removed if the AST nodes ever become arena allocated and
//! a key can be used from the arena instead of a direct reference in the ModuleCache.

pub fn extend_lifetime<'a, 'b, T>(x: &'a mut T) -> &'b mut T {
    unsafe { std::mem::transmute(x) }
}

pub fn make_mut<'a, T>(x: *const T) -> &'a mut T {
    #[allow(clippy::transmute_ptr_to_ref)]
    unsafe {
        std::mem::transmute(x)
    }
}

pub fn make_mut_ref<'a, 'b, T>(x: &'a T) -> &'b mut T {
    #[allow(mutable_transmutes)]
    unsafe {
        std::mem::transmute(x)
    }
}
