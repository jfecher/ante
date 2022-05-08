//! unsafecache.rs - Provides a container whose elements are never freed
//! and can thus hand out references for any lifetime. Used to store
//! the Ast after parsing.
use std::cell::UnsafeCell;
use std::marker::{PhantomData, PhantomPinned};
use std::pin::Pin;

/// A container whose elements are never freed until the program ends.
/// Since these elements are not freed, they can be retrieved via references
/// with any desired lifetime. Note that this type is unsafe to use if the
/// UnsafeCache itself lives longer than any references it passes out.
#[derive(Debug)]
pub struct UnsafeCache<'a, T: 'a> {
    cache: Vec<Pin<Box<UnsafeCell<T>>>>,
    lifetime: PhantomData<&'a T>,

    /// Ensures we cannot move out of the cache, this would invalidate existing references.
    #[allow(dead_code)]
    no_pin: PhantomPinned,
}

impl<'a, T> UnsafeCache<'a, T> {
    pub fn get_mut(&self, index: usize) -> Option<&'a mut T> {
        let value = self.cache.get(index)?;
        // SAFETY: the contained value is guaranteed to never be deallocated until `self` is,
        // since we neither expose method removing values from the `inner`, nor expose any
        // option to mutate the containing Box. The lifetime should be fine, though this
        // does permit multiple mutable references to a given element
        unsafe { value.get().as_mut() }
    }

    /// Push a new element to the cache and return its index
    pub fn push(&mut self, t: T) -> usize {
        let len = self.cache.len();
        self.cache.push(Box::pin(UnsafeCell::new(t)));
        len
    }
}

impl<'a, T> Default for UnsafeCache<'a, T> {
    fn default() -> UnsafeCache<'a, T> {
        UnsafeCache { cache: vec![], lifetime: PhantomData, no_pin: PhantomPinned }
    }
}
