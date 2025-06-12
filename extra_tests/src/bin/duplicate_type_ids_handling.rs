#![allow(non_camel_case_types)]
// We don't want to register this executable
// because it correctly abnormally exits before starting main
// which used to collect tests.
#![cfg(not(any(test, doctest)))]

use std::collections::HashSet;

use short_type_id::HasTypeId;

#[derive(short_type_id::HasTypeId)]
struct uaaaaa58 {
    _f: u32,
}

#[derive(short_type_id::HasTypeId)]
enum iaaaac3b {
    _A,
    _B,
}

const _: () = {
    assert!(iaaaac3b::TYPE_ID.as_u32() == uaaaaa58::TYPE_ID.as_u32());
};

fn main() {
    let mut set = HashSet::new();
    for entry in short_type_id::iter_registered_entries() {
        println!("Checking {}", entry.type_id);
        if !set.insert(entry.type_id) {
            eprintln!(
                "Detected error at the start of main! Found duplicate type_id {}.",
                entry.type_id
            );
        }
    }
}
