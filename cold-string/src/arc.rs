use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    str::Utf8Error,
    string::String,
};
use core::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    iter::FromIterator,
    ops::Deref,
    str,
};

#[cfg(all(loom, test, target_arch = "x86_64"))]
use loom::sync::atomic::{fence, AtomicU16, AtomicU32, AtomicU8, AtomicUsize, Ordering::*};
#[cfg(not(all(loom, test, target_arch = "x86_64")))]
use portable_atomic::{fence, AtomicU16, AtomicU32, AtomicU8, AtomicUsize, Ordering::*};

use crate::encoded::Encoded;
use crate::heap::Global;

#[doc(hidden)]
pub trait RefCount: Send + Sync + 'static {
    fn new() -> Self;
    fn increment(&self);
    fn decrement(&self) -> bool;

    #[cfg(test)]
    fn refs(&self) -> usize;

    #[cfg(test)]
    fn near_overflow() -> Self;

    #[cfg(test)]
    fn is_immortal(&self) -> bool;

    #[cfg(all(test, not(all(loom, target_arch = "x86_64"))))]
    fn min_immortal() -> Self;
}

macro_rules! impl_ref_count {
    ($atomic:ty, $int:ty) => {
        impl RefCount for $atomic {
            #[inline]
            fn new() -> Self {
                <$atomic>::new(2)
            }

            #[inline]
            fn increment(&self) {
                let mut old = self.load(Relaxed);
                loop {
                    if old & 1 != 0 {
                        return;
                    }

                    let new = if old == <$int>::MAX - 1 {
                        <$int>::MAX
                    } else {
                        old + 2
                    };
                    match self.compare_exchange_weak(old, new, Relaxed, Relaxed) {
                        Ok(_) => return,
                        Err(actual) => old = actual,
                    }
                }
            }

            #[inline]
            fn decrement(&self) -> bool {
                self.fetch_sub(2, Release) == 2
            }

            #[cfg(test)]
            fn refs(&self) -> usize {
                (self.load(Relaxed) >> 1) as usize
            }

            #[cfg(test)]
            fn near_overflow() -> Self {
                Self::new(<$int>::MAX - 1)
            }

            #[cfg(test)]
            fn is_immortal(&self) -> bool {
                self.load(Relaxed) & 1 != 0
            }

            #[cfg(all(test, not(all(loom, target_arch = "x86_64"))))]
            fn min_immortal() -> Self {
                Self::new(1)
            }
        }
    };
}

impl_ref_count!(AtomicU8, u8);
impl_ref_count!(AtomicU16, u16);
impl_ref_count!(AtomicU32, u32);
impl_ref_count!(AtomicUsize, usize);

#[doc(hidden)]
#[repr(transparent)]
pub struct ArcColdStringInner<A: RefCount> {
    encoded: Encoded<A>,
}

/// A one-word, atomically reference-counted immutable UTF-8 string.
///
/// Strings up to one machine word are stored inline. Longer strings use one
/// allocation containing the reference count, variable-length length, and bytes.
/// If the reference count saturates, the allocation becomes immortal (and is
/// never deallocated) rather than allowing the count to wrap.
///
/// ```
/// use cold_string::ArcColdString;
///
/// let first = ArcColdString::new("a string longer than one machine word");
/// let second = first.clone();
/// assert_eq!(first, second);
/// ```
pub type ArcColdString = ArcColdStringInner<AtomicUsize>;

/// An [`ArcColdString`] with an 8-bit reference count.
///
/// It supports up to 127 live references. Cloning beyond that makes the
/// allocation immortal.
pub type ArcColdString8 = ArcColdStringInner<AtomicU8>;

/// An [`ArcColdString`] with a 16-bit reference count.
///
/// It supports up to 32,767 live references. Cloning beyond that makes the
/// allocation immortal.
pub type ArcColdString16 = ArcColdStringInner<AtomicU16>;

