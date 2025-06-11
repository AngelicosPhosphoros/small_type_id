#![allow(non_camel_case_types)]

#[cfg(not(test))]
#[cfg(not(doctest))]
use std::collections::HashSet;

#[cfg(not(test))]
#[cfg(not(doctest))]
use short_type_id::HasTypeId;

#[cfg(not(test))]
#[cfg(not(doctest))]
#[derive(short_type_id::HasTypeId)]
struct uaaaaa58 {
    _f: u32,
}

#[cfg(not(test))]
#[cfg(not(doctest))]
#[derive(short_type_id::HasTypeId)]
enum iaaaac3b {
    _A,
    _B,
}

#[cfg(not(test))]
#[cfg(not(doctest))]
const _: () = {
    assert!(iaaaac3b::TYPE_ID.as_u32() == uaaaaa58::TYPE_ID.as_u32());
    ()
};

#[cfg(not(test))]
#[cfg(not(doctest))]
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

// We don't want to run this executable because it correctly abnormally exits before starting main.
#[cfg(any(test, doctest))]
fn main() {}
