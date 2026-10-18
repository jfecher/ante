//! Serializing many [Arc]s to the same value and deserializing them typically results in each
//! previously shared [Arc] being given a fresh allocation, leading to a lot of extra memory and
//! duplicate work. This file implements a [Shared] wrapper with utils to deduplicate during
//! deserialization.
//!
//! TODO: We could consider using interning instead in ante or in inc-complete directly.
use std::{any::Any, cell::RefCell, sync::Arc};

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

type AnyArc = Arc<dyn Any + Send + Sync>;

#[derive(Default)]
struct Session {
    /// Id of each allocation already written. The `Arc` keeps its address from being reused.
    written: FxHashMap<usize, (u32, AnyArc)>,

    /// Each allocation read so far, indexed by id
    read: Vec<AnyArc>,
}

std::thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
}

/// Run `f`, deduplicating every shared `Arc` it serializes or deserializes
pub fn with_sharing<R>(f: impl FnOnce() -> R) -> R {
    let previous = SESSION.with(|session| session.replace(Some(Session::default())));
    let result = f();
    SESSION.with(|session| session.replace(previous));
    result
}

#[derive(Serialize)]
enum Write<'a, T> {
    New(&'a T),
    Ref(u32),
}

#[derive(Deserialize)]
enum Read<T> {
    New(T),
    Ref(u32),
}

/// An `Arc` which is (de)serialized once per allocation when using [with_sharing]
#[derive(Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Self {
        Shared(Arc::new(value))
    }

    pub fn ptr_eq(this: &Self, other: &Self) -> bool {
        Arc::ptr_eq(&this.0, &other.0)
    }

    pub fn as_ptr(this: &Self) -> *const T {
        Arc::as_ptr(&this.0)
    }

    pub fn into_arc(this: Self) -> Arc<T> {
        this.0
    }
}

impl<T: Clone> Shared<T> {
    pub fn make_mut(this: &mut Self) -> &mut T {
        Arc::make_mut(&mut this.0)
    }

    pub fn unwrap_or_clone(this: Self) -> T {
        Arc::unwrap_or_clone(this.0)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T> std::ops::Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<T> for Shared<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> std::borrow::Borrow<T> for Shared<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: Serialize + Send + Sync + 'static> Serialize for Shared<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let address = Arc::as_ptr(&self.0) as *const () as usize;
        let existing = SESSION.with(|session| Some(session.borrow().as_ref()?.written.get(&address)?.0));
        if let Some(id) = existing {
            return Write::<T>::Ref(id).serialize(serializer);
        }

        let result = Write::New(self.0.as_ref()).serialize(serializer)?;
        SESSION.with(|session| {
            if let Some(session) = session.borrow_mut().as_mut() {
                let id = session.written.len() as u32;
                session.written.insert(address, (id, self.0.clone()));
            }
        });
        Ok(result)
    }
}

impl<'de, T: Deserialize<'de> + Send + Sync + 'static> Deserialize<'de> for Shared<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match Read::<T>::deserialize(deserializer)? {
            Read::New(value) => {
                let value = Arc::new(value);
                SESSION.with(|session| {
                    if let Some(session) = session.borrow_mut().as_mut() {
                        session.read.push(value.clone());
                    }
                });
                Ok(Shared(value))
            },
            Read::Ref(id) => {
                let existing = SESSION.with(|session| Some(session.borrow().as_ref()?.read.get(id as usize)?.clone()));
                let existing =
                    existing.ok_or_else(|| serde::de::Error::custom(format!("unknown shared Arc id {id}")))?;
                existing
                    .downcast()
                    .map(Shared)
                    .map_err(|_| serde::de::Error::custom(format!("shared Arc id {id} has the wrong type")))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{Shared, with_sharing};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Inner(String);

    #[derive(Serialize, Deserialize)]
    struct Outer {
        inner: Shared<Inner>,
    }

    #[derive(Serialize, Deserialize)]
    struct Root {
        outers: Vec<Shared<Outer>>,
        inners: Vec<Shared<Inner>>,
    }

    #[test]
    fn shared_arcs_are_written_once_and_stay_shared() {
        let inner = Shared::new(Inner("x".repeat(1000)));
        let outer = Shared::new(Outer { inner: inner.clone() });
        let root = Root {
            outers: vec![outer.clone(), outer, Shared::new(Outer { inner: inner.clone() })],
            inners: vec![inner],
        };

        let unshared = postcard::to_stdvec(&root).unwrap();
        let bytes = with_sharing(|| postcard::to_stdvec(&root).unwrap());
        assert!(bytes.len() < 1100 && unshared.len() > 4000, "{} vs {}", bytes.len(), unshared.len());

        let root: Root = with_sharing(|| postcard::from_bytes(&bytes).unwrap());
        assert!(Shared::ptr_eq(&root.outers[0], &root.outers[1]));
        assert!(Shared::ptr_eq(&root.outers[0].inner, &root.outers[2].inner));
        assert!(Shared::ptr_eq(&root.outers[0].inner, &root.inners[0]));
        assert_eq!(root.inners[0].as_ref().0.len(), 1000);

        // Without a session, references can't be resolved
        assert!(postcard::from_bytes::<Root>(&bytes).is_err());
    }
}
