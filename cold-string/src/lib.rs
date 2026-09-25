#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]
#![allow(unknown_lints, unexpected_cfgs)]
#![allow(unstable_name_collisions)]
#![no_std]

#![cfg_attr(feature = "nightly", feature(allocator_api))]

extern crate alloc;

#[cfg(test)]
extern crate std;

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
use crate::heap::Global;

#[cfg(test)]
use core::{mem, ptr};

mod arc;
mod arena;
mod encoded;
mod heap;
mod vint;

pub use crate::arc::ArcColdString;
pub use crate::arc::ArcColdString16;
pub use crate::arc::ArcColdString32;
pub use crate::arc::ArcColdString8;
pub use crate::arena::ArenaString;
use crate::encoded::Encoded;

#[cfg(feature = "rkyv")]
mod rkyv;

/// Compact representation of immutable UTF-8 strings. Optimized for memory usage and struct packing.
///
/// # Example
/// ```
/// let s = cold_string::ColdString::new("qwerty");
/// assert_eq!(s.as_str(), "qwerty");
/// ```
/// ```
/// use core::mem::size_of;
/// use cold_string::ColdString;
///
/// assert_eq!(size_of::<ColdString>(), size_of::<usize>());
/// assert_eq!(size_of::<Option<ColdString>>(), size_of::<ColdString>());
/// ```
#[repr(transparent)]
pub struct ColdString {
    /// The first byte of `encoded` is the "tag" and it determines the type:
    /// - 10xxxxxx: an encoded address for the heap. To decode, 10 is set to 00 and swapped
    ///   with the LSB bits of the tag byte. The address is always a multiple of 4 (`HEAP_ALIGN`).
    /// - 11111xxx: xxx is the length in range 0..=7, followed by length UTF-8 bytes.
    /// - xxxxxxxx (valid UTF-8): 8 UTF-8 bytes.
    ///
    /// The exception is if `encoded` is `usize::MAX`, which represents one word of NUL bytes.
    encoded: Encoded<()>,
}

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
    pub fn from_utf8<B: AsRef<[u8]>>(v: B) -> Result<Self, Utf8Error> {
        Ok(Self::new(str::from_utf8(v.as_ref())?))
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
    ///
    /// # Safety
    ///
    /// `v` must contain valid UTF-8.
    pub unsafe fn from_utf8_unchecked<B: AsRef<[u8]>>(v: B) -> Self {
        Self::new(str::from_utf8_unchecked(v.as_ref()))
    }

    /// Creates a new [`ColdString`] from any type that implements `AsRef<str>`.
    /// If the string is at most `core::mem::size_of::<usize>()` bytes, then it
    /// will be inlined on the stack.
    pub fn new<T: AsRef<str>>(x: T) -> Self {
        let s = x.as_ref();
        Self {
            encoded: Encoded::new(s, ()),
        }
    }

    /// Creates a new inline [`ColdString`] from `&'static str` at compile time.
    ///
    /// In a dynamic context you can use the method [`ColdString::new()`].
    ///
    /// # Panics
    /// The string must be at most `core::mem::size_of::<usize>()`. Creating
    /// a [`ColdString`] larger than that is not supported.
    ///
    ///
    /// # Examples
    /// ```
    /// use cold_string::ColdString;
    ///
    /// const DEFAULT_NAME: ColdString = ColdString::new_inline_const("cold");
    /// ```
    #[rustversion::since(1.61)]
    #[inline]
    pub const fn new_inline_const(s: &str) -> Self {
        Self {
            encoded: Encoded::new_inline_const(s),
        }
    }

    /// Returns `true` if the string bytes are inlined.
    #[inline]
    pub fn is_inline(&self) -> bool {
        self.encoded.is_inline()
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
        self.encoded.len()
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
        self.encoded.as_bytes()
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

    /// Returns `true` if this `ColdString` has a length of zero, and `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = cold_string::ColdString::new("");
    /// assert!(v.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ColdString {
    fn default() -> Self {
        Self::new("")
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
            // SAFETY: a non-inline `ColdString` uniquely owns its allocation.
            unsafe { self.encoded.deallocate(Global) }
        }
    }
}

impl Clone for ColdString {
    fn clone(&self) -> Self {
        if self.is_inline() {
            Self {
                encoded: self.encoded,
            }
        } else {
            Self::new(self.as_str())
        }
    }
}

impl PartialEq for ColdString {
    fn eq(&self, other: &Self) -> bool {
        self.encoded.addr() == other.encoded.addr() || self.as_bytes() == other.as_bytes()
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
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ColdString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
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

impl From<ColdString> for String {
    fn from(s: ColdString) -> Self {
        s.as_str().to_owned()
    }
}

impl From<ColdString> for Cow<'_, str> {
    #[inline]
    fn from(s: ColdString) -> Self {
        Self::Owned(s.into())
    }
}

impl<'a> From<&'a ColdString> for Cow<'a, str> {
    #[inline]
    fn from(s: &'a ColdString) -> Self {
        Self::Borrowed(s)
    }
}

