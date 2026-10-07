//! Inspired and adapted from <https://www.inkandswitch.com/tangents/bijou64/>.

use crate::encoded::WIDTH;

/// Returns size of vint and first byte.
#[inline]
pub fn write_partial(value: usize) -> (usize, u8) {
    if value < 248 {
        (1, value as u8)
    } else {
        let size = 9 - (value.leading_zeros() >> 3) as usize;
        (size, 246 + size as u8)
    }
}

/// Returns the value and how many bytes were read.
#[allow(unsafe_op_in_unsafe_fn)]
#[inline]
pub unsafe fn read(ptr: *const u8) -> (usize, usize) {
    let b0 = *ptr;
    if b0 < 248 {
        return (b0 as usize, 1);
    }
    // 5+/9+ trailing bytes buffer: unconditional 8-byte read is safe.
    let raw = usize::from_le_bytes(ptr.add(1).cast::<[u8; WIDTH]>().read_unaligned());
    let len = (b0 - 247) as usize;
    // remove junk bytes past `len`
    let shift = ((8 - len) as u32) << 3;
    let val = (raw << shift) >> shift;
    (val as usize, len + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vint_simple() {
        assert_correct(451);
    }

    fn write(value: usize) -> (usize, [u8; WIDTH + 1]) {
        let (len, first_byte) = write_partial(value);
        let mut buf = [0; WIDTH + 1];
        buf[0] = first_byte;
        buf[1..WIDTH + 1].copy_from_slice(&value.to_le_bytes());
        for i in 1 + len..WIDTH + 1 {
            buf[i] = 42;
        }
        (len, buf)
    }

    fn assert_correct(x: usize) {
        let (wrote, b) = write(x);
        assert!((1..=10).contains(&wrote));
        let ptr = b.as_ptr();
        let (y, read) = unsafe { read(ptr) };
        assert_eq!(wrote, read);
        assert_eq!(x, y);
    }

    #[test]
    fn vint_round_trip() {
        #[cfg(not(miri))]
        let num = 20_000_000;

        #[cfg(miri)]
        let num = 1024;

        for x in 0..=num {
            assert_correct(x);
        }
    }

    #[test]
    fn vint_bounds_round_trip() {
        let mut shift = 7;
        while shift < usize::BITS {
            let boundary = 1usize << shift;
            for x in boundary - 1..=boundary + 1 {
                assert_correct(x);
            }
            shift += 7;
        }

        for x in 0..=1000 {
            assert_correct(usize::MAX - x);
        }
    }
}
