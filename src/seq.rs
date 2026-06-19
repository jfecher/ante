//! A persistent, contiguous, and growable sequence of elements.
//! A faithful port of `stdlib/src/Seq.an`.
//!
//! Performance is meant to be comparable to a mutable `Vec`:
//! - push: O(1) amortized but can be O(N) with worst-case sharing patterns.
//! - pop: O(1)
//! - get: O(1)
//! - clone: O(1)
//!
//! Each `Seq` holds its own length plus a pointer to a `Data` object shared by
//! potentially many `Seq`s. The `Data` holds the furthest length it has been pushed
//! to, its capacity, two reference counts, and an inline array of elements.
//!
//! Many `Seq`s can share the same data even after the data is pushed to. Pushing an
//! element does not fork as long as the array has spare capacity and either the inner
//! `direct_rc` is 1 with no forks, or the index being written is past the largest index
//! previously written to the shared data. Because each `Seq` stores its length
//! individually and never reads past it, we can safely push past it without
//! reallocating.
//!
//! When there is no spare capacity, or a previously-written index would be overwritten
//! while shared, we reallocate (when `direct_rc == 1 && fork_rc == 0`) or fork to a new
//! array. Forking bitwise-copies the elements and links back to the parent `Data`,
//! recording the length at the fork point and bumping the parent's `fork_rc`. This
//! avoids cloning each element; parent links are only followed on drop or fork.
//!
//! Note that `Seq` is not thread-safe (it uses non-atomic reference counts and raw
//! pointers, so it is neither `Send` nor `Sync`).

#![allow(dead_code)]

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ptr;

/// The shared backing store for one or more [`Seq`]s.
///
/// Laid out as a header followed by an inline array of `cap` elements (a C-style
/// flexible array member). The element array is not a struct field; it lives in the
/// bytes immediately after the header and is reached via [`elements_ptr`].
#[repr(C)]
struct Data<T> {
    /// Number of `Seq` handles pointing directly at this `Data`.
    direct_rc: u32,
    /// Number of descendant forks aliasing our prefix.
    fork_rc: u32,
    /// Furthest length written.
    len: u32,
    cap: u32,
    /// Private elements start at this index.
    fork_point: u32,
    /// Null iff `fork_point == 0`.
    parent: *mut Data<T>,
    _marker: PhantomData<T>,
}

/// Layout of a `Data<T>` with `cap` inline elements, plus the byte offset to the
/// first element. The offset depends only on the header and `align_of::<T>()`, so it
/// is constant across capacities (which is what makes `realloc` valid).
fn data_layout<T>(cap: usize) -> (Layout, usize) {
    Layout::new::<Data<T>>().extend(Layout::array::<T>(cap).unwrap()).unwrap()
}

/// Byte offset from the start of a `Data<T>` allocation to its inline element array.
/// Constant across capacities, which is what makes `realloc` valid.
fn elements_offset<T>() -> usize {
    data_layout::<T>(0).1
}

/// Pointer to the inline element array of `data`.
///
/// # Safety
/// `data` must be non-null and point to a live `Data<T>` allocation.
unsafe fn elements_ptr<T>(data: *mut Data<T>) -> *mut T {
    unsafe { (data as *mut u8).add(elements_offset::<T>()) as *mut T }
}

/// Allocate `layout` bytes for a `Data<T>`, aborting on allocation failure.
unsafe fn alloc_checked<T>(layout: Layout) -> *mut Data<T> {
    let buffer = unsafe { alloc(layout) } as *mut Data<T>;
    if buffer.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    buffer
}

/// Allocate a fresh `Data<T>` with room for `cap` elements and the given `len`, with
/// its header initialized to `direct_rc == 1`, `fork_rc == 0`, and no parent. The
/// element array is left uninitialized.
unsafe fn alloc_data<T>(cap: u32, len: u32) -> *mut Data<T> {
    let (layout, _) = data_layout::<T>(cap as usize);
    let buffer = unsafe { alloc_checked::<T>(layout) };
    unsafe {
        ptr::write(
            buffer,
            Data {
                direct_rc: 1,
                fork_rc: 0,
                len,
                cap,
                fork_point: 0,
                parent: ptr::null_mut(),
                _marker: PhantomData,
            },
        );
    }
    buffer
}

/// A persistent, contiguous, growable sequence. See the module docs.
pub struct Seq<T> {
    len: u32,
    data: *mut Data<T>,
}

