//! Inspired and adapted from <https://www.inkandswitch.com/tangents/bijou64/>.

use crate::encoded::WIDTH;

/// Returns size of vint and first byte. `value` is written after. The size is used for the allocation.
#[inline]
pub fn write_partial(value: &mut usize) -> (usize, u8) {
    if *value < 248 {
        (1, *value as u8)
    } else {
        *value -= 247;
        let size = WIDTH + 1 - (value.leading_zeros() >> 3) as usize;
        (size, (246 + size) as u8)
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
    // 6+/10+ trailing bytes buffer: unconditional 8-byte read is safe.
    let raw = usize::from_le_bytes(ptr.add(1).cast::<[u8; WIDTH]>().read_unaligned());
    // remove junk bytes past `len`
    let size = (b0 - 247) as usize;
    let shift = (WIDTH - size) as u32 * 8;
    let val = (raw << shift) >> shift;
    (val + 247, size + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vint_simple() {
        assert_correct(451);
    }

    #[test]
    fn vint_edges() {
        for size in [0, 247, 248, 502, 503, 65782, 65783, 16_777_462] {
            assert_correct(size);
        }
    
        for (val, expected) in [(0, 1), (247, 1), (248, 2), (503, 3)] {
            let (wrote, buf) = write(val);
            assert_eq!(wrote, expected, "Value {} took {} bytes instead of {}", val, wrote, expected);
            let (read_val, read_len) = unsafe { read(buf.as_ptr()) };
            assert_eq!(val, read_val);
            assert_eq!(expected, read_len);
        }
    }

    fn write(mut value: usize) -> (usize, [u8; WIDTH + 1]) {
        let (len, first_byte) = write_partial(&mut value);
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
