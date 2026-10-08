use alloc::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use core::{
    cmp,
    mem::{align_of, size_of},
    ptr::{self, NonNull},
    slice,
};

use crate::{encoded::WIDTH, vint};

pub(crate) const ALIGN_BITS: u32 = 2;
pub(crate) const HEAP_ALIGN: usize = 1 << ALIGN_BITS;

/// A heap string with an arbitrary fixed-size header followed by
/// `[vint (length - WIDTH)][UTF-8 bytes]`.
#[repr(C)]
pub(crate) struct VintStringInner<H> {
    pub(crate) header: H,
    payload: [u8; 0],
}

impl<H> VintStringInner<H> {
    #[inline]
    fn layout(len: usize, vint_len: usize) -> Layout {
        let size = size_of::<Self>()
            .checked_add(vint_len)
            .and_then(|size| size.checked_add(len))
            .expect("capacity overflow");
        let align = cmp::max(align_of::<Self>(), HEAP_ALIGN);
        Layout::from_size_align(size, align).expect("capacity overflow")
    }

    #[inline]
    unsafe fn payload(ptr: NonNull<Self>) -> *mut u8 {
        ptr::addr_of_mut!((*ptr.as_ptr()).payload).cast()
    }

    #[inline]
    unsafe fn read_len(payload: *const u8) -> (usize, usize) {
        let (stored_len, vint_len) = vint::read(payload);
        (stored_len + WIDTH + 1, vint_len)
    }

    #[inline]
    pub(crate) fn allocate(header: H, s: &str) -> NonNull<Self> {
        assert!(s.len() > WIDTH, "heap string must exceed inline capacity");
        let mut stored_len = s.len() - WIDTH - 1;
        let (size, fist_byte) = vint::write_partial(&mut stored_len);
        let layout = Self::layout(s.len(), size);

        unsafe {
            // SAFETY: `layout` has non-zero size because the vint is at least one byte.
            let raw = alloc(layout).cast::<Self>();
            let ptr = match NonNull::new(raw) {
                Some(ptr) => ptr,
                None => handle_alloc_error(layout),
            };

            ptr::addr_of_mut!((*raw).header).write(header);
            let payload = Self::payload(ptr);
            *payload = fist_byte;
            ptr::copy_nonoverlapping(stored_len.to_le_bytes().as_ptr(), payload.add(1), WIDTH);
            ptr::copy_nonoverlapping(s.as_ptr(), payload.add(size), s.len());
            ptr
        }
    }

    /// The caller must keep the allocation alive and immutable for `'a`.
    #[inline]
    pub(crate) unsafe fn as_bytes<'a>(ptr: NonNull<Self>) -> &'a [u8] {
        let payload = Self::payload(ptr);
        let (len, vint_len) = Self::read_len(payload);
        slice::from_raw_parts(payload.add(vint_len), len)
    }

    #[inline]
    pub(crate) unsafe fn prefix<'a>(ptr: NonNull<Self>) -> &'a [u8] {
        // SAFETY: `WIDTH` bytes are guaranteed to be on the heap
        slice::from_raw_parts(Self::payload(ptr), WIDTH)
    }

    /// The caller must have exclusive ownership of this allocation.
    #[inline]
    pub(crate) unsafe fn deallocate(ptr: NonNull<Self>) {
        let payload = Self::payload(ptr);
        let (len, vint_len) = Self::read_len(payload);
        let layout = Self::layout(len, vint_len);

        ptr::drop_in_place(ptr::addr_of_mut!((*ptr.as_ptr()).header));
        dealloc(ptr.as_ptr().cast(), layout);
    }
}