impl<T> Seq<T> {
    /// Return an empty `Seq`.
    pub fn empty() -> Seq<T> {
        Seq { len: 0, data: ptr::null_mut() }
    }

    /// Create a `Seq` holding each item from the given iterator.
    pub fn of<I: ExactSizeIterator<Item = T>>(iter: I) -> Seq<T> {
        let mut seq = Seq::with_capacity(iter.len());
        for elem in iter {
            seq = seq.push(elem);
        }
        seq
    }

    /// Create a `Seq` with the given initial capacity.
    pub fn with_capacity(capacity: usize) -> Seq<T> {
        if capacity == 0 {
            return Seq::empty();
        }
        let data = unsafe { alloc_data::<T>(capacity as u32, 0) };
        Seq { len: 0, data }
    }

    /// The number of elements visible through this handle.
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Push an element to this sequence, returning a new sequence.
    pub fn push(mut self, elem: T) -> Seq<T> {
        unsafe {
            if self.data.is_null() {
                let data_ptr = alloc_data::<T>(1, 1);
                ptr::write(elements_ptr(data_ptr), elem);
                self.len = 1;
                self.data = data_ptr;
                return self;
            }

            let data = self.data;
            let cap = (*data).cap;
            let data_len = (*data).len;
            let direct_rc = (*data).direct_rc;
            let fork_rc = (*data).fork_rc;

            // Fast case: just insert the new element in place.
            if self.len < cap && (self.len == data_len || (direct_rc == 1 && fork_rc == 0)) {
                let new_len = self.len + 1;
                let elements = elements_ptr(data);

                // If this was cloned then popped, the popped elements were not dropped.
                // If the alias is later dropped, we would own elements past our own
                // length, so drop the stale element here before writing over it.
                if self.len < data_len {
                    ptr::drop_in_place(elements.add(self.len as usize));
                } else {
                    // Writing to `len` above would shrink the max-written-to len.
                    (*data).len = new_len;
                }

                ptr::write(elements.add(self.len as usize), elem);
                self.len = new_len;
                self.data = data;
                return self;
            }

            // No room (self.len >= cap) or another seq has pushed to the same shared
            // data (self.len != data_len): either way allocate a new buffer.
            let new_cap = (self.len * 2).max(1);
            let (new_layout, _) = data_layout::<T>(new_cap as usize);

            // Fork if any Seqs share this data directly, or if any descendant forks
            // alias this data as a prefix.
            let fork = direct_rc != 1 || fork_rc != 0;

            let buffer = if fork {
                collapse_parents(data);
                (*data).fork_rc += 1;
                (*data).direct_rc -= 1;

                let buffer = alloc_checked::<T>(new_layout);
                // Bitwise-copy the header and the elements we view ([0..self.len)).
                // The remaining elements stay owned by the parent chain.
                let header_and_elems = elements_offset::<T>() + self.len as usize * std::mem::size_of::<T>();
                ptr::copy_nonoverlapping(data as *const u8, buffer as *mut u8, header_and_elems);

                (*buffer).fork_rc = 0;
                (*buffer).fork_point = self.len;
                (*buffer).parent = data;
                buffer
            } else {
                let (old_layout, _) = data_layout::<T>(cap as usize);
                let buffer = realloc(data as *mut u8, old_layout, new_layout.size()) as *mut Data<T>;
                if buffer.is_null() {
                    std::alloc::handle_alloc_error(new_layout);
                }
                buffer
            };

            (*buffer).len = self.len + 1;
            (*buffer).cap = new_cap;
            (*buffer).direct_rc = 1;

            ptr::write(elements_ptr(buffer).add(self.len as usize), elem);
            self.len += 1;
            self.data = buffer;
            self
        }
    }

    /// Return a new `Seq` without the last element, plus the popped element if this
    /// seq was non-empty.
    pub fn pop(mut self) -> (Seq<T>, Option<T>)
    where
        T: Copy,
    {
        if self.len != 0 {
            let elem = self.get_copied(self.len - 1);
            self.len -= 1;
            (self, elem)
        } else {
            (self, None)
        }
    }

    /// Same as [`pop`](Seq::pop) but does not return the last element, and thus does
    /// not require `T: Copy`.
    pub fn remove_last(mut self) -> Seq<T> {
        if self.len != 0 {
            self.len -= 1;
        }
        self
    }

    /// Return a reference to the element at `index`, if in bounds.
    pub fn get(&self, index: u32) -> Option<&T> {
        if index < self.len {
            unsafe { Some(&*elements_ptr(self.data).add(index as usize)) }
        } else {
            None
        }
    }

