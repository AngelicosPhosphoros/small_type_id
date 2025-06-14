use core::num::NonZeroUsize;

const MAX_HEX_DIGITS: usize = core::mem::size_of::<u32>() * 2;

// Note that its implementation also tested using check_every_u32_hex code using Address Sanitizer
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

    #[allow(clippy::items_after_statements)]
    pub(crate) const fn new(val: u32) -> HexView {
        let len: u32 = if val == 0 {
            1
        } else {
            8 - val.leading_zeros() / 4
        };

        let x = val as u64;
        // 0x1234_ABCD => 0x1234_0000_ABCD;
        let x = ((x & 0xFFFF_0000) << 16) | (x & 0xFFFF);
        // 0x1234_0000_ABCD => 0x0012_0023_00AB_00CD
        let x = ((x & 0xFF00_0000_FF00) << 8) | (x & 0x00FF_0000_00FF);
        // 0x0012_0023_00AB_00CD => 0x0102_0203_0A0B_0C0D
        let x = ((x & 0x00F0_00F0_00F0_00F0) << 4) | (x & 0x000F_000F_000F_000F);
        // Remove leading zeros
        let x = x << ((8 - len) * 8);
        // Add 6 to every byte so we got 1 in larger half of bytes that
        // contain values from 10
        let mask = ((x + 0x0606_0606_0606_0606) & 0x1010_1010_1010_1010) >> 4;
        let mask = mask * 0xFF;
        let decimals = x & !mask;
        let letters = x & mask;
        const ADD_DIGITS: u64 = u64::from_le_bytes([b'0'; 8]);
        const ADD_LETTERS: u64 = u64::from_le_bytes([b'A' - 10; 8]);
        let x = ((decimals + ADD_DIGITS) & !mask) | ((letters + ADD_LETTERS) & mask);

        let buffer = x.to_be_bytes();

        HexView {
            buffer,
            len: NonZeroUsize::new(len as usize).unwrap(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    #[test]
    fn basic_nums() {
        assert_eq!(HexView::new(0).as_str(), "0");
        assert_eq!(HexView::new(u32::MAX).as_str(), "FFFFFFFF");
        assert_eq!(HexView::new(u32::MAX).as_str(), format!("{:X}", u32::MAX));
        assert_eq!(HexView::new(0x0F0F0F0F).as_str(), "F0F0F0F");
        assert_eq!(HexView::new(0xF0F0F0F0).as_str(), "F0F0F0F0");
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
