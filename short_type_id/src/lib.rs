#![deny(unsafe_op_in_unsafe_fn)]

use core::num::NonZeroU32;

mod implementation;

pub use implementation::private;

pub trait HasTypeId: 'static {
    const TYPE_ID: NonZeroU32;
}

pub struct TypeEntry {
    pub type_id: NonZeroU32,
    #[cfg(feature = "debug_type_name")]
    pub type_name: &'static str,
}

pub fn iter_registered_entries() -> impl Iterator<Item = TypeEntry> {
    implementation::iter_registered_entries()
}