    /// Return a copy of the element at `index`, if in bounds.
    pub fn get_copied(&self, index: u32) -> Option<T>
    where
        T: Copy,
    {
        self.get(index).copied()
    }

    /// Apply `f` to each element by reference.
    pub fn iter(&self) -> Iter<T> {
        Iter { seq: self, index: 0 }
    }
}

/// Iterator over `&T` produced by `&Seq<T>`.
pub struct Iter<'a, T> {
    seq: &'a Seq<T>,
    index: u32,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let item = self.seq.get(self.index);
        if item.is_some() {
            self.index += 1;
        }
        item
    }
}

impl<'a, T> IntoIterator for &'a Seq<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        Iter { seq: self, index: 0 }
    }
}

impl<T: Debug> Debug for Seq<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for i in 0..self.len {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", self.get(i).unwrap())?;
        }
        write!(f, "]")
    }
}

impl<T> Clone for Seq<T> {
    fn clone(&self) -> Seq<T> {
        if !self.data.is_null() {
            unsafe {
                (*self.data).direct_rc += 1;
            }
        }
        Seq { len: self.len, data: self.data }
    }
}

impl<T> Drop for Seq<T> {
    fn drop(&mut self) {
        if !self.data.is_null() {
            unsafe {
                if (*self.data).direct_rc > 1 {
                    (*self.data).direct_rc -= 1;
                } else {
                    (*self.data).direct_rc = 0;
                    if (*self.data).fork_rc == 0 {
                        drop_data(self.data);
                    }
                }
            }
        }
    }
}

/// Free ancestors that only this child keeps alive. When an ancestor has
/// `direct_rc == 0` and `fork_rc == 1`, this child is its sole referent, so we remove
/// the parent from the chain and place the child in its position.
///
/// # Safety
/// `child` must be non-null and point to a live `Data<T>`.
unsafe fn collapse_parents<T>(child: *mut Data<T>) {
    unsafe {
        let mut parent_ptr = (*child).parent;
        while !parent_ptr.is_null() {
            if (*parent_ptr).direct_rc != 0 || (*parent_ptr).fork_rc != 1 {
                parent_ptr = ptr::null_mut();
            } else {
                // Drop the parent's private tail, otherwise it would be leaked.
                let parent_elems = elements_ptr(parent_ptr);
                for i in (*child).fork_point..(*parent_ptr).len {
                    ptr::drop_in_place(parent_elems.add(i as usize));
                }

                (*child).fork_point = (*parent_ptr).fork_point;
                (*child).parent = (*parent_ptr).parent;
                let (layout, _) = data_layout::<T>((*parent_ptr).cap as usize);
                dealloc(parent_ptr as *mut u8, layout);
                parent_ptr = (*child).parent;
            }
        }
    }
}

