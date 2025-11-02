use std::collections::BTreeMap;

/// Shorthand for `items.into_iter().map(f).collect()`
#[inline]
pub(crate) fn map<T, U, R>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> U) -> R
where
    R: FromIterator<U>,
{
    items.into_iter().map(f).collect()
}

/// Shorthand for `items.into_iter().map(f).collect::<Vec<_>>()`
#[inline]
pub(crate) fn vecmap<T, U>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> U) -> Vec<U> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<Result<Vec<_>, _>>()`
#[inline]
pub(crate) fn try_vecmap<T, U, E>(
    items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> Result<U, E>,
) -> Result<Vec<U>, E> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<BTreeMap<_>>()`
#[inline]
pub(crate) fn btree_map<T, K: Ord, V>(
    items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> (K, V),
) -> BTreeMap<K, V> {
    map(items, f)
}