/// An [`ArcColdString`] with a 32-bit reference count.
///
/// It supports up to 2,147,483,647 live references. Cloning beyond that makes
/// the allocation immortal.
pub type ArcColdString32 = ArcColdStringInner<AtomicU32>;

impl<A: RefCount> ArcColdStringInner<A> {
    pub fn from_utf8<B: AsRef<[u8]>>(bytes: B) -> Result<Self, Utf8Error> {
        Ok(Self::new(str::from_utf8(bytes.as_ref())?))
    }

    /// # Safety
    ///
    /// `bytes` must contain valid UTF-8.
    pub unsafe fn from_utf8_unchecked<B: AsRef<[u8]>>(bytes: B) -> Self {
        Self::new(str::from_utf8_unchecked(bytes.as_ref()))
    }

    pub fn new<T: AsRef<str>>(value: T) -> Self {
        let s = value.as_ref();
        Self {
            encoded: Encoded::new(s, A::new()),
        }
    }

    #[rustversion::since(1.61)]
    #[inline]
    pub const fn new_inline_const(s: &str) -> Self {
        Self {
            encoded: Encoded::new_inline_const(s),
        }
    }

    #[inline]
    fn count(&self) -> &A {
        debug_assert!(!self.is_inline());
        unsafe { &(*self.encoded.heap_ptr().as_ptr()).header }
    }

    #[cfg(test)]
    pub(crate) fn encoded_addr(&self) -> usize {
        self.encoded.addr()
    }

    #[inline]
    pub fn is_inline(&self) -> bool {
        self.encoded.is_inline()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.encoded.len()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.encoded.as_bytes()
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: constructors accept only valid UTF-8.
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<A: RefCount> Clone for ArcColdStringInner<A> {
    #[inline]
    fn clone(&self) -> Self {
        if !self.is_inline() {
            self.count().increment();
        }

        Self {
            encoded: self.encoded,
        }
    }
}

impl<A: RefCount> Drop for ArcColdStringInner<A> {
    #[inline]
    fn drop(&mut self) {
        if self.is_inline() {
            return;
        }

        if !self.count().decrement() {
            return;
        }

        fence(Acquire);
        // SAFETY: this was the last reference and the count is synchronized.
        unsafe { self.encoded.deallocate(Global) }
    }
}

impl<A: RefCount> Default for ArcColdStringInner<A> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<A: RefCount> Deref for ArcColdStringInner<A> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<A: RefCount> PartialEq for ArcColdStringInner<A> {
    fn eq(&self, other: &Self) -> bool {
        self.encoded.addr() == other.encoded.addr() || self.as_bytes() == other.as_bytes()
    }
}

impl<A: RefCount> Eq for ArcColdStringInner<A> {}

impl<A: RefCount> Hash for ArcColdStringInner<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<A: RefCount> fmt::Debug for ArcColdStringInner<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<A: RefCount> fmt::Display for ArcColdStringInner<A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<A: RefCount> From<&str> for ArcColdStringInner<A> {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl<A: RefCount> From<String> for ArcColdStringInner<A> {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl<A: RefCount> From<Box<str>> for ArcColdStringInner<A> {
    fn from(s: Box<str>) -> Self {
        Self::new(&s)
    }
}

impl<A: RefCount> From<ArcColdStringInner<A>> for String {
    fn from(s: ArcColdStringInner<A>) -> Self {
        s.as_str().to_owned()
    }
}

impl<A: RefCount> From<ArcColdStringInner<A>> for Cow<'_, str> {
    fn from(s: ArcColdStringInner<A>) -> Self {
        Self::Owned(s.into())
    }
}

impl<'a, A: RefCount> From<&'a ArcColdStringInner<A>> for Cow<'a, str> {
    fn from(s: &'a ArcColdStringInner<A>) -> Self {
        Self::Borrowed(s)
    }
}

impl<'a, A: RefCount> From<Cow<'a, str>> for ArcColdStringInner<A> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new(s)
    }
}

