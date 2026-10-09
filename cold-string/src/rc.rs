use core::cell::Cell;

use crate::shared::{RefCount, RefCounted};

macro_rules! impl_ref_count {
    ($cell:ty, $int:ty) => {
        impl RefCount for $cell {
            #[inline]
            fn new() -> Self {
                Cell::new(1)
            }

            #[inline]
            fn increment(&self) {
                self.set(self.get().saturating_add(1));
            }

            #[inline]
            fn decrement(&self) -> bool {
                let old = self.get();
                if old == <$int>::MAX {
                    return false;
                }
                self.set(old - 1);
                old == 1
            }

            #[inline]
            fn acquire_fence() {}

            #[cfg(test)]
            fn refs(&self) -> usize {
                self.get() as usize
            }

            #[cfg(test)]
            fn near_overflow() -> Self {
                Self::new(<$int>::MAX - 1)
            }

            #[cfg(test)]
            fn is_immortal(&self) -> bool {
                self.get() == <$int>::MAX
            }

            #[cfg(all(test, not(all(loom, target_arch = "x86_64"))))]
            fn min_immortal() -> Self {
                Cell::new(<$int>::MAX)
            }
        }
    };
}

impl_ref_count!(Cell<u8>, u8);
impl_ref_count!(Cell<u16>, u16);
impl_ref_count!(Cell<u32>, u32);
impl_ref_count!(Cell<usize>, usize);

/// A one-word, non-atomically reference-counted immutable UTF-8 string.
///
/// Strings up to one machine word are stored inline. Longer strings use one
/// allocation containing the reference count, variable-length length, and bytes.
/// If the reference count saturates, the allocation becomes immortal (and is
/// never deallocated) rather than allowing the count to wrap. This type is not
/// thread-safe and is neither `Send` nor `Sync`.
pub type RcColdString = RefCounted<Cell<usize>>;

/// An [`RcColdString`] with an 8-bit reference count.
///
/// It supports up to 254 live references. Cloning beyond that makes the
/// allocation immortal.
pub type RcColdString8 = RefCounted<Cell<u8>>;

/// An [`RcColdString`] with a 16-bit reference count.
///
/// It supports up to 65,534 live references. Cloning beyond that makes the
/// allocation immortal.
pub type RcColdString16 = RefCounted<Cell<u16>>;

/// An [`RcColdString`] with a 32-bit reference count.
///
/// It supports up to 4,294,967,294 live references. Cloning beyond that makes
/// the allocation immortal.
pub type RcColdString32 = RefCounted<Cell<u32>>;
