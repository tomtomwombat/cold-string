use crate::{encoded::Encoded, heap::Allocator};
use core::{fmt, marker::PhantomData, ops::Deref};

#[allow(unused_imports, reason="used in doc comments")]
use crate::ColdString;
/// An arena-allocated string type that can be used to store strings in a memory arena.
/// Short strings are stored inline, same as [`ColdString`], while longer strings are allocated in the arena.
/// Deallocation happens by dropping the arena, which will free all strings allocated in it.
///
/// See [`ColdString`] for method documentation.

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ArenaString<'arena> {
    encoded: Encoded<()>,
    _arena: PhantomData<&'arena str>,
}

unsafe impl Send for ArenaString<'_> {}
unsafe impl Sync for ArenaString<'_> {}

impl<'arena> ArenaString<'arena> {
    /// Creates a new `ArenaString` from a string slice, allocating it in the given arena.
    pub fn new_in<A: Allocator + 'arena>(s: &str, arena: A) -> Self {
        let encoded = Encoded::new_in(s, (), arena);
        Self {
            encoded,
            _arena: PhantomData,
        }
    }

    #[rustversion::since(1.61)]
    #[inline]
    pub const fn new_inline_const(s: &str) -> ArenaString<'static> {
         ArenaString::<'static> {
            encoded: Encoded::new_inline_const(s),
            _arena: PhantomData,
        }
    }

    #[inline]
    fn new_inline(s: &str) -> Self {
        Self {
            encoded: Encoded::new_inline(s),
            _arena: PhantomData,
        }
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

impl Default for ArenaString<'_> {
    fn default() -> Self {
        Self::new_inline("")
    }
}

impl Deref for ArenaString<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for ArenaString<'_> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for ArenaString<'_> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl PartialEq for ArenaString<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.encoded.addr() == other.encoded.addr() || self.as_bytes() == other.as_bytes()
    }
}

impl Eq for ArenaString<'_> {}

impl Ord for ArenaString<'_> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for ArenaString<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for ArenaString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for ArenaString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq<str> for ArenaString<'_> {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<ArenaString<'_>> for str {
    fn eq(&self, other: &ArenaString<'_>) -> bool {
        other.eq(self)
    }
}

impl PartialEq<&str> for ArenaString<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<ArenaString<'_>> for &str {
    fn eq(&self, other: &ArenaString<'_>) -> bool {
        other.eq(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::String;
    use bumpalo::Bump;

    fn alloc_str<'a>(s: &str, arena: &'a Bump) -> ArenaString<'a> {
        ArenaString::new_in(s, arena)
    }

    #[test]
    fn test_arena_alloc() {
        let arena = &Bump::new();
        let s1 = ArenaString::new_in("hello", arena);
        let s2 = {
            // Make sure that this works with the shorter lifetime of x
            let x = String::from("world, in a longer string");
            alloc_str(&x, arena)
        };
        assert!(!s2.is_inline());
        assert_eq!(s1.as_str(), "hello");
        let static_hello = ArenaString::new_inline_const("hello");
        assert_eq!(s1, static_hello);
    }
}