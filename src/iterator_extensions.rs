use std::{collections::BTreeMap, sync::Arc};

/// Shorthand for `items.into_iter().map(f).collect()`
#[inline]
fn map<T, U, R>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> U) -> R
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

/// Shorthand for `items.into_iter().map(f).collect::<Option<Vec<_>, _>>()`
#[inline]
pub(crate) fn opt_vecmap<T, U>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> Option<U>) -> Option<Vec<U>> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<BTreeMap<_>>()`
#[inline]
pub(crate) fn btree_map<T, K: Ord, V>(
    items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> (K, V),
) -> BTreeMap<K, V> {
    map(items, f)
}

pub(crate) fn join_arc_str(strings: Vec<Arc<String>>, separator: &str) -> String {
    let mut strings_iter = strings.into_iter();
    let Some(first) = strings_iter.next() else { return String::new(); };

    let mut result = first.as_ref().clone();
    for next in strings_iter {
        result += separator;
        result += &next;
    }
    result
}
