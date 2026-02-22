#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
    alloc::{alloc, dealloc, Layout},
    str::Utf8Error,
    string::String,
};
use core::{
    fmt,
    hash::{Hash, Hasher},
    mem,
    ops::Deref,
    ptr::{self, with_exposed_provenance_mut},
    slice, str,
};

mod vint;
use crate::vint::VarInt;

const HEAP_ALIGN: usize = 2;
const WIDTH: usize = mem::size_of::<usize>();

/// Compact representation of immutable UTF-8 strings. Optimized for memory usage and struct packing.
///
/// # Example
/// ```
/// let s = cold_string::ColdString::new("qwerty");
/// assert_eq!(s.as_str(), "qwerty");
/// ```
/// ```
/// use std::mem;
/// use cold_string::ColdString;
///
/// assert_eq!(mem::size_of::<ColdString>(), 8);
/// assert_eq!(mem::align_of::<ColdString>(), 1);
/// assert_eq!(mem::size_of::<(ColdString, u8)>(), 9);
/// assert_eq!(mem::align_of::<(ColdString, u8)>(), 1);
/// ```
#[repr(transparent)]
pub struct ColdString([u8; WIDTH]);

impl ColdString {
    /// Convert a slice of bytes into a [`ColdString`].
    ///
    /// A [`ColdString`] is a contiguous collection of bytes (`u8`s) that is valid [`UTF-8`](https://en.wikipedia.org/wiki/UTF-8).
    /// This method converts from an arbitrary contiguous collection of bytes into a
    /// [`ColdString`], failing if the provided bytes are not `UTF-8`.
    ///
    /// # Examples
    /// ### Valid UTF-8
    /// ```
    /// # use cold_string::ColdString;
    /// let bytes = [240, 159, 166, 128, 240, 159, 146, 175];
    /// let compact = ColdString::from_utf8(&bytes).expect("valid UTF-8");
    ///
    /// assert_eq!(compact, "🦀💯");
    /// ```
    ///
    /// ### Invalid UTF-8
    /// ```
    /// # use cold_string::ColdString;
    /// let bytes = [255, 255, 255];
    /// let result = ColdString::from_utf8(&bytes);
    ///
    /// assert!(result.is_err());
    /// ```
    pub fn from_utf8(v: &[u8]) -> Result<Self, Utf8Error> {
        Ok(Self::new(str::from_utf8(v)?))
    }

    /// Converts a vector of bytes to a [`ColdString`] without checking that the string contains
    /// valid UTF-8.
    ///
    /// See the safe version, [`ColdString::from_utf8`], for more details.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # use cold_string::ColdString;
    /// // some bytes, in a vector
    /// let sparkle_heart = [240, 159, 146, 150];
    ///
    /// let sparkle_heart = unsafe {
    ///     ColdString::from_utf8_unchecked(&sparkle_heart)
    /// };
    ///
    /// assert_eq!("💖", sparkle_heart);
    /// ```
    pub unsafe fn from_utf8_unchecked(v: &[u8]) -> Self {
        Self::new(str::from_utf8_unchecked(v))
    }

    /// Creates a new [`ColdString`] from any type that implements `AsRef<str>`.
    /// If the string is short enough, then it will be inlined on the stack.
    pub fn new<T: AsRef<str>>(x: T) -> Self {
        let s = x.as_ref();
        if s.len() < WIDTH {
            Self::new_inline(s)
        } else {
            Self::new_heap(s)
        }
    }

    #[inline]
    const fn is_inline(&self) -> bool {
        self.0[0] & 1 == 1
    }

    #[inline]
    const fn new_inline(s: &str) -> Self {
        debug_assert!(s.len() < WIDTH);
        let mut buf = [0u8; WIDTH];
        unsafe {
            let dest_ptr = buf.as_mut_ptr().add(1);
            ptr::copy_nonoverlapping(s.as_ptr(), dest_ptr, s.len());
        }
        buf[0] = ((s.len() as u8) << 1) | 1;
        Self(buf)
    }

    #[inline]
    fn new_heap(s: &str) -> Self {
        let len = s.len();
        let mut len_buf = [0u8; 10];
        let vint_len = VarInt::write(len as u64, &mut len_buf);
        let total = vint_len + len;
        let layout = Layout::from_size_align(total, HEAP_ALIGN).unwrap();

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }

            // TODO: can optimize this
            ptr::copy_nonoverlapping(len_buf.as_ptr(), ptr, vint_len);
            ptr::copy_nonoverlapping(s.as_ptr(), ptr.add(vint_len), len);