/// Walk up the parent chain, dropping each ancestor's private data and freeing its
/// allocation when both its reference counts reach zero.
///
/// # Safety
/// `data` must be non-null and point to a live `Data<T>`.
unsafe fn drop_data<T>(mut data: *mut Data<T>) {
    unsafe {
        while !data.is_null() {
            let elements = elements_ptr(data);
            for i in (*data).fork_point..(*data).len {
                ptr::drop_in_place(elements.add(i as usize));
            }

            let parent = (*data).parent;
            let (layout, _) = data_layout::<T>((*data).cap as usize);
            dealloc(data as *mut u8, layout);
            data = parent;

            if !data.is_null() {
                (*data).fork_rc -= 1;
                if (*data).direct_rc != 0 || (*data).fork_rc != 0 {
                    data = ptr::null_mut();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    fn collect<T: Copy>(seq: &Seq<T>) -> Vec<T> {
        seq.into_iter().copied().collect()
    }

    #[test]
    fn empty_is_empty() {
        let seq: Seq<i32> = Seq::empty();
        assert_eq!(seq.len(), 0);
        assert!(seq.is_empty());
        assert_eq!(seq.get(0), None);
    }

    #[test]
    fn push_get_pop() {
        let mut seq = Seq::empty();
        for i in 0..10 {
            seq = seq.push(i);
        }
        assert_eq!(seq.len(), 10);
        assert_eq!(collect(&seq), (0..10).collect::<Vec<_>>());
        assert_eq!(seq.get(5).copied(), Some(5));
        assert_eq!(seq.get_copied(9), Some(9));
        assert_eq!(seq.get(10), None);

        let (seq, popped) = seq.pop();
        assert_eq!(popped, Some(9));
        assert_eq!(seq.len(), 9);

        let empty: Seq<i32> = Seq::empty();
        let (empty, popped) = empty.pop();
        assert_eq!(popped, None);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn remove_last() {
        let seq = Seq::of(0..5);
        let seq = seq.remove_last();
        assert_eq!(collect(&seq), vec![0, 1, 2, 3]);
        let seq: Seq<i32> = Seq::empty().remove_last();
        assert_eq!(seq.len(), 0);
    }

    #[test]
    fn with_capacity_no_realloc() {
        let mut seq = Seq::with_capacity(8);
        let data = seq.data;
        for i in 0..8 {
            seq = seq.push(i);
        }
        // No reallocation should have occurred while within capacity.
        assert_eq!(seq.data, data);
        assert_eq!(collect(&seq), (0..8).collect::<Vec<_>>());

        let zero: Seq<i32> = Seq::with_capacity(0);
        assert!(zero.data.is_null());
    }

    #[test]
    fn of_and_debug() {
        let seq = Seq::of([1, 2, 3].iter());
        assert_eq!(format!("{seq:?}"), "[1, 2, 3]");
        let empty: Seq<i32> = Seq::of(std::iter::empty());
        assert_eq!(format!("{empty:?}"), "[]");
    }

    #[test]
    fn iter() {
        let seq = Seq::of(0..4);
        let mut sum = 0;
        seq.iter().for_each(|x| sum += *x);
        assert_eq!(sum, 6);
        assert_eq!(collect(&seq), vec![0, 1, 2, 3]);
    }

    #[test]
    fn clone_shares_then_diverges() {
        // Cloning is O(1) and shares the buffer; pushing to the clone past the
        // original's length does not fork.
        let base = Seq::of(0..4);
        let clone = base.clone();
        let extended = clone.push(4);
        assert_eq!(collect(&base), vec![0, 1, 2, 3]);
        assert_eq!(collect(&extended), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn forced_fork_keeps_contents_independent() {
        // The pathological sharing scenario from the module docs: clone, push, then
        // overwrite the same shared slot, forcing a fork each round.
        let mut seq = Seq::empty();
        let mut snapshots = Vec::new();
        for round in 0..16 {
            let view = seq.clone();
            seq = seq.push(round);
            // Overwrite the just-written slot through another handle, forcing a fork.
            let other = view.push(-round);
            snapshots.push(other);
        }
        assert_eq!(collect(&seq), (0..16).collect::<Vec<_>>());
        for (round, snap) in snapshots.iter().enumerate() {
            let round = round as i32;
            let mut expected: Vec<i32> = (0..round).collect();
            expected.push(-round);
            assert_eq!(collect(snap), expected, "snapshot {round}");
        }
    }

    /// Element type that bumps a shared counter on drop, to detect leaks/double-drops.
    struct Tracked {
        counter: Rc<Cell<i64>>,
    }

    impl Tracked {
        fn new(counter: &Rc<Cell<i64>>) -> Tracked {
            counter.set(counter.get() + 1);
            Tracked { counter: counter.clone() }
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.counter.set(self.counter.get() - 1);
        }
    }

    #[test]
    fn drops_every_element_exactly_once() {
        let counter = Rc::new(Cell::new(0));
        {
            let mut seq = Seq::empty();
            for _ in 0..20 {
                seq = seq.push(Tracked::new(&counter));
            }
            // Forks and clones aliasing the same prefix.
            let clone = seq.clone();
            let mut forked = clone.clone();
            for _ in 0..5 {
                forked = forked.push(Tracked::new(&counter));
            }
            // remove_last drops nothing yet; the underlying element is freed on data drop.
            let _shorter = seq.clone().remove_last();
            assert!(counter.get() > 0);
            drop(seq);
            drop(clone);
            drop(forked);
            drop(_shorter);
        }
        // Every live counter increment must be matched by a drop.
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn collapse_parents_chain() {
        // Build a deep fork chain then drop intermediates so collapse_parents runs.
        let counter = Rc::new(Cell::new(0));
        {
            let mut base = Seq::empty();
            for _ in 0..3 {
                base = base.push(Tracked::new(&counter));
            }
            let mut current = base.clone();
            let mut keep = Vec::new();
            for _ in 0..6 {
                let view = current.clone();
                current = current.push(Tracked::new(&counter));
                keep.push(view.push(Tracked::new(&counter)));
            }
            drop(base);
            drop(current);
            drop(keep);
        }
        assert_eq!(counter.get(), 0);
    }
}
