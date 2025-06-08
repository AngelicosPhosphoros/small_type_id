use core::num::NonZeroUsize;

const MAX_HEX_DIGITS: usize = core::mem::size_of::<u32>() * 2;

pub(crate) struct HexView {
    buffer: [u8; MAX_HEX_DIGITS],
    // Invariant: less than buffer, more than 1
    len: NonZeroUsize,
}

impl HexView {
    #[inline]
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: Only values in range b'0'..=b'9' and b'A'..=b'F'.
        // It makes it valid ASCII string.
        // len cannot be bigger than MAX_HEX_DIGITS by construction.
        unsafe {
            let part = self.buffer.get_unchecked(..self.len.get());
            str::from_utf8_unchecked(part)
        }
    }

    pub(crate) const fn new(mut val: u32) -> HexView {
        let mut buffer = [0; MAX_HEX_DIGITS];
        let mut pos = 0;
        loop {
            let digit = (val & 0xF) as u8;
            val >>= 4;

            let offset = if digit < 10 { b'0' } else { b'A' - 10 };
            buffer[pos] = digit + offset;
            pos += 1;

            if val == 0 {
                break;
            }
        }
        debug_assert!(pos <= MAX_HEX_DIGITS);
        // Can't use buffer[..pos].reverse() because it is not const stable yet.
        let mut i = 0;
        let mut j = pos - 1;
        while i < j {
            buffer.swap(i, j);
            i += 1;
            j -= 1;
        }

        HexView {
            buffer,
            len: NonZeroUsize::new(pos).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_nums() {
        assert_eq!(HexView::new(0).as_str(), "0");
        assert_eq!(HexView::new(u32::MAX).as_str(), "FFFFFFFF");
        assert_eq!(HexView::new(u32::MAX).as_str(), format!("{:X}", u32::MAX));
        assert_eq!(HexView::new(0x0F0F0F0F).as_str(), "F0F0F0F");
        assert_eq!(HexView::new(0xA).as_str(), "A");
        assert_eq!(HexView::new(0xAB).as_str(), "AB");
        assert_eq!(HexView::new(0xABC).as_str(), "ABC");
        assert_eq!(HexView::new(0xABCD).as_str(), "ABCD");
        assert_eq!(HexView::new(0xABCDEF).as_str(), "ABCDEF");
        assert_eq!(HexView::new(0xABCDEF3).as_str(), "ABCDEF3");
        assert_eq!(HexView::new(0xABCDEF35).as_str(), "ABCDEF35");
        assert_eq!(HexView::new(0x12345678).as_str(), "12345678");
        assert_eq!(HexView::new(0x90ABCDEF).as_str(), "90ABCDEF");
    }

    #[test]
    fn first_100_nums() {
        for i in 0..100 {
            assert_eq!(HexView::new(i).as_str(), format!("{:X}", i));
        }
    }

    #[test]
    fn mult_ten() {
        let mut val = 1u32;
        while let Some(x) = val.checked_mul(10) {
            assert_eq!(HexView::new(x).as_str(), format!("{:X}", x));
            val = x;
        }
    }
}
