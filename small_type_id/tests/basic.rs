#![cfg_attr(miri, allow(unused_imports))]

use std::collections::HashSet;

use small_type_id::{HasTypeId, TypeId};

#[derive(small_type_id::HasTypeId)]
struct MyType(#[allow(unused)] u32);

#[derive(small_type_id::HasTypeId)]
#[small_type_id_seed = 797]
enum EnumType {
    _A,
    _B,
    _C(u32),
}

#[derive(small_type_id::HasTypeId)]
#[repr(C)]
#[cfg_attr(windows, small_type_id_seed = 797)]
union UnionType {
    _f: f32,
    _u: usize,
}

mod some_module {
    #[derive(small_type_id::HasTypeId)]
    pub struct MyType(#[allow(unused)] u32);
}

#[derive(small_type_id::HasTypeId)]
struct r#CheckRawKeyword;

#[derive(small_type_id::HasTypeId)]
#[allow(non_camel_case_types)]
struct r#pub;

#[test]
fn check_values() {
    let arr = [
        MyType::TYPE_ID,
        EnumType::TYPE_ID,
        UnionType::TYPE_ID,
        some_module::MyType::TYPE_ID,
        CheckRawKeyword::TYPE_ID,
        r#pub::TYPE_ID,
    ];
    for (i, &left) in arr.iter().enumerate() {
        for &right in arr[i + 1..].iter() {
            assert_ne!(left, right);
        }
    }
}

#[test]
fn types_in_modules_differ() {
    assert_ne!(MyType::TYPE_ID, some_module::MyType::TYPE_ID);
}

#[test]
// MIRI unsupported until https://github.com/rust-lang/miri/issues/450 fixed
#[cfg(not(miri))]
fn iter_types() {
    if cfg!(feature = "unsafe_dont_register_types") {
        assert_eq!(small_type_id::iter_registered_entries().count(), 0);
        return;
    }
    let all_type_ids: Vec<TypeId> = small_type_id::iter_registered_entries()
        .map(|x| x.type_id)
        .collect();
    assert_eq!(all_type_ids.len(), 6);
    let set: HashSet<TypeId> = all_type_ids.iter().copied().collect();
    let etalon: HashSet<TypeId> = {
        [
            MyType::TYPE_ID,
            EnumType::TYPE_ID,
            UnionType::TYPE_ID,
            some_module::MyType::TYPE_ID,
            CheckRawKeyword::TYPE_ID,
            r#pub::TYPE_ID,
        ]
        .into_iter()
        .collect()
    };
    assert_eq!(set, etalon);
    let second_iteration: Vec<TypeId> = small_type_id::iter_registered_entries()
        .map(|x| x.type_id)
        .collect();
    assert_eq!(all_type_ids, second_iteration);
}

#[cfg(feature = "debug_type_name")]
// MIRI unsupported until https://github.com/rust-lang/miri/issues/450 fixed
#[cfg(not(miri))]
#[test]
fn test_id_to_name() {
    use std::collections::HashMap;

    #[rustfmt::skip]
    let key_to_name: HashMap<TypeId, &str> = [
            (MyType::TYPE_ID,              "basic::MyType"),
            (EnumType::TYPE_ID,            "basic::EnumType"),
            (UnionType::TYPE_ID,           "basic::UnionType"),
            (some_module::MyType::TYPE_ID, "basic::some_module::MyType"),
            (CheckRawKeyword::TYPE_ID,     "basic::CheckRawKeyword"),
            (r#pub::TYPE_ID,               "basic::pub"),
        ]
        .into_iter()
        .collect();
    for entry in small_type_id::iter_registered_entries() {
        assert_eq!(entry.debug_type_name, key_to_name[&entry.type_id]);
    }
}

#[test]
fn check_initial_value() {
    eprintln!("{}", env!("CARGO_PKG_VERSION"));
    assert_eq!(
        xxhash_rust::const_xxh32::xxh32(
            concat!("basic::UnionType::", env!("CARGO_PKG_VERSION")).as_bytes(),
            if cfg!(windows) { 797 } else { 0 }
        ) ^ 0x8000_0000,
        UnionType::TYPE_ID.as_u32()
    );
}
