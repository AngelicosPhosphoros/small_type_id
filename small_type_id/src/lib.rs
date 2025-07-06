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
//!    * Allows users to use this bit to distinguish with some other kind of id in a union (e.g. runtime types from some scripting engine).
//!
//! The following guarantees are effective for current version but may change with major release:
//!
//! 1. `TypeId` doesn't depend on layout of type. It is possible to change type from struct to
//!    enum and it would have same `TypeId`.
//! 2. `TypeId` doesn't change between recompilations.
//! 3. `TypeId` doesn't depend on compilation target if the declaration
//!    of the type doesn't move.
//!
//! `TypeId` can change on any of following changes:
//!
//!  * `small_type_id` crate version changes (including patch releases)
//!  * placement of the type changes (e.g. type moved into different module)
//!  * compiler version changes
//!  * version of a crate, that declare the type, changes (including patch releases).
//!
//! ### Limitations
//!
//! Doesn't support non-static types and types with generic parameters.
//! To derive `HasTypeId` trait on such type, consider using [newtype][2] pattern.
//!
//! ```
//! # use small_type_id::HasTypeId as _;
//! #[derive(small_type_id::HasTypeId)]
//! struct VecWithInts(/* wrapped generic type*/ Vec<i32>);
//! ```
//!
//! It is possible that 2 types end up having same type id
//! which would stop program from running.
//! In such case, it is possible to adjust generated ids manually by using
//! `#[small_type_id_seed=number]` attribute.
//!
//! ```
//! # use small_type_id::HasTypeId as _;
//! #[derive(small_type_id::HasTypeId)]
//! struct Type1{}
//! #[derive(small_type_id::HasTypeId)]
//! #[small_type_id_seed=42]
//! struct Type2{}
//! assert_ne!(Type1::TYPE_ID, Type2::TYPE_ID);
//! ```
//!
//! ### How uniqueness of `TypeIds` are enforced
//!
//! Using only 31 bit for [`TypeId`] makes it quite possible (though unlikely)
//! so it is necessaryto verify uniqueness of generated ids.
//!
//! All invocations of [`HasTypeId`](derive.HasTypeId.html) macro generate
//! code that registers the type in big list of types that have [`TypeId`].
//! Then, verification code is executed before `main`.
//!
//! Verification code generally executes with complexity _O(n<sup>2</sup>)_
//! although with very small constant on Windows and Linux, e.g. it processes 60000 types faster than 100 ms in debug build.
//!
//! However, if it is inacceptible, it can be disabled using [`unsafe_remove_duplicate_checks`](#feature-unsafe_remove_duplicate_checks)
//! feature. Enabling this feature is equivalent to **running unsafe code** so please consult it documentation
//! before enabling.
//!
//! If duplicate `TypeId`s detected, program would write some debug information to stderr
//! and terminate with error before reaching `main`.
//!
//! ## Available features
//!
//! ### Feature `debug_type_name`
//!
//! Saves type name in derive invocation of [`HasTypeId`](derive.HasTypeId.html) macro,
//! allowing to printing conflicting types in case of collision of [`HasTypeId::TYPE_ID`] values.
//!
//! The purpose of this feature only to debug cases of [`TypeId`] collisions.
//!
//! It is disabled by default to avoid wasting place in binary for useless strings.
//!
//! ### Feature `unsafe_remove_duplicate_checks`
//!
//! Disables automatic verification of uniqueness of [`TypeId`]s.
//! Use [`iter_registered_types`] function to run verification yourself.
//!
//! The purpose of this feature is
//!
//! * to avoid running any code before `main`
//! * to avoid _O(n<sup>2</sup>)_ complexity of automatic verification
//! * to prevent linking with libc or kernel32.
//!
//! Please, don't enable this feauture in library crates. This should be done only
//! in final binary crates because it may affect other libraries.
//!
//! ### Feature `unsafe_dont_register_types`
//!
//! Implies `unsafe_remove_duplicate_checks`.
//!
//! Disables type registration entirely. Exists to be used in embedded environments
//! when every single preserved byte is important.
//!
//! If you use this feature, do run tests without it before deploying your code.
//!
//! If this feature is enabled, there is no way to ensure that uniqueness
//! of `TypeId`s is still guaranteed.
//!
//! ## Semver breaking policy
//!
//! The following changes are not considered breaking:
//!
//! 1. Changes of string representation of type ids
//! 2. Changes of uniqueness verification algorithm
//! 3. Change of type id generation algorithm (and resulting values of type ids)
//! 4. Changes of type registration code
//! 5. Additions of new types, functions, modules
//! 6. Any changes in `private` module (users should not use it directly)
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
//! Adjusting generated `TypeId` using `#[small_type_id_seed]`
//!
//! ```
//! #[derive(small_type_id::HasTypeId)]
//! #[small_type_id_seed=42]
//! enum MyType {A, B}
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
//! We collect all derived types either by using statics linked to special section (on Linux and Windows),
//! or put them in a linked list by running code before `main`. We run code before `main` using crate [`ctor`][4].
//!
//!
//!
//! [1]: https://doc.rust-lang.org/std/option/index.html#representation
//! [2]: https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html
//! [3]: https://xxhash.com/
//! [4]: https://crates.io/crates/ctor
//!

