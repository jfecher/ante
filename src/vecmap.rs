use std::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct VecMap<K, V> {
    items: Vec<V>,
    id: PhantomData<K>,
}

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self { items: Vec::new(), id: PhantomData }
    }
}

impl<K, V> VecMap<K, V> {
    pub fn push(&mut self, item: V) -> K
    where
        K: From<usize>,
    {
        let key = K::from(self.items.len());
        self.items.push(item);
        key
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<K, V> Index<K> for VecMap<K, V>
where
    K: Into<usize>,
{
    type Output = V;

    fn index(&self, index: K) -> &Self::Output {
        &self.items[index.into()]
    }
}

impl<K, V> IndexMut<K> for VecMap<K, V>
where
    K: Into<usize>,
{
    fn index_mut(&mut self, index: K) -> &mut Self::Output {
        &mut self.items[index.into()]
    }
}

impl<K, V> Serialize for VecMap<K, V>
where
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.items.serialize(serializer)
    }
}

impl<'de, K, V> Deserialize<'de> for VecMap<K, V>
where
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let items = Deserialize::deserialize(deserializer)?;
        Ok(VecMap { items, id: PhantomData })
    }
}
