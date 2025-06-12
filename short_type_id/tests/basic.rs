#![cfg_attr(miri, allow(unused_imports))]

use std::collections::HashSet;

use short_type_id::{HasTypeId, TypeId};

#[derive(short_type_id::HasTypeId)]
struct MyType(#[allow(unused)] u32);

#[derive(short_type_id::HasTypeId)]
enum EnumType {
    _A,
    _B,
    _C(u32),
}

#[derive(short_type_id::HasTypeId)]
union UnionType {
    _f: f32,
    _u: usize,
}

mod some_module {
    #[derive(short_type_id::HasTypeId)]
    pub struct MyType(#[allow(unused)] u32);
}

#[derive(short_type_id::HasTypeId)]
struct r#CheckRawKeyword;

#[derive(short_type_id::HasTypeId)]
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
    let all_type_ids: Vec<TypeId> = short_type_id::iter_registered_entries()
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
    let second_iteration: Vec<TypeId> = short_type_id::iter_registered_entries()
        .map(|x| x.type_id)
        .collect();
    assert_eq!(all_type_ids, second_iteration);
}
