#![allow(
    non_camel_case_types,
    clippy::uninlined_format_args,
    clippy::collapsible_if
)]
// We don't want to register this executable
// because it correctly abnormally exits before starting main
// which used to collect tests.
#![cfg(not(any(test, doctest)))]

use std::collections::HashSet;

use small_type_id::HasTypeId;

#[derive(small_type_id::HasTypeId)]
pub struct XaaG {
    pub f: u32,
}

#[derive(small_type_id::HasTypeId)]
pub enum Jaaadtd {
    A,
    B,
}

const _: () = {
    assert!(XaaG::TYPE_ID.as_u32() == Jaaadtd::TYPE_ID.as_u32());
};

fn main() {
    let mut set = HashSet::new();
    let mut tested = 0;
    #[cfg(feature = "debug_type_name")]
    let mut names = Vec::new();
    for entry in small_type_id::iter_registered_entries() {
        if !set.insert(entry.type_id) {
            eprintln!(
                "Detected error at the start of main! Found duplicate type_id {}.",
                entry.type_id
            );
        }
        tested += 1;
        #[cfg(feature = "debug_type_name")]
        names.push(entry.debug_type_name);
    }
    println!("Tested {} entries, found {} types", tested, set.len());
    #[cfg(feature = "debug_type_name")]
    {
        names.sort_unstable();
        let joined = names.join(", ");
        println!("Got names: {}", joined);
    }
}
