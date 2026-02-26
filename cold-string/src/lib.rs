#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]
#![allow(unstable_name_collisions)]
#![no_std]

extern crate alloc;

#[rustversion::before(1.84)]
use sptr::Strict;

use alloc::{
    alloc::{alloc, dealloc, Layout},
    str::Utf8Error,
    string::String,
};
use core::{
    fmt,
    hash::{Hash, Hasher},
    iter::FromIterator,
    mem,
    ops::Deref,
    ptr, slice, str,
};

mod vint;
use crate::vint::VarInt;

const HEAP_ALIGN: usize = 4;
const WIDTH: usize = mem::size_of::<usize>();

/// Compact representation of immutable UTF-8 strings. Optimized for memory usage and struct packing.
///
/// # Example
/// ```
/// let s = cold_string::ColdString::new("qwerty");
/// assert_eq!(s.as_str(), "qwerty");
/// ```
/// ```
/// use core::mem;
/// use cold_string::ColdString;
///
/// assert_eq!(mem::size_of::<ColdString>(), mem::size_of::<usize>());
/// assert_eq!(mem::align_of::<ColdString>(), 1);
/// assert_eq!(mem::size_of::<(ColdString, u8)>(), mem::size_of::<usize>() + 1);
/// assert_eq!(mem::align_of::<(ColdString, u8)>(), 1);
/// ```
#[repr(packed)]
pub struct ColdString {
    /// The first byte of `encoded` is the "tag" and it determines the type:
    /// - 10xxxxxx: an encoded address for the heap. To decode, 10 is set to 00 and swapped
    ///   with the LSB bits of the tag byte. The address is always a multiple of 4 (`HEAP_ALIGN`).
    /// - 11111xxx: xxx is the length in range 0..=7, followed by length UTF-8 bytes.
    /// - xxxxxxxx (valid UTF-8): 8 UTF-8 bytes.
    encoded: *mut u8,
}

impl ColdString {
    const TAG_MASK: usize = usize::from_ne_bytes(0b11000000usize.to_le_bytes());
    const INLINE_TAG: usize = usize::from_ne_bytes(0b11111000usize.to_le_bytes());
    const PTR_TAG: usize = usize::from_ne_bytes(0b10000000usize.to_le_bytes());
    const LEN_MASK: usize = usize::from_ne_bytes(0b111usize.to_le_bytes());
    const ROT: u32 = if cfg!(target_endian = "little") {
        0
    } else {
        8 * (WIDTH - 1) as u32
    };

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
    /// If the string is shorter than `core::mem::size_of::<usize>()`, then it
    /// will be inlined on the stack.
    pub fn new<T: AsRef<str>>(x: T) -> Self {
        let s = x.as_ref();
        if s.len() <= WIDTH {
            Self::new_inline(s)
        } else {
            Self::new_heap(s)
        }
    }

    #[inline]
    const fn inline_buf(s: &str) -> [u8; WIDTH] {
        debug_assert!(s.len() <= WIDTH);
        let mut buf = [0u8; WIDTH];
        if s.len() < WIDTH {
            let tag =
                (Self::INLINE_TAG | s.len().rotate_left(Self::ROT)).rotate_right(Self::ROT) as u8;
            buf[0] = tag;
        }
        buf
    }

    #[rustversion::attr(since(1.61), const)]
    #[inline]
    fn from_inline_buf(b: [u8; WIDTH]) -> Self {
        let encoded = ptr::null_mut::<u8>().wrapping_add(usize::from_ne_bytes(b));
        Self { encoded }
    }

    #[inline]
    const fn utf8_start(l: usize) -> usize {
        (l < WIDTH) as usize
    }

    #[inline]
    fn new_inline(s: &str) -> Self {
        let mut buf = Self::inline_buf(s);
        let start = Self::utf8_start(s.len());
        buf[start..s.len() + start].copy_from_slice(s.as_bytes());
        Self::from_inline_buf(buf)
    }