impl<'a> From<Cow<'a, str>> for ColdString {
    fn from(cow: Cow<'a, str>) -> Self {
        Self::new(cow)
    }
}

impl From<Box<str>> for ColdString {
    #[inline]
    #[track_caller]
    fn from(b: Box<str>) -> Self {
        Self::new(&b)
    }
}

impl FromIterator<char> for ColdString {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect::<String>())
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
        self.as_str() == other
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

impl Ord for ColdString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for ColdString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl alloc::str::FromStr for ColdString {
    type Err = core::convert::Infallible;
    fn from_str(s: &str) -> Result<ColdString, Self::Err> {
        Ok(ColdString::new(s))
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

#[cfg(test)]
trait TestString:
    Clone + Default + fmt::Debug + Eq + Hash + PartialEq<str> + for<'a> PartialEq<&'a str>
{
    fn new(s: &str) -> Self;
    fn from_utf8(bytes: &[u8]) -> Result<Self, Utf8Error>;
    fn new_inline(s: &str) -> Self;
    fn is_inline(&self) -> bool;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
    fn as_bytes(&self) -> &[u8];
    fn as_str(&self) -> &str;
    fn encoded_addr(&self) -> usize;
}

#[cfg(test)]
impl TestString for ColdString {
    fn new(s: &str) -> Self {
        Self::new(s)
    }

    fn from_utf8(bytes: &[u8]) -> Result<Self, Utf8Error> {
        Self::from_utf8(bytes)
    }

    fn new_inline(s: &str) -> Self {
        Self::new_inline_const(s)
    }

    fn is_inline(&self) -> bool {
        self.is_inline()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    fn as_str(&self) -> &str {
        self.as_str()
    }

    fn encoded_addr(&self) -> usize {
        self.encoded.addr()
    }
}

#[cfg(test)]
impl<A: arc::RefCount> TestString for arc::ArcColdStringInner<A> {
    fn new(s: &str) -> Self {
        Self::new(s)
    }

    fn from_utf8(bytes: &[u8]) -> Result<Self, Utf8Error> {
        Self::from_utf8(bytes)
    }

    fn new_inline(s: &str) -> Self {
        Self::new_inline_const(s)
    }

    fn is_inline(&self) -> bool {
        self.is_inline()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn as_bytes(&self) -> &[u8] {
        self.as_bytes()
    }

    fn as_str(&self) -> &str {
        self.as_str()
    }

    fn encoded_addr(&self) -> usize {
        self.encoded_addr()
    }
}

#[cfg(test)]
macro_rules! each_string {
    ($test:ident $(, $arg:expr)*) => {
        $test::<ColdString>($($arg),*);
        $test::<ArcColdString>($($arg),*);
        $test::<ArcColdString8>($($arg),*);
        $test::<ArcColdString16>($($arg),*);
        $test::<ArcColdString32>($($arg),*);
    };
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    fn assert_serde<T>(s: &'static str)
    where
        T: TestString + serde::Serialize + for<'de> serde::Deserialize<'de>,
    {
        assert_tokens(&T::new(s), &[Token::Str(s)]);
    }

    #[test]
    fn test_serde_cold_string_inline() {
        each_string!(assert_serde, "ferris");
    }

    #[test]
    fn test_serde_cold_string_heap() {
        let long_str = "This is a significantly longer string for heap testing";
        each_string!(assert_serde, long_str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::hash::BuildHasher;
    use hashbrown::hash_map::DefaultHashBuilder;

    fn assert_layout<T: TestString>() {
        assert_eq!(mem::size_of::<T>(), mem::size_of::<usize>());
        assert_eq!(mem::size_of::<Option<T>>(), mem::size_of::<T>());
    }

    #[test]
    fn test_layout() {
        each_string!(assert_layout);
    }

    fn assert_default<T: TestString>() {
        assert!(T::default().is_empty());
        assert_eq!(T::default().len(), 0);
        assert_eq!(T::default(), "");
        assert_eq!(T::default(), T::new(""));
    }

    #[test]
    fn test_default() {
        each_string!(assert_default);
    }

    fn assert_utf8_validation<T: TestString>() {
        for valid in ["", "🦀", "valid UTF-8 🦀 longer than one word"] {
            assert_eq!(T::from_utf8(valid.as_bytes()).unwrap().as_str(), valid);
        }

        for invalid in [
            &[0x80][..],
            &[0xff][..],
            &[0xc0, 0x80][..],
            &[0xe2, 0x82][..],
        ] {
            assert!(T::from_utf8(invalid).is_err());
        }
    }

    #[test]
    fn test_utf8_validation() {
        each_string!(assert_utf8_validation);
    }

    fn assert_correct<T: TestString>(s: &str)
    where
        str: PartialEq<T>,
        for<'a> &'a str: PartialEq<T>,
    {
        let cs = T::new(s);
        assert_eq!(s.len() <= mem::size_of::<usize>(), cs.is_inline());
        assert_eq!(cs.len(), s.len());
        assert_eq!(cs.as_bytes(), s.as_bytes());
        assert_eq!(cs.as_str().as_bytes(), s.as_bytes());
        assert_eq!(cs.clone(), cs);
        let bh = DefaultHashBuilder::new();
        let mut hasher1 = bh.build_hasher();
        cs.hash(&mut hasher1);
        let mut hasher2 = bh.build_hasher();
        cs.clone().hash(&mut hasher2);
        assert_eq!(hasher1.finish(), hasher2.finish());
        assert_eq!(cs, s);
        assert_eq!(s, cs);
        assert_eq!(cs, *s);
        assert_eq!(*s, cs);
        let opt_s = Some(cs.clone());
        assert_eq!(opt_s, Some(T::new(s)));
        assert!(opt_s.is_some());
    }

    #[test]
    fn it_works() {
        for s in [
            "1",
            "12",
            "123",
            "1234",
            "12345",
            "123456",
            "1234567",
            "12345678",
            "123456789",
            str::from_utf8(&[240, 159, 146, 150]).unwrap(),
            "✅",
            "❤️",
            "🦀💯",
            "🦀",
            "💯",
            "abcd",
            "test",
            "",
            "\0",
            "\0\0",
            "\0\0\0",
            "\0\0\0\0",
            "\0\0\0\0\0\0\0",
            "\0\0\0\0\0\0\0\0",
            "1234567",
            "12345678",
            "longer test",
            str::from_utf8(&[103, 39, 240, 145, 167, 156, 194, 165]).unwrap(),
            "AaAa0 ® ",
            str::from_utf8(&[240, 158, 186, 128, 240, 145, 143, 151]).unwrap(),
        ] {
            each_string!(assert_correct, s);
        }
    }

    fn char_from_leading_byte(b: u8) -> Option<char> {
        match b {
            0x00..=0x7F => Some(b as char),
            0xC2..=0xDF => str::from_utf8(&[b, 0x91]).unwrap().chars().next(),
            0xE0 => str::from_utf8(&[b, 0xA0, 0x91]).unwrap().chars().next(),
            0xE1..=0xEC | 0xEE..=0xEF => str::from_utf8(&[b, 0x91, 0xA5]).unwrap().chars().next(),
            0xED => str::from_utf8(&[b, 0x80, 0x91]).unwrap().chars().next(),
            0xF0 => str::from_utf8(&[b, 0x90, 0x91, 0xA5])
                .unwrap()
                .chars()
                .next(),
            0xF1..=0xF3 => str::from_utf8(&[b, 0x91, 0xA5, 0x82])
                .unwrap()
                .chars()
                .next(),
            0xF4 => str::from_utf8(&[b, 0x80, 0x91, 0x82])
                .unwrap()
                .chars()
                .next(),
            _ => None,
        }
    }

    #[test]
    fn test_edges() {
        let width = mem::size_of::<usize>();
        for len in [width - 1, width, width + 1] {
            for first_byte in 0u8..=255 {
                let first_char = match char_from_leading_byte(first_byte) {
                    Some(c) => c,
                    None => continue,
                };

                let mut s = String::with_capacity(len);
                s.push(first_char);

                while s.len() < len {
                    let c = core::char::from_digit((len - s.len()) as u32, 10).unwrap();
                    s.push(c);
                }

                each_string!(assert_correct, &s);
            }
        }
    }

    fn assert_unaligned_placement<T: TestString>() {
        for s_content in ["torture", "tor", "tortures", "tort", "torture torture"] {
            let mut buffer = [0u8; 32];
            for offset in 0..8 {
                unsafe {
                    let dst = buffer.as_mut_ptr().add(offset).cast::<T>();
                    let s = T::new(s_content);
                    ptr::write_unaligned(dst, s);
                    let recovered = ptr::read_unaligned(dst);
                    assert_eq!(recovered.as_str(), s_content);
                }
            }
        }
    }

    #[test]
    fn test_unaligned_placement() {
        each_string!(assert_unaligned_placement);
    }

    #[test]
    fn ensure_zero_repr() {
        assert!(str::from_utf8(&Encoded::<()>::WORD_NUL_MAP.to_ne_bytes()).is_err());
    }

    fn assert_const_word_nul<T: TestString>() {
        let nul = str::from_utf8(&encoded::WORD_NUL).unwrap();
        let const_value = T::new_inline(nul);
        let non_const = T::new(nul);
        let cloned = non_const.clone();
        assert_eq!(const_value.encoded_addr(), non_const.encoded_addr());
        assert_eq!(const_value.encoded_addr(), cloned.encoded_addr());
        // The sentinel returns a slice into the shared word-sized NUL array.
        assert_eq!(
            &const_value.as_str().as_bytes()[0] as *const u8,
            (&encoded::WORD_NUL) as *const u8
        );
    }

    #[test]
    fn test_const_word_nul() {
        each_string!(assert_const_word_nul);
    }
}