impl<A: RefCount> FromIterator<char> for ArcColdStringInner<A> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<String>())
    }
}

impl<A: RefCount> core::borrow::Borrow<str> for ArcColdStringInner<A> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<A: RefCount> PartialEq<str> for ArcColdStringInner<A> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<A: RefCount> PartialEq<ArcColdStringInner<A>> for str {
    fn eq(&self, other: &ArcColdStringInner<A>) -> bool {
        other == self
    }
}

impl<A: RefCount> PartialEq<&str> for ArcColdStringInner<A> {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl<A: RefCount> PartialEq<ArcColdStringInner<A>> for &str {
    fn eq(&self, other: &ArcColdStringInner<A>) -> bool {
        other == *self
    }
}

impl<A: RefCount> AsRef<str> for ArcColdStringInner<A> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<A: RefCount> AsRef<[u8]> for ArcColdStringInner<A> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<A: RefCount> Ord for ArcColdStringInner<A> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<A: RefCount> PartialOrd for ArcColdStringInner<A> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<A: RefCount> str::FromStr for ArcColdStringInner<A> {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

#[cfg(feature = "serde")]
impl<A: RefCount> serde::Serialize for ArcColdStringInner<A> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de, A: RefCount> serde::Deserialize<'de> for ArcColdStringInner<A> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

unsafe impl<A: RefCount> Send for ArcColdStringInner<A> {}
unsafe impl<A: RefCount> Sync for ArcColdStringInner<A> {}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    macro_rules! each_ref_count {
        ($test:ident) => {
            $test::<AtomicU8>();
            $test::<AtomicU16>();
            $test::<AtomicU32>();
            $test::<AtomicUsize>();
        };
    }

    fn assert_layout<A: RefCount>() {
        assert_eq!(size_of::<ArcColdStringInner<A>>(), size_of::<usize>());
        assert_eq!(
            size_of::<Option<ArcColdStringInner<A>>>(),
            size_of::<ArcColdStringInner<A>>()
        );
        assert_eq!(size_of::<crate::heap::VintStringInner<A>>(), size_of::<A>());
        assert_eq!(
            align_of::<crate::heap::VintStringInner<A>>(),
            align_of::<A>()
        );
    }

    #[test]
    fn layout() {
        each_ref_count!(assert_layout);
    }

    fn assert_inline_and_heap_clone<A: RefCount>() {
        let inline = ArcColdStringInner::<A>::new("tiny");
        let inline_clone = inline.clone();
        assert!(inline.is_inline());
        assert_eq!(inline, inline_clone);

        let heap = ArcColdStringInner::<A>::new("a string longer than one machine word");
        let heap_clone = heap.clone();
        assert!(!heap.is_inline());
        assert_eq!(heap.encoded.addr(), heap_clone.encoded.addr());
        assert_eq!(
            heap.encoded.heap_ptr().as_ptr() as usize
                % core::cmp::max(align_of::<A>(), crate::heap::HEAP_ALIGN),
            0
        );
        assert_eq!(heap.count().refs(), 2);
        drop(heap_clone);
        assert_eq!(heap.count().refs(), 1);
    }

    #[test]
    fn inline_and_heap_clone() {
        each_ref_count!(assert_inline_and_heap_clone);
    }