    /// Creates a new inline [`ColdString`] from `&'static str` at compile time.
    ///
    /// In a dynamic context you can use the method [`ColdString::new()`].
    ///
    /// # Panics
    /// The string must be less than `core::mem::size_of::<usize>()`. Creating
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
        if s.len() > WIDTH {
            panic!(
                "Length for `new_inline_const` must be less than `core::mem::size_of::<usize>()`."
            );
        }
        let mut buf = Self::inline_buf(s);
        let start = Self::utf8_start(s.len());
        let mut i = 0;
        while i < s.len() {
            buf[i + start] = s.as_bytes()[i];
            i += 1;
        }
        Self::from_inline_buf(buf)
    }

    #[rustversion::attr(since(1.71), const)]
    #[inline]
    unsafe fn ptr(&self) -> *mut u8 {
        ptr::read_unaligned(ptr::addr_of!(self.encoded))
    }

    #[inline]
    fn addr(&self) -> usize {
        unsafe { self.ptr().addr() }
    }

    #[inline]
    fn tag(&self) -> usize {
        self.addr() & Self::TAG_MASK
    }

    /// Returns `true` if the string bytes are inlined.
    #[inline]
    pub fn is_inline(&self) -> bool {
        self.tag() != Self::PTR_TAG
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
            let encoded = ptr.map_addr(|addr| {
                debug_assert!(addr % HEAP_ALIGN == 0);
                let mut addr = addr.rotate_left(6 + Self::ROT);
                addr |= Self::PTR_TAG;
                addr
            });
            Self { encoded }
        }
    }

    #[inline]
    fn heap_ptr(&self) -> *mut u8 {
        debug_assert!(!self.is_inline());
        unsafe {
            self.ptr().map_addr(|mut addr| {
                addr ^= Self::PTR_TAG;
                let addr = addr.rotate_right(6 + Self::ROT);
                debug_assert!(addr % HEAP_ALIGN == 0);
                addr
            })
        }
    }

    #[inline]
    fn inline_len(&self) -> usize {
        let addr = self.addr();
        match addr & Self::INLINE_TAG {
            Self::INLINE_TAG => (addr & Self::LEN_MASK).rotate_right(Self::ROT),
            _ => WIDTH,
        }
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
        // SAFETY: addr_of! avoids &self.ptr (which is UB due to alignment)
        let self_bytes_ptr = ptr::addr_of!(self.encoded) as *const u8;
        let start = Self::utf8_start(len);
        slice::from_raw_parts(self_bytes_ptr.add(start), len)
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
        Self::new_inline("")
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
            true => unsafe {
                Self {
                    encoded: self.ptr(),
                }
            },
            false => Self::new_heap(self.as_str()),
        }
    }
}

impl PartialEq for ColdString {
    fn eq(&self, other: &Self) -> bool {
        match (self.is_inline(), other.is_inline()) {
            (true, true) => unsafe { self.ptr() == other.ptr() },
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
    use core::hash::BuildHasher;
    use hashbrown::hash_map::DefaultHashBuilder;

    #[test]
    fn test_layout() {
        assert_eq!(mem::size_of::<ColdString>(), mem::size_of::<usize>());
        assert_eq!(mem::align_of::<ColdString>(), 1);
        struct Foo {
            _s: ColdString,
            _b: u8,
        }

        assert_eq!(mem::size_of::<Foo>(), mem::size_of::<usize>() + 1);
        assert_eq!(mem::align_of::<Foo>(), 1);
    }

    #[test]
    fn test_default() {
        assert!(ColdString::default().is_empty());
        assert_eq!(ColdString::default().len(), 0);
        assert_eq!(ColdString::default(), "");
        assert_eq!(ColdString::default(), ColdString::new(""));
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
            let cs = ColdString::new(s);
            assert_eq!(s.len() <= mem::size_of::<usize>(), cs.is_inline());
            assert_eq!(cs.len(), s.len());
            assert_eq!(cs.as_bytes(), s.as_bytes());
            assert_eq!(cs.as_str(), s);
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
        }
    }

    #[test]
    fn test_unaligned_placement() {
        for s_content in ["torture", "tor", "tortures", "tort", "torture torture"] {
            let mut buffer = [0u8; 32];
            for offset in 0..8 {
                unsafe {
                    let dst = buffer.as_mut_ptr().add(offset) as *mut ColdString;
                    let s = ColdString::new(s_content);
                    ptr::write_unaligned(dst, s);
                    let recovered = ptr::read_unaligned(dst);
                    assert_eq!(recovered.as_str(), s_content);
                }
            }
        }
    }
}
