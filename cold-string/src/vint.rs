pub struct VarInt;

impl VarInt {
    pub fn write(mut value: u64, buf: &mut [u8; 10]) -> usize {
        let mut i = 0;
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            buf[i] = byte;
            i += 1;
            if value == 0 {
                break;
            }
        }
        i
    }

    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn read(ptr: *const u8) -> (u64, usize) {
        let mut result = 0u64;
        let mut shift = 0;
        let mut i = 0;
        loop {
            let byte = *ptr.add(i);
            result |= ((byte & 0x7F) as u64) << shift;
            shift += 7;
            i += 1;

            if byte & 0x80 == 0 {
                break;
            }
        }
        (result, i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vint_round_trip() {
        for x in [
            0,
            1,
            42,
            59243,
            5,
            8,
            7,
            63,
            64,
            5892389523,
            (1 << 56) - 1,
            5892389523582389523,
            1 << 56,
            u64::MAX,
        ] {
            let mut b = [0u8; 10];
            let wrote = VarInt::write(x, &mut b);
            assert!(wrote >= 1 && wrote <= 10);
            let ptr = b.as_ptr();
            let (y, read) = unsafe { VarInt::read(ptr) };
            assert_eq!(wrote, read);
            assert_eq!(x, y);
        }
    }
}
