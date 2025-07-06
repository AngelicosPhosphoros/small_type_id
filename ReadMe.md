# THIS IS ALPHA RELEASE PLEASE DO NOT USE

[![Crates.io Version](https://img.shields.io/crates/v/small_type_id)](https://crates.io/crates/small_type_id)
[![tests](https://github.com/AngelicosPhosphoros/small_type_id/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AngelicosPhosphoros/small_type_id/actions)
[![docs.rs](https://img.shields.io/docsrs/small_type_id)](https://docs.rs/small_type_id/latest/small_type_id/)

# Small Type Id

This crate provides trait `HasTypeId` with associated constant `TYPE_ID` and derive macro to implement it for your types.

There are 4 guarantees:

1. `TYPE_ID` is a constant.
2. Size is 32 bit.
3. `TypeId` cannot be zero which allows [niche optimizations][1] (`size_of::<Option<TypeId>>()` is 4 bytes).
4. Most significant bit (_MSB_) is guaranteed to be zero:
   * Allows users to use this bit to distinguish with some other kind of id in a union (e.g. Runtime types from some scripting engine).
   * Does **not** allow niche optimizations on current version of Rust yet.

Those guarantees would never be removed (even in semver breaking releases) so you can update dependency on this crate without validating your code that rely on that guarantees.

Uniqueness of `TypeIds` is enforced by running code before `fn main()` using [`ctor`][5] crate.

## Comparison with [`std::any::TypeId`][4]

With `std::any::TypeId` as at Rust 1.87.

#### Advantages

1. `TYPE_ID` is a constant.
2. Size of `small_type_id::TypeId` is guaranteed to be 32 bits.
3. Size of `small_type_id::TypeId` is significantly smaller(4 vs 16 bytes), allowing better performance due to less usage of CPU cache.
4. `small_type_id::TypeId` supports niche optimization for `Option<small_type_id::TypeId>`.
5. `small_type_id::TypeId` guarantees that MSB is zero, allowing creating 32 bit identifiers by users using `union`s:
   * Since user types would need to set MSB to 1, resulting value is still cannot be zero, allowing niche optimization.

#### Downsides

1. `small_type_id::HasTypeId` needs to be derived for supported types, it doesn't work automatically.
2. `small_type_id::HasTypeId` doesn't support generic types.

## Comparison with [`typeid::ConstTypeId`][2]

With [`typeid`][3] version 1.0.3

#### Advantages

1. Has smaller size (32 bit vs 64 bit on 64-bit targets).
2. Has defined internal representation that can be utilized by users.

#### Disadvantages

1. Doesn't support every type.

## How it works and how to use API.

Example:

```rust
use small_type_id::HasTypeId as _;

#[derive(small_type_id::HasTypeId)]
pub struct Struct {
    pub a: u32,
    pub b: usize,
}

#[derive(small_type_id::HasTypeId)]
pub enum Enum {
    A(u32), B(String)
}

// Check that they are different:
assert_ne!(Struct::TYPE_ID, Enum::TYPE_ID);
// Or even in compile time:
const { assert!(Struct::TYPE_ID.as_u32() != Enum::TYPE_ID.as_u32()); };
```

More examples and implementation explanation are available in [documentation][6].

## Safety

Uniqueness is tested before running `fn main()` (unless opt-out), so code can rely on type ids being unique.
Code is tested in CI on following platforms:

1. macos
2. Windows
3. Linux with the following libc implementations:
    * glibc
    * musl
    * [eyra-libc][10]

There is also some testing with [Address-Sanitizer][11].

## Acknowledgments

* Thanks to [**mmastrac**][7] for crate [`ctor`][5] used for implementing this crate.
* Thanks to [**dtolnay**][8] for crate `linkme` which helped to learn me about using
link sections for gathering statics.
* Thanks to [**Raymond Chen**][9] for his explanations about linker sections.

[1]: https://doc.rust-lang.org/std/option/index.html#representation
[2]: https://docs.rs/typeid/1.0.3/typeid/struct.ConstTypeId.html
[3]: https://crates.io/crates/typeid
[4]: https://doc.rust-lang.org/std/any/struct.TypeId.html
[5]: https://crates.io/crates/ctor
[6]: https://docs.rs/small_type_id/latest/small_type_id/
[7]: https://crates.io/users/mmastrac
[8]: https://crates.io/users/dtolnay
[9]: https://devblogs.microsoft.com/oldnewthing/author/oldnewthing
[10]: https://crates.io/crates/eyra
[11]: https://doc.rust-lang.org/beta/unstable-book/compiler-flags/sanitizer.html#addresssanitizer
