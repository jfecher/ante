use std::{collections::BTreeMap, sync::Arc};

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
pub(crate) fn mapvec<T, U>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> U) -> Vec<U> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<Result<Vec<_>, _>>()`
#[inline]
pub(crate) fn try_mapvec<T, U, E>(
    items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> Result<U, E>,
) -> Result<Vec<U>, E> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<Option<Vec<_>, _>>()`
#[inline]
pub(crate) fn opt_mapvec<T, U>(items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> Option<U>) -> Option<Vec<U>> {
    map(items, f)
}

/// Shorthand for `items.into_iter().map(f).collect::<BTreeMap<_>>()`
#[inline]
pub(crate) fn map_btree<T, K: Ord, V>(
    items: impl IntoIterator<Item = T>, f: impl FnMut(T) -> (K, V),
) -> BTreeMap<K, V> {
    map(items, f)
}

/// Sort each group of `items` between two boundary items using `compare`. This is a stable sort and
/// no item is ever moved past a boundary.
pub(crate) fn sort_runs<T>(
    items: &mut [T], is_boundary: impl Fn(&T) -> bool, mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering,
) {
    let mut start = 0;
    for end in 0..=items.len() {
        if end == items.len() || is_boundary(&items[end]) {
            items[start..end].sort_by(&mut compare);
            start = end + 1;
        }
    }
}

pub(crate) fn join_arc_str(strings: &[Arc<String>], separator: &str) -> String {
    let mut strings_iter = strings.iter();
    let Some(first) = strings_iter.next() else {
        return String::new();
    };

    let mut result = first.as_ref().clone();
    for next in strings_iter {
        result += separator;
        result += next;
    }
    result
}
