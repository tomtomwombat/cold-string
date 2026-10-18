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

use crate::encoded::Encoded;

pub trait RefCount: 'static {
    fn new() -> Self;
    fn increment(&self);
    fn decrement(&self) -> bool;
    fn acquire_fence();

    #[cfg(test)]
    fn refs(&self) -> usize;

    #[cfg(test)]
    fn near_overflow() -> Self;

    #[cfg(test)]
    fn is_immortal(&self) -> bool;

    #[cfg(all(test, not(all(loom, target_arch = "x86_64"))))]
    fn min_immortal() -> Self;
}

#[doc(hidden)]
#[repr(transparent)]
pub struct RefCounted<C: RefCount> {
    pub(crate) encoded: Encoded<C>,
}

impl<C: RefCount> RefCounted<C> {
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
            encoded: Encoded::new(s, C::new()),
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
    pub(crate) fn count(&self) -> &C {
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

impl<C: RefCount> Clone for RefCounted<C> {
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

impl<C: RefCount> Drop for RefCounted<C> {
    #[inline]
    fn drop(&mut self) {
        if self.is_inline() || !self.count().decrement() {
            return;
        }

        C::acquire_fence();
        // SAFETY: this was the last reference and the count is synchronized.
        unsafe { self.encoded.deallocate() }
    }
}

impl<C: RefCount> Default for RefCounted<C> {
    fn default() -> Self {
        Self::new("")
    }
}

impl<C: RefCount> Deref for RefCounted<C> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<C: RefCount> PartialEq for RefCounted<C> {
    fn eq(&self, other: &Self) -> bool {
        let left_addr = self.encoded.addr();
        let right_addr = other.encoded.addr();
        if left_addr == right_addr {
            return true;
        }
        if self.is_inline() || other.is_inline() {
            return false;
        }
        self.encoded.heap_eq(&other.encoded)
    }
}

impl<C: RefCount> Eq for RefCounted<C> {}

impl<C: RefCount> Hash for RefCounted<C> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl<C: RefCount> fmt::Debug for RefCounted<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl<C: RefCount> fmt::Display for RefCounted<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl<C: RefCount> From<&str> for RefCounted<C> {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl<C: RefCount> From<String> for RefCounted<C> {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl<C: RefCount> From<Box<str>> for RefCounted<C> {
    fn from(s: Box<str>) -> Self {
        Self::new(&s)
    }
}

impl<C: RefCount> From<RefCounted<C>> for String {
    fn from(s: RefCounted<C>) -> Self {
        s.as_str().to_owned()
    }
}

impl<C: RefCount> From<RefCounted<C>> for Cow<'_, str> {
    fn from(s: RefCounted<C>) -> Self {
        Self::Owned(s.into())
    }
}

impl<'a, C: RefCount> From<&'a RefCounted<C>> for Cow<'a, str> {
    fn from(s: &'a RefCounted<C>) -> Self {
        Self::Borrowed(s)
    }
}

impl<'a, C: RefCount> From<Cow<'a, str>> for RefCounted<C> {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new(s)
    }
}

impl<C: RefCount> FromIterator<char> for RefCounted<C> {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<String>())
    }
}

impl<C: RefCount> core::borrow::Borrow<str> for RefCounted<C> {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl<C: RefCount> PartialEq<str> for RefCounted<C> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl<C: RefCount> PartialEq<RefCounted<C>> for str {
    fn eq(&self, other: &RefCounted<C>) -> bool {
        other == self
    }
}

impl<C: RefCount> PartialEq<&str> for RefCounted<C> {
    fn eq(&self, other: &&str) -> bool {
        self == *other
    }
}

impl<C: RefCount> PartialEq<RefCounted<C>> for &str {
    fn eq(&self, other: &RefCounted<C>) -> bool {
        other == *self
    }
}

impl<C: RefCount> AsRef<str> for RefCounted<C> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<C: RefCount> AsRef<[u8]> for RefCounted<C> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<C: RefCount> Ord for RefCounted<C> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl<C: RefCount> PartialOrd for RefCounted<C> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: RefCount> str::FromStr for RefCounted<C> {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

#[cfg(feature = "serde")]
impl<C: RefCount> serde::Serialize for RefCounted<C> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de, C: RefCount> serde::Deserialize<'de> for RefCounted<C> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

unsafe impl<C: RefCount + Send + Sync> Send for RefCounted<C> {}
unsafe impl<C: RefCount + Send + Sync> Sync for RefCounted<C> {}