    fn assert_clones_across_threads<A: RefCount>() {
        let value = ArcColdStringInner::<A>::new("a shared string longer than one machine word");
        let threads: alloc::vec::Vec<_> = (0..8)
            .map(|_| {
                let clone = value.clone();
                std::thread::spawn(move || {
                    assert_eq!(clone, "a shared string longer than one machine word")
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(value.count().refs(), 1);
    }

    #[test]
    fn clones_across_threads() {
        each_ref_count!(assert_clones_across_threads);
    }

    #[cfg(all(loom, target_arch = "x86_64"))]
    fn model_clone_drop<A: RefCount>() {
        loom::model(|| {
            const TEXT: &str = "a shared string longer than one machine word";

            let value = ArcColdStringInner::<A>::new(TEXT);
            let left = value.clone();
            let right = value.clone();

            let left = loom::thread::spawn(move || {
                let clone = left.clone();
                assert_eq!(clone.as_str(), TEXT);
                drop(clone);
                drop(left);
            });
            let right = loom::thread::spawn(move || {
                let clone = right.clone();
                assert_eq!(clone.as_str(), TEXT);
                drop(right);
                drop(clone);
            });

            left.join().unwrap();
            right.join().unwrap();
            assert_eq!(value.count().refs(), 1);
        });
    }

    #[cfg(all(loom, target_arch = "x86_64"))]
    fn model_final_drop<A: RefCount>() {
        loom::model(|| {
            let first =
                ArcColdStringInner::<A>::new("a shared string longer than one machine word");
            let second = first.clone();

            let first = loom::thread::spawn(move || drop(first));
            let second = loom::thread::spawn(move || drop(second));

            first.join().unwrap();
            second.join().unwrap();
        });
    }

    #[cfg(all(loom, target_arch = "x86_64"))]
    fn model_overflow_race<A: RefCount>() {
        loom::model(|| {
            let count = loom::sync::Arc::new(A::near_overflow());
            let expected_refs = count.refs();
            let increment = count.clone();
            let decrement = count.clone();

            let increment = loom::thread::spawn(move || increment.increment());
            let decrement = loom::thread::spawn(move || assert!(!decrement.decrement()));

            increment.join().unwrap();
            decrement.join().unwrap();

            if count.is_immortal() {
                assert!(!count.decrement());
                assert!(count.is_immortal());
            } else {
                assert_eq!(count.refs(), expected_refs);
            }
        });
    }

    #[cfg(all(loom, target_arch = "x86_64"))]
    #[test]
    fn loom_clone_drop() {
        each_ref_count!(model_clone_drop);
        each_ref_count!(model_final_drop);
        each_ref_count!(model_overflow_race);
    }

    #[cfg(not(all(loom, target_arch = "x86_64")))]
    fn assert_refcount_edges<A: RefCount>() {
        let count = A::new();
        assert_eq!(count.refs(), 1);
        count.increment();
        assert_eq!(count.refs(), 2);
        assert!(!count.decrement());
        assert_eq!(count.refs(), 1);
        assert!(count.decrement());
        assert_eq!(count.refs(), 0);

        let count = A::near_overflow();
        count.increment();
        assert!(count.is_immortal());
        assert!(!count.decrement());
        assert!(count.is_immortal());

        let count = A::min_immortal();
        assert!(count.is_immortal());
        assert!(!count.decrement());
        assert!(count.is_immortal());
    }

    #[cfg(not(all(loom, target_arch = "x86_64")))]
    #[test]
    fn refcount_edges() {
        each_ref_count!(assert_refcount_edges);
    }

    #[test]
    fn const_inline_matches_runtime() {
        macro_rules! assert_const_inline {
            ($atomic:ty) => {{
                const VALUE: ArcColdStringInner<$atomic> =
                    ArcColdStringInner::<$atomic>::new_inline_const("cold");
                assert_eq!(VALUE, ArcColdStringInner::<$atomic>::new("cold"));
            }};
        }

        assert_const_inline!(AtomicU8);
        assert_const_inline!(AtomicU16);
        assert_const_inline!(AtomicU32);
        assert_const_inline!(AtomicUsize);
    }

    #[cfg(feature = "serde")]
    fn assert_serde_roundtrip<A: RefCount>() {
        use serde_test::{assert_tokens, Token};

        let value = ArcColdStringInner::<A>::new("a shared string longer than one machine word");
        assert_tokens(
            &value,
            &[Token::Str("a shared string longer than one machine word")],
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrip_shape() {
        each_ref_count!(assert_serde_roundtrip);
    }
}
