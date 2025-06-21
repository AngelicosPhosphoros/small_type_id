//! # THIS IS ALPHA RELEASE PLEASE DO NOT USE
//! This crate provides 3 things:
//!
//! 1. [`TypeId`] type that is used to identify type.
//! 2. [`HasTypeId`] trait with associated constant [`TYPE_ID`][HasTypeId::TYPE_ID].
//! 3. [`HasTypeId`](derive.HasTypeId.html) derive macro for implementing `HasTypeId` for types.
//!
//! `TypeId` has several guarantees about its layout and those guarantees would be held even in major semver updates:
//!
//! 1. `TYPE_ID` is a constant.
//! 2. Size is 32 bit.
//! 3. `TypeId` cannot be zero which allows [niche optimizations][1] (`size_of::<Option<TypeId>>()` is 4 bytes).
//! 4. Most significant bit (_MSB_) is guaranteed to be zero:
//!    * Allows users to use this bit to distinguish with some other kind of id in a union (e.g. Runtime types from some scripting engine).
//!
//! Further guarantees can change with versions of this crate.
//!
//! `TypeId` doesn't depends on contents of the type so it would remain same if it changes.
//!
//! `TypeId` can change on following changes:
//!     * `small_type_id` crate version changes
//!     * placement of the type changes (e.g. type moved into different module)
//!     * compiler version changes
//!     * version of a crate, where the type is declared, changes.
//!
//! ### Limitations
//!
//! Doesn't support non-static types and types with generic parameters.
//! If you want to derive `HasTypeId` trait on such type, consider using [newtype][2] pattern.
//!
//! ```
//! # use small_type_id::HasTypeId as _;
//! #[derive(small_type_id::HasTypeId)]
//! struct VecWithInts(/* wrapped generic type*/ Vec<i32>);
//! ```
//!
//! ### How uniqueness of TypeIds are enforced
//!
//! Uniqueness
//!
//! ## Examples
//!
//! Use for distinguishing 2 types.
//!
//! ```
//! # use small_type_id::HasTypeId as _;
//! #[derive(small_type_id::HasTypeId)]
//! struct A;
//! #[derive(small_type_id::HasTypeId)]
//! struct B;
//!
//! assert_ne!(A::TYPE_ID, B::TYPE_ID);
//! // Test in compile time:
//! const { assert!(A::TYPE_ID.as_u32() != B::TYPE_ID.as_u32()) };
//! ```
//!
//! Detect that types are same or not in generic code in compile time.
//!
//! ```
//! # use small_type_id::HasTypeId;
//! const fn is_types_unique<T0, T1, T2>()->bool
//! where
//!     T0: HasTypeId,
//!     T1: HasTypeId,
//!     T2: HasTypeId,
//! {
//!     let types = [T0::TYPE_ID, T1::TYPE_ID, T2::TYPE_ID];
//!     let mut i = 0;
//!     // Cannot use for loops because it is a const function.
//!     while i < types.len() {
//!         let t = types[i];
//!         let mut j = i + 1;
//!         while j < types.len() {
//!             if t.as_u32() == types[j].as_u32() {
//!                 return false;
//!             }
//!             j += 1;
//!         }
//!         i += 1;
//!     }
//!     true
//! }
//!
//! #[derive(small_type_id::HasTypeId)]
//! struct A;
//! #[derive(small_type_id::HasTypeId)]
//! struct B;
//! #[derive(small_type_id::HasTypeId)]
//! enum C{ A, B}
//!
//! const { assert!(is_types_unique::<A, B, C>()) };
//! const { assert!(! is_types_unique::<A, B, A>()) };
//! assert!(is_types_unique::<A, B, C>());
//! assert!(! is_types_unique::<A, B, A>());
//! ```
//!
//! ## Implementation details
//!
//! This implementation details are subject to change and should be not relied upon.
//!
//! Type id is computed by hashing string that contain full module
//! path, type name and crate version. Usage of module path allows to distinguish between
//! types with same name declared in different places, usage of crate version makes same type from
//! different versions of the same crate be interpreted as different types (which is correct behaviour).
//!
//! Current computation algorithm is [xxhash32][3].
//!
//! Also, we utilize [`ctor`][4] crate to register that type in global linked list and run verification
//! of uniqueness of `TypeId`s.
//!
//!
//!
//! [1]: https://doc.rust-lang.org/std/option/index.html#representation
//! [2]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
//! [3]: https://xxhash.com/
//! [4]: https://crates.io/crates/ctor
//!

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::uninlined_format_args)]
#![cfg_attr(not(test), no_std)]

use core::num::NonZeroU32;

mod hex;
mod implementation;
mod macros;

pub use implementation::private;

/// Implements [`HasTypeId`] trait and registers implementation for runtime verification.
///
/// Example:
/// ```
/// # use small_type_id::HasTypeId as _;
/// #[derive(small_type_id::HasTypeId)]
/// struct FirstStruct;
///
/// #[derive(small_type_id::HasTypeId)]
/// struct SecondStruct;
///
/// assert_ne!(FirstStruct::TYPE_ID, SecondStruct::TYPE_ID);
/// ```
///
/// It doesn't mix up types with same name from different modules:
/// ```
/// # use small_type_id::HasTypeId as _;
/// #[derive(small_type_id::HasTypeId)]
/// struct Struct;
///
/// mod inner {
///     #[derive(small_type_id::HasTypeId)]
///     pub struct Struct;
/// }
///
/// assert_ne!(Struct::TYPE_ID, inner::Struct::TYPE_ID);
/// ```
///
/// It doesn't support generic types, including lifetimes:
///
/// ```compile_fail
/// #[derive(small_type_id::HasTypeId)]
/// struct Generic<T>(T);
/// ```
///
/// ```compile_fail
/// #[derive(small_type_id::HasTypeId)]
/// struct Generic<'a>(&'a u32);
/// ```
///
pub use small_type_id_proc_macro::HasTypeId;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TypeId(pub(crate) NonZeroU32);

pub unsafe trait HasTypeId: 'static {
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
    #[must_use]
    #[inline]
    pub const fn from_user_code(code: NonZeroU32) -> Self {
        assert!(
            code.get() & 0x8000_0000 == 0x8000_0000,
            "User provided codes must set most significant byte to distinguish it from derived ones.",
        );
        Self(code)
    }

    #[must_use]
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    #[cfg(not(target_pointer_width = "16"))]
    #[must_use]
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0.get() as _
    }

    #[must_use]
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
        core::fmt::UpperHex::fmt(&self, f)
    }
}

impl core::fmt::Display for TypeId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::UpperHex::fmt(&self, f)
    }
}

impl core::fmt::UpperHex for TypeId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let hx = hex::HexView::new(self.as_u32());
        f.write_str(hx.as_str())
    }
}

impl core::fmt::Display for ErrorFromZeroBytes {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ErrorFromZeroBytes")
    }
}

impl core::error::Error for ErrorFromZeroBytes {}

#[cfg(doctest)]
#[doc = include_str!("../../ReadMe.md")]
pub struct ReadmeDoctests;

/// ```compile_fail
/// # use small_type_id::HasTypeId as _;
/// #[derive(small_type_id::HasTypeId)]
/// struct A;
/// #[derive(small_type_id::HasTypeId)]
/// struct B;
/// const { assert!(A::TYPE_ID.as_u32() == B::TYPE_ID.as_u32()) };
/// ```
#[cfg(doctest)]
pub struct ExtraDocTests;