            let addr = ptr.expose_provenance();
            debug_assert!(addr % 2 == 0);
            Self(addr.to_le_bytes())
        }
    }

    #[inline]
    fn heap_ptr(&self) -> *mut u8 {
        // Can be const in 1.91
        debug_assert!(!self.is_inline());
        let addr = usize::from_le_bytes(self.0);
        debug_assert!(addr % 2 == 0);
        with_exposed_provenance_mut::<u8>(addr)
    }

    #[inline]
    const fn inline_len(&self) -> usize {
        self.0[0] as usize >> 1
    }

    /// Returns the length of this `ColdString`, in bytes, not [`char`]s or
    /// graphemes. In other words, it might not be what a human considers the
    /// length of the string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cold_string::ColdString;
    ///
    /// let a = ColdString::from("foo");
    /// assert_eq!(a.len(), 3);
    ///
    /// let fancy_f = String::from("ƒoo");
    /// assert_eq!(fancy_f.len(), 4);
    /// assert_eq!(fancy_f.chars().count(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        if self.is_inline() {
            self.inline_len()
        } else {
            unsafe {
                let ptr = self.heap_ptr();
                let (len, _) = VarInt::read(ptr);
                len as usize
            }
        }
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline]
    unsafe fn decode_inline(&self) -> &[u8] {
        let len = self.inline_len();
        let ptr = self.0.as_ptr().add(1);
        slice::from_raw_parts(ptr, len)
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline]
    unsafe fn decode_heap(&self) -> &[u8] {
        let ptr = self.heap_ptr();
        let (len, header) = VarInt::read(ptr);
        let data = ptr.add(header);
        slice::from_raw_parts(data, len as usize)
    }

    /// Returns a byte slice of this `ColdString`'s contents.
    ///
    /// The inverse of this method is [`from_utf8`].
    ///
    /// [`from_utf8`]: String::from_utf8
    ///
    /// # Examples
    ///
    /// ```
    /// let s = cold_string::ColdString::from("hello");
    ///
    /// assert_eq!(&[104, 101, 108, 108, 111], s.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        match self.is_inline() {
            true => unsafe { self.decode_inline() },
            false => unsafe { self.decode_heap() },
        }
    }

    /// Returns a string slice containing the entire [`ColdString`].
    ///
    /// # Examples
    /// ```
    /// let s = cold_string::ColdString::new("hello");
    ///
    /// assert_eq!(s.as_str(), "hello");
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl Deref for ColdString {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Drop for ColdString {
    fn drop(&mut self) {
        if !self.is_inline() {
            unsafe {
                let ptr = self.heap_ptr();
                let (len, header) = VarInt::read(ptr);
                let total = header + len as usize;
                let layout = Layout::from_size_align(total, HEAP_ALIGN).unwrap();
                dealloc(ptr, layout);
            }
        }
    }
}

impl Clone for ColdString {
    fn clone(&self) -> Self {
        match self.is_inline() {
            true => Self(self.0),
            false => Self::new_heap(self.as_str()),
        }
    }
}

impl PartialEq for ColdString {
    fn eq(&self, other: &Self) -> bool {
        match (self.is_inline(), other.is_inline()) {
            (true, true) => self.0 == other.0,
            (false, false) => unsafe { self.decode_heap() == other.decode_heap() },
            _ => false,
        }
    }
}

impl Eq for ColdString {}

impl Hash for ColdString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl fmt::Debug for ColdString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Display for ColdString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl From<&str> for ColdString {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ColdString {
    fn from(s: String) -> Self {
        Self::new(&s)
    }
}

impl FromIterator<char> for ColdString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        let s: String = iter.into_iter().collect();
        ColdString::new(&s)
    }
}

unsafe impl Send for ColdString {}
unsafe impl Sync for ColdString {}

impl core::borrow::Borrow<str> for ColdString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<str> for ColdString {
    fn eq(&self, other: &str) -> bool {
        if self.is_inline() {
            unsafe { self.decode_inline() == other.as_bytes() }
        } else {
            unsafe { self.decode_heap() == other.as_bytes() }
        }
    }
}

impl PartialEq<ColdString> for str {
    fn eq(&self, other: &ColdString) -> bool {
        other.eq(self)
    }
}

impl PartialEq<&str> for ColdString {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<ColdString> for &str {
    fn eq(&self, other: &ColdString) -> bool {
        other.eq(*self)
    }
}

impl AsRef<str> for ColdString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for ColdString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for ColdString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ColdString {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(ColdString::new(&s))
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_serde_cold_string_inline() {
        let cs = ColdString::new("ferris");
        assert_tokens(&cs, &[Token::Str("ferris")]);
    }

    #[test]
    fn test_serde_cold_string_heap() {
        let long_str = "This is a significantly longer string for heap testing";
        let cs = ColdString::new(long_str);
        assert_tokens(&cs, &[Token::Str(long_str)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout() {
        assert_eq!(mem::size_of::<ColdString>(), 8);
        assert_eq!(mem::align_of::<ColdString>(), 1);
        struct Foo {
            _s: ColdString,
            _b: u8,
        }

        assert_eq!(mem::size_of::<Foo>(), 9);
        assert_eq!(mem::align_of::<Foo>(), 1);
    }

    #[test]
    fn it_works() {
        for s in ["test", "", "1234567", "longer test"] {
            let cs = ColdString::new(s);
            assert_eq!(cs.as_str(), s);
            assert_eq!(cs.len(), s.len());
            assert_eq!(cs.len() < 8, cs.is_inline());
            assert_eq!(cs.clone(), cs);
            #[cfg(feature = "std")]
            {
                use std::hash::{BuildHasher, RandomState};
                let bh = RandomState::new();
                assert_eq!(bh.hash_one(&cs), bh.hash_one(&cs.clone()));
            }
            assert_eq!(cs, s);
            assert_eq!(s, cs);
            assert_eq!(cs, *s);
            assert_eq!(*s, cs);
        }
    }
}
