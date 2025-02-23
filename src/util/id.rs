use std::marker::PhantomData;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<T> {
    index: u32,
    tag: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(index: u32) -> Self {
        Self { index, tag: PhantomData }
    }
}

impl<T> From<u32> for Id<T> {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl<T> From<Id<T>> for u32 {
    fn from(value: Id<T>) -> Self {
        value.index
    }
}
