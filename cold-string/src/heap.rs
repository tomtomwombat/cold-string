use alloc::alloc::{handle_alloc_error, Layout};
use core::{
    cmp,
    mem::{align_of, size_of},
    ptr::{self, NonNull},
    slice,
};

#[cfg(not(feature = "nightly"))]
pub use allocator_api2::alloc::{Allocator, Global};

#[cfg(feature = "nightly")]
pub use alloc::alloc::{Allocator, Global};

use crate::{encoded::WIDTH, vint::VarInt};

pub(crate) const HEAP_ALIGN: usize = 4;

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
        let (stored_len, vint_len) = VarInt::read(payload);
        (stored_len + WIDTH, vint_len)
    }

    /// The outer type must store a reference to the allocator to handle deallocation, if needed.
    #[inline]
    pub(crate) fn allocate<A: Allocator>(header: H, s: &str, allocator: A) -> NonNull<Self> {
        assert!(s.len() > WIDTH, "heap string must exceed inline capacity");
        let (vint_len, len_buf) = VarInt::write((s.len() - WIDTH) as u64);
        let layout = Self::layout(s.len(), vint_len);

        unsafe {
            // SAFETY: `layout` has non-zero size because the vint is at least one byte.
            let ptr = match allocator.allocate(layout) {
                Ok(ptr) => ptr.cast::<Self>(),
                Err(_) => handle_alloc_error(layout),
            };
            let raw = ptr.as_ptr();
            ptr::addr_of_mut!((*raw).header).write(header);
            let payload = Self::payload(ptr);
            ptr::copy_nonoverlapping(len_buf.as_ptr(), payload, vint_len);
            ptr::copy_nonoverlapping(s.as_ptr(), payload.add(vint_len), s.len());
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

    /// SAFETY: The caller must have exclusive ownership of this allocation,
    /// and the allocator must be the same one used to allocate it.
    #[inline]
    pub(crate) unsafe fn deallocate<A: Allocator>(ptr: NonNull<Self>, allocator: A) {
        let payload = Self::payload(ptr);
        let (len, vint_len) = Self::read_len(payload);
        let layout = Self::layout(len, vint_len);

        ptr::drop_in_place(ptr::addr_of_mut!((*ptr.as_ptr()).header));
        allocator.deallocate(ptr.cast(), layout);
    }
}