#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::uninlined_format_args, clippy::collapsible_if)]
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
/// struct SecondStruct{}
///
/// assert_ne!(FirstStruct::TYPE_ID, SecondStruct::TYPE_ID);
/// ```
///
/// It doesn't mix up types with same name from different modules:
/// ```
/// # use small_type_id::HasTypeId as _;
/// #[derive(small_type_id::HasTypeId)]
/// struct Struct{}
///
/// mod inner {
///     #[derive(small_type_id::HasTypeId)]
///     pub struct Struct{}
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

/// Unique id for a type.
/// Have extra invariants about internal structure, described in [module documentation](index.html).
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
#[non_exhaustive]
pub struct TypeId(pub(crate) NonZeroU32);

/// Marks that type has [`TypeId`]
///
/// Implemented using derive macro.
///
/// ```
/// #[derive(small_type_id::HasTypeId)]
/// struct SomeType{}
/// ```
///
/// It is possible to adjust resulting `TYPE_ID` by setting seed (must be `u32` literal):
///
/// ```
/// #[derive(small_type_id::HasTypeId)]
/// #[small_type_id_seed=42]
/// struct SomeType{}
/// ```
///
/// # Safety
///
/// To ensure that all [`HasTypeId::TYPE_ID`] values are unique,
/// derive macro does bookkeeping and verification before invokation of `main`.
/// Please, just use [derive macro](derive.HasTypeId.html).
pub unsafe trait HasTypeId: 'static {
    /// Unique identifier of type.
    const TYPE_ID: TypeId;
}

/// Entry that describes registered type information.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct TypeEntry {
    /// Type id of entry.
    /// Useful for testing that all generated type ids are unique.
    pub type_id: TypeId,
    /// This field are useful for debugging.
    /// **Do not** use it as key.
    /// Available only if feature [`debug_type_name`](./index.html#feature-debug_type_name) is enabled.
    #[cfg(feature = "debug_type_name")]
    pub debug_type_name: &'static str,
}

/// Allows iteration over types that implemented [`HasTypeId`] trait using derive macro.
///
/// Doesn't work if feature [`unsafe_dont_register_types`](./index.html#feature-unsafe_dont_register_types) is enabled.
pub fn iter_registered_types() -> impl Iterator<Item = TypeEntry> {
    implementation::pub_iter_registered_types()
}

/// Error type for [`TypeId::from_bytes`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct ErrorInvalidBytes {
    // Prevent construction in user code.
    _x: (),
}

impl TypeId {
    /// Returns value of type id as [`u32`].
    /// Useful for comparing type ids in const context
    /// because [`PartialEq`] trait doesn't work in it.
    ///
    /// ```
    /// # use small_type_id::HasTypeId as _;
    /// #[derive(small_type_id::HasTypeId)]
    /// struct Type1{}
    /// #[derive(small_type_id::HasTypeId)]
    /// #[small_type_id_seed=42]
    /// struct Type2{}
    /// const { assert!(Type1::TYPE_ID.as_u32() != Type2::TYPE_ID.as_u32()) };
    /// ```
    #[must_use]
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.get()
    }

    /// Just for convenient conversion to [`usize`].
    #[cfg(not(target_pointer_width = "16"))]
    #[must_use]
    #[inline]
    pub const fn as_usize(self) -> usize {
        self.0.get() as _
    }

    /// Allows serializing value to bytes.
    /// Note that value of `TypeId` for a type is not stable
    /// (it can change if you change version of the crate)
    /// so don't use it for persistent data.
    #[must_use]
    #[inline]
    pub const fn to_bytes(self) -> [u8; 4] {
        // We use to_le_bytes because
        // it is native for most modern systems.
        self.0.get().to_le_bytes()
    }

    /// Allows deserializing value from bytes.
    ///
    /// # Safety
    ///
    /// Bytes should be from call to [`TypeId::to_bytes`].
    ///
    /// # Errors
    ///
    /// Return error if bytes contain definitely invalid `TypeId`
    /// (doesn't conform to invariants).
    #[inline]
    pub const unsafe fn from_bytes(bytes: [u8; 4]) -> Result<Self, ErrorInvalidBytes> {
        const ALLOWED_BITS: u32 = u32::MAX >> 1;
        let val = u32::from_le_bytes(bytes);
        match NonZeroU32::new(val) {
            Some(x) if val & ALLOWED_BITS == val => Ok(Self(x)),
            _ => Err(ErrorInvalidBytes { _x: () }),
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

impl core::fmt::Display for ErrorInvalidBytes {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ErrorInvalidBytes")
    }
}

impl core::error::Error for ErrorInvalidBytes {}

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
