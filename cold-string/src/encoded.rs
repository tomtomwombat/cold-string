// On pre-1.84 compilers these calls resolve to the `sptr` polyfill imported below.
#![allow(clippy::incompatible_msrv)]

use core::{mem, ptr, ptr::NonNull, slice};

#[rustversion::before(1.84)]
use sptr::Strict;

use crate::heap::{VintStringInner, ALIGN_BITS, HEAP_ALIGN};

pub(crate) const WIDTH: usize = mem::size_of::<usize>();
pub(crate) static WORD_NUL: [u8; WIDTH] = [0u8; WIDTH];

/// The common one-word representation used by both owning string types.
#[repr(transparent)]
pub(crate) struct Encoded<H> {
    ptr: NonNull<VintStringInner<H>>,
}

impl<H> Copy for Encoded<H> {}

impl<H> Clone for Encoded<H> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<H> Encoded<H> {
    const TAG_MASK: usize = usize::from_ne_bytes(0b11000000usize.to_le_bytes());
    const PTR_TAG: usize = usize::from_ne_bytes(0b10000000usize.to_le_bytes());
    pub(crate) const WORD_NUL_MAP: usize = (usize::MAX >> ALIGN_BITS * 2) << ALIGN_BITS;
    const ROT: u32 = if cfg!(target_endian = "little") {
        0
    } else {
        8 * (WIDTH - 1) as u32
    };

    #[inline]
    pub(crate) fn new(s: &str, header: H) -> Self {
        if s.len() <= WIDTH {
            Self::new_inline(s)
        } else {
            Self::from_heap(VintStringInner::allocate(header, s))
        }
    }

    #[inline]
    pub(crate) fn new_inline(s: &str) -> Self {
        debug_assert!(s.len() <= WIDTH);
        if s.as_bytes() == WORD_NUL {
            return Self::new_word_nul();
        }
        let mut buf = [u8::MAX; WIDTH];
        buf[0..s.len()].copy_from_slice(s.as_bytes());
        // SAFETY: short strings contain a non-zero inline tag, while a full-width
        // all-zero string was handled above.
        unsafe { Self::from_inline_buf(buf) }
    }

    #[rustversion::since(1.61)]
    #[inline]
    pub(crate) const fn new_inline_const(s: &str) -> Self {
        if s.len() > WIDTH {
            panic!(
                "Length for `new_inline_const` must be at most `core::mem::size_of::<usize>()`."
            );
        }
        let mut buf = [u8::MAX; WIDTH];
        let mut i = 0;
        while i < s.len() {
            buf[i] = s.as_bytes()[i];
            i += 1;
        }

        if usize::from_ne_bytes(buf) == 0 {
            return Self::new_word_nul();
        }

        // SAFETY: the all-zero representation was handled above.
        unsafe { Self::from_inline_buf(buf) }
    }

    #[inline]
    fn from_heap(ptr: NonNull<VintStringInner<H>>) -> Self {
        let encoded = ptr.as_ptr().map_addr(|addr| {
            debug_assert_eq!(addr % HEAP_ALIGN, 0);
            let rot = 8 - ALIGN_BITS + Self::ROT;
            addr.rotate_left(rot) | Self::PTR_TAG
        });

        // SAFETY: the pointer tag is non-zero and `map_addr` preserves provenance.
        Self {
            ptr: unsafe { NonNull::new_unchecked(encoded) },
        }
    }

    #[inline]
    pub(crate) fn heap_ptr(&self) -> NonNull<VintStringInner<H>> {
        debug_assert!(!self.is_inline());
        let ptr = self.ptr.as_ptr();
        let rot = 8 - ALIGN_BITS + Self::ROT;
        let decoded = ptr.map_addr(|addr| (addr ^ Self::PTR_TAG).rotate_right(rot));
        debug_assert_eq!(decoded.addr() % HEAP_ALIGN, 0);

        // SAFETY: decoding reverses `from_heap`, so the result is non-null and
        // retains the allocation's provenance.
        unsafe { NonNull::new_unchecked(decoded) }
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        if self.is_inline() {
            // SAFETY: inline representations contain their bytes in this word.
            unsafe { self.inline_bytes() }
        } else {
            // SAFETY: heap representations point to a live allocation kept alive
            // by the owner of this encoded word.
            unsafe { VintStringInner::as_bytes(self.heap_ptr()) }
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.as_bytes().len()
    }

    /// The caller must own the heap allocation exclusively.
    #[inline]
    pub(crate) unsafe fn deallocate(&self) {
        debug_assert!(!self.is_inline());
        VintStringInner::deallocate(self.heap_ptr());
    }

    #[inline]
    pub(crate) fn is_inline(&self) -> bool {
        self.addr() & Self::TAG_MASK != Self::PTR_TAG
    }

    #[inline]
    fn is_word_nul(&self) -> bool {
        self.addr() == Self::WORD_NUL_MAP
    }

    #[inline]
    fn inline_len(&self) -> usize {
        debug_assert!(self.is_inline());
        let addr = self.addr();
        let trailing = if cfg!(target_endian = "little") {
            addr.leading_ones()
        } else {
            addr.trailing_ones()
        };
        WIDTH - (trailing as usize >> 3)
    }

    #[inline]
    pub(crate) unsafe fn inline_bytes(&self) -> &[u8] {
        debug_assert!(self.is_inline());
        if self.is_word_nul() {
            return &WORD_NUL;
        }
        let len = self.inline_len();
        let bytes = ptr::addr_of!(self.ptr).cast::<u8>();
        slice::from_raw_parts(bytes, len)
    }

    #[inline]
    pub(crate) fn addr(&self) -> usize {
        self.ptr.as_ptr().addr()
    }

    #[inline]
    const fn new_word_nul() -> Self {
        // SAFETY: `WORD_NUL_MAP` is non-zero.
        unsafe { Self::from_inline_buf(Self::WORD_NUL_MAP.to_ne_bytes()) }
    }

    /// `buf` must not be all zeroes.
    #[inline]
    const unsafe fn from_inline_buf(buf: [u8; WIDTH]) -> Self {
        let addr = usize::from_ne_bytes(buf);
        debug_assert!(addr != 0);
        let ptr = sptr::invalid_mut::<VintStringInner<H>>(addr);
        Self {
            ptr: NonNull::new_unchecked(ptr),
        }
    }
}
