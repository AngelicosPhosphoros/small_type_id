pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

const fn murmur_v3(src: &[u8], seed: u32) -> u32 {
    let mut h: u32 = seed;
    let mut i = 0;
    while i + 4 <= src.len() {
        let mut k: u32 = 0;
        let mut j = 0;
        while j < 4 {
            k |= (src[i] as u32) << (8 * j);
            i += 1;
            j += 1;
        }

        h ^= murmur_32_scramble(k);
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xE6546B64);
    }
    if i < src.len() {
        let mut k: u32 = 0;
        let mut j = 0;
        while i < src.len() {
            k |= (src[i] as u32) << (8 * j);
            i += 1;
            j += 1;
        }

        h ^= murmur_32_scramble(k);
    }

    let len = src.len() as u32;
    h ^= len;
    h ^= h >> 16;
    h = u32::wrapping_mul(h, 0x85ebca6b);
    h ^= h >> 13;
    h = u32::wrapping_mul(h, 0xc2b2ae35);
    h ^= h >> 16;

    h
}

const fn murmur_32_scramble(k: u32) -> u32 {
    k.wrapping_mul(0xcc9e2d51)
        .rotate_left(15)
        .wrapping_mul(0x1b873593)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroU32;

    const MY_HASH: NonZeroU32 = NonZeroU32::new(murmur_v3(
        concat!(module_path!(), "::", "MyType").as_bytes(),
        55979,
    ))
    .unwrap();

    #[test]
    fn murmur() {
        assert_eq!(murmur_v3("test".as_bytes(), 0), 0xba6bd213);
        assert_eq!(murmur_v3("test".as_bytes(), 0x9747b28c), 0x704b81dc);
        assert_eq!(murmur_v3("Hello, world!".as_bytes(), 0), 0xc0363e43);
        assert_eq!(
            murmur_v3("Hello, world!".as_bytes(), 0x9747b28c),
            0x24884cba
        );
        assert_eq!(
            murmur_v3("The quick brown fox jumps over the lazy dog".as_bytes(), 0),
            0x2e4ff723
        );
        assert_eq!(
            murmur_v3(
                "The quick brown fox jumps over the lazy dog".as_bytes(),
                0x9747b28c
            ),
            0x2fa826cd
        );
    }
}
