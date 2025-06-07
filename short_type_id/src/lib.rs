#![deny(unsafe_op_in_unsafe_fn)]
#![no_std]

use core::num::NonZeroU32;

mod implementation;

pub use implementation::private;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub(crate) NonZeroU32);

pub trait HasTypeId: 'static {
    const TYPE_ID: TypeId;
}

#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct TypeEntry {
    pub type_id: TypeId,
    #[cfg(feature = "debug_type_name")]
    pub type_name: &'static str,
}

pub fn iter_registered_entries() -> impl Iterator<Item = TypeEntry> {
    implementation::iter_registered_entries()
}

#[derive(Debug, Clone, Copy)]
pub struct ErrorFromZeroBytes {}

impl TypeId {
    pub const fn from_user_code(code: NonZeroU32) -> Self {
        assert!(
            code.get() & 0x8000_0000 == 0x8000_0000,
            "User provided codes must set most significant byte to distinguish it from derived ones.",
        );
        Self(code)
    }

    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    #[cfg(not(target_pointer_width = "16"))]
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0.get() as _
    }

    #[inline]
    pub const fn to_bytes(self) -> [u8; 4] {
        self.0.get().to_le_bytes()
    }

    #[inline]
    pub const fn from_bytes(bytes: [u8; 4]) -> Result<Self, ErrorFromZeroBytes> {
        let val = u32::from_le_bytes(bytes);
        if let Some(x) = NonZeroU32::new(val) {
            Ok(Self(x))
        } else {
            Err(ErrorFromZeroBytes {})
        }
    }
}

impl core::fmt::Debug for TypeId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self, f)
    }
}

impl core::fmt::Display for TypeId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut buffer = itoa::Buffer::new();
        let s = buffer.format(self.as_u32());
        f.write_str(s)
    }
}

impl core::fmt::Display for ErrorFromZeroBytes {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ErrorFromZeroBytes")
    }
}

impl core::error::Error for ErrorFromZeroBytes {}
