#![allow(unused)]
use std::marker::PhantomData;

/// A dense map from keys of type K to values of type V.
///
/// Only values are stored internally, and keys correspond
/// to the integer indices these values are stored at. To keep
/// these indices consistent, this map is append-only.
pub struct VecMap<K, V> {
    data: Vec<V>,
    tag: PhantomData<K>,
}

impl<K, V> VecMap<K, V> {
    pub fn new() -> Self {
        Self { data: Vec::new(), tag: PhantomData }
    }

    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }

    pub fn push(&mut self, element: V) -> K
    where
        K: From<u32>,
    {
        let index = self.len();
        self.data.push(element);
        index.into()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.data.iter()
    }
}

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        VecMap { data: Vec::new(), tag: PhantomData }
    }
}

impl<K: Into<u32>, V> std::ops::Index<K> for VecMap<K, V> {
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.data[index.into() as usize]
    }
}

impl<K: Into<u32>, V> std::ops::IndexMut<K> for VecMap<K, V> {
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.data[index.into() as usize]
    }
}
