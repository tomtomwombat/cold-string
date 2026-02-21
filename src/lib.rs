#![allow(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{
    alloc::{alloc, dealloc, Layout},
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

#[repr(transparent)]
pub struct ColdString([u8; WIDTH]);

impl ColdString {
    pub fn new(s: &str) -> Self {
        if s.len() < WIDTH {
            Self::new_inline(s)
        } else {
            Self::new_heap(s)
        }
    }

    #[inline]
    fn is_inline(&self) -> bool {
        self.0[WIDTH - 1] & 1 == 1
    }

    #[inline]
    fn new_inline(s: &str) -> Self {
        debug_assert!(s.len() < WIDTH);
        let mut buf = [0u8; WIDTH];
        buf[..s.len()].copy_from_slice(s.as_bytes());
        buf[WIDTH - 1] = ((s.len() as u8) << 1) | 1;
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

            Self(ptr.addr().to_le_bytes())
        }
    }

    #[inline]
    fn heap_ptr(&self) -> *mut u8 {
        debug_assert!(!self.is_inline());
        with_exposed_provenance_mut::<u8>(usize::from_le_bytes(self.0))
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.is_inline() {
            self.0[WIDTH - 1] as usize >> 1
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
    unsafe fn decode_heap(&self) -> &[u8] {
        let ptr = self.heap_ptr();
        let (len, header) = VarInt::read(ptr);
        let data = ptr.add(header);
        slice::from_raw_parts(data, len as usize)
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        if self.is_inline() {
            let len = self.len();
            unsafe {
                let ptr = self.0.as_ptr();
                let slice = slice::from_raw_parts(ptr, len);
                str::from_utf8_unchecked(slice)
            }
        } else {
            unsafe { str::from_utf8_unchecked(self.decode_heap()) }
        }
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
        ColdString::new(self.as_str())
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
        }
    }
}
