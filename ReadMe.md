# THIS IS ALPHA RELEASE PLEASE DO NOT USE
# Small Type Id

This crate provides trait `HasTypeId` with associated constant `TYPE_ID` and derive macro to implement it for your types.

There are 4 guarantees:

1. `TYPE_ID` is a constant.
2. Size is 32 bit.
3. `TypeId` cannot be zero which allows [niche optimizations][1] (`size_of::<Option<TypeId>>()` is 4 bytes).
4. Most significant bit (_MSB_) is guaranteed to be zero:
   * Allows users to use this bit to distinguish with some other kind of id in a union (e.g. Runtime types from some scripting engine).
   * Does **not** allow niche optimizations on current version of Rust yet.

Those guarantees would never be removed (even in semver breaking releases) so you can update dependency on this crate without validating your code that rely on them.

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

More examples and implementation explanation are available in documentation.

[1]: https://doc.rust-lang.org/std/option/index.html#representation
[2]: https://docs.rs/typeid/1.0.3/typeid/struct.ConstTypeId.html
[3]: https://crates.io/crates/typeid
[4]: https://doc.rust-lang.org/std/any/struct.TypeId.html
[5]: https://crates.io/crates/ctor
