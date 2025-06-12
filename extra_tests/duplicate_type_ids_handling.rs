#![allow(non_camel_case_types)]
// We don't want to register this executable
// because it correctly abnormally exits before starting main
// which used to collect tests.
#![cfg(not(any(test, doctest)))]

use std::collections::HashSet;

use short_type_id::HasTypeId;

#[derive(short_type_id::HasTypeId)]
pub struct uaaaaa58 {
    pub f: u32,
}

#[derive(short_type_id::HasTypeId)]
pub enum iaaaac3b {
    A,
    B,
}

const _: () = {
    assert!(iaaaac3b::TYPE_ID.as_u32() == uaaaaa58::TYPE_ID.as_u32());
};

fn main() {
    let mut set = HashSet::new();
    let mut tested = 0;
    #[cfg(feature = "debug_type_name")]
    let mut names = Vec::new();
    for entry in short_type_id::iter_registered_entries() {
        if !set.insert(entry.type_id) {
            eprintln!(
                "Detected error at the start of main! Found duplicate type_id {}.",
                entry.type_id
            );
        }
        tested += 1;
        #[cfg(feature = "debug_type_name")]
        names.push(entry.type_name);
    }
    println!("Tested {} entries, found {} types", tested, set.len());
    #[cfg(feature = "debug_type_name")]
    {
        use std::println;

        names.sort_unstable();
        let joined = names.join(", ");
        println!("Got names: {}", joined);
    }
}
