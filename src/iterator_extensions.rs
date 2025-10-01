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
