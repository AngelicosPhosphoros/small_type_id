use core::num::NonZeroU32;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed};

use xxhash_rust::const_xxh32::xxh32;

use crate::TypeId;
#[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
use crate::hex;

// Functions and types used in macro generated code.
#[doc(hidden)]
pub mod private {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub use ctor::declarative::ctor;

    pub struct TypeEntry {
        pub(super) type_id: TypeId,
        #[cfg(feature = "debug_type_name")]
        pub(super) type_name: &'static str,
        pub(super) next: AtomicPtr<TypeEntry>,
    }

    impl TypeEntry {
        #[must_use]
        pub const fn new(type_name: &'static str, type_id: TypeId) -> TypeEntry {
            let _ = type_name;

            Self {
                type_id,
                next: AtomicPtr::new(ptr::null_mut()),
                #[cfg(feature = "debug_type_name")]
                type_name,
            }
        }
    }

    #[cold]
    pub unsafe fn register_type(entry: &'static TypeEntry) {
        debug_assert!(
            entry.next.load(Relaxed).is_null(),
            "TypeEntries must be generated only using macro"
        );

        let mut next = LAST_ADDED_TYPE.load(Relaxed);
        loop {
            entry.next.store(next, Relaxed);
            let p: *mut TypeEntry = ptr::from_ref(entry).cast_mut();
            match LAST_ADDED_TYPE.compare_exchange_weak(next, p, AcqRel, Relaxed) {
                Ok(_) => break,
                Err(p) => next = p,
            }
        }

        // This code tests that we don't have registered any duplicates.
        // Unfortunately, it runs in quadratic time.
        #[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
        unsafe {
            let typeid = entry.type_id;
            let mut it: *const TypeEntry = entry.next.load(Relaxed);
            while !it.is_null() {
                if (*it).type_id == typeid {
                    handle_duplicate_typeid(
                        typeid,
                        #[cfg(feature = "debug_type_name")]
                        (*it).type_name,
                        #[cfg(feature = "debug_type_name")]
                        entry.type_name,
                    );
                }
                it = (*it).next.load(Relaxed);
            }
        }
    }

    /// Use [`compute_input_len`] to compute `TOTAL_INPUT_LEN`.
    #[must_use]
    pub const fn compute_id<const TOTAL_INPUT_LEN: usize>(
        module_and_name: &str,
        crate_version: Option<&str>,
    ) -> TypeId {
        let hash = if let Some(crate_version) = crate_version {
            let mut concatenated = [0; TOTAL_INPUT_LEN];
            // Need to use split_at_mut because slice[x..y] syntax doesn't work in const fns.
            let (head, tail) = concatenated.split_at_mut(module_and_name.len());
            let (delim, version) = tail.split_at_mut(2);

            head.copy_from_slice(module_and_name.as_bytes());
            delim.copy_from_slice(b"::");
            version.copy_from_slice(crate_version.as_bytes());

            xxh32(&concatenated, 0)
        } else {
            assert!(TOTAL_INPUT_LEN == module_and_name.len());
            xxh32(module_and_name.as_bytes(), 0)
        };

        let val = if hash == 0 { 1 } else { hash } & 0x7FFF_FFFF_u32;
        TypeId(NonZeroU32::new(val).unwrap())
    }

    #[must_use]
    pub const fn compute_input_len(module_and_name: &str, crate_version: Option<&str>) -> usize {
        module_and_name.len()
            + match crate_version {
                Some(v) => 2 + v.len(),
                None => 0,
            }
    }

    pub use crate::private_macro_implement_type_and_register as implement_type_and_register;
    pub use crate::private_macro_implement_type_id as implement_type_id;
    pub use crate::private_macro_register_type_id as register_type_id;
}

static LAST_ADDED_TYPE: AtomicPtr<private::TypeEntry> = AtomicPtr::new(ptr::null_mut());

pub(crate) fn iter_registered_entries() -> impl Iterator<Item = crate::TypeEntry> {
    let mut current = LAST_ADDED_TYPE.load(Acquire);
    core::iter::from_fn(move || unsafe {
        if let Some(rf) = current.as_ref() {
            current = rf.next.load(Relaxed);
            Some(crate::TypeEntry {
                type_id: rf.type_id,
                #[cfg(feature = "debug_type_name")]
                type_name: rf.type_name,
            })
        } else {
            None
        }
    })
}

#[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
#[cold]
#[inline(never)]
fn handle_duplicate_typeid(
    type_id: TypeId,
    #[cfg(feature = "debug_type_name")] type_name1: &str,
    #[cfg(feature = "debug_type_name")] type_name2: &str,
) -> ! {
    #[cfg_attr(windows, path = "win.rs")]
    #[cfg_attr(unix, path = "unix.rs")]
    mod platform;

    let hex_val = hex::HexView::new(type_id.as_u32());
    // Safety: well, we just call libc or WinAPI functions.
    // This code runs before main so we cannot run code from stdlib so we can't really synchronize access to stderr.
    // It probably the only running thread in application.
    // Anyway, this function ends by terminates current process so any memory unsafety would end here.
    unsafe {
        let mut stderr = platform::get_stderr();

        platform::print_error(&mut stderr, "small_type_id: Found duplicate type_id ");
        platform::print_error(&mut stderr, hex_val.as_str());
        #[cfg(not(feature = "debug_type_name"))]
        {
            platform::print_error(
                &mut stderr,
                r#". Consider enabling "debug_type_name" feature to display conflicting type names"#,
            );
        }
        #[cfg(feature = "debug_type_name")]
        {
            platform::print_error(&mut stderr, " for types ");
            platform::print_error(&mut stderr, type_name1);
            platform::print_error(&mut stderr, " and ");
            platform::print_error(&mut stderr, type_name2);
        }
        platform::print_error(&mut stderr, ".\n");

        platform::terminate_current_process(stderr)
    }
}

#[cfg(test)]
#[allow(clippy::wildcard_imports)]
mod tests {
    use super::*;

    // This checks that we can compute const TypeId in compile time.
    #[allow(unused)]
    const MY_HASH_AND_CRATE: TypeId = {
        const INPUT_LEN: usize =
            private::compute_input_len(concat!(module_path!(), "::", "MyType"), Some("0.1.1"));
        private::compute_id::<INPUT_LEN>(concat!(module_path!(), "::", "MyType"), Some("0.1.1"))
    };
    #[allow(unused)]
    const MY_HASH_AND_NO_CRATE: TypeId = {
        const INPUT_LEN: usize =
            private::compute_input_len(concat!(module_path!(), "::", "MyType"), None);
        private::compute_id::<INPUT_LEN>(concat!(module_path!(), "::", "MyType"), None)
    };

    #[test]
    fn check_constants() {
        assert_eq!(
            xxh32(b"small_type_id::implementation::tests::MyType", 0),
            0xF8DF6782
        );
        assert_eq!(
            xxh32(b"small_type_id::implementation::tests::MyType::0.1.1", 0),
            0x3C3D45A6
        );
        assert_eq!(MY_HASH_AND_CRATE.as_u32(), 0x3C3D45A6);
        assert_eq!(MY_HASH_AND_NO_CRATE.as_u32(), 0x78DF6782);
    }

    #[test]
    fn test_xxh32() {
        // Same as test in extra_tests/duplicate_type_ids_handling
        assert_eq!(
            xxh32(b"duplicate_type_ids_handling::XaaG::0.0.0", 0),
            xxh32(b"duplicate_type_ids_handling::Jaaadtd::0.0.0", 0),
        )
    }

    #[test]
    fn compute_id() {
        use private::compute_id;

        // This would be used for testing duplicate lookup in types.
        assert_eq!(
            compute_id::<7>("hogtied", None),
            compute_id::<10>("scouriness", None)
        );
        assert_eq!(compute_id::<7>("hogtied", None).as_u32(), 0x5034ABE3);

        assert_eq!(compute_id::<5>("usize", None).as_u32(), 0x7847CC2E);
        assert_eq!(compute_id::<12>("usize", Some("0.0.0")).as_u32(), 0x3064D6AA);
        assert_eq!(compute_id::<12>("usize", Some("0.0.1")).as_u32(), 0x791B53F2);

        // Check that we do not generate zeros.
        assert_eq!(xxh32(b"AasZkWq", 0), 0);
        assert_eq!(xxh32(b"RalEB24", 0), 0);
        assert_ne!(compute_id::<7>("AasZkWq", None).as_u32(), 0);
        assert_ne!(compute_id::<7>("RalEB24", None).as_u32(), 0);
        assert_eq!(compute_id::<7>("AasZkWq", None).as_u32(), 1);
        assert_eq!(compute_id::<7>("RalEB24", None).as_u32(), 1);

        assert_eq!(u32::MAX >> 31, 1);
        assert_eq!(compute_id::<9>("assaulted", None).as_u32() >> 31, 0);
        assert_eq!(compute_id::<5>("usize", None).as_u32() >> 31, 0);
        assert_eq!(compute_id::<7>("AasZkWq", None).as_u32() >> 31, 0);
        assert_eq!(compute_id::<7>("RalEB24", None).as_u32() >> 31, 0);
    }

    #[test]
    #[should_panic]
    fn compute_id_invalid_len_none() {
        use private::compute_id;
        let _ = compute_id::<6>("hogtied", None);
    }

    #[test]
    #[should_panic]
    fn compute_id_invalid_len_some() {
        use private::compute_id;
        let _ = compute_id::<7>("hogtied", Some("a.b.c"));
    }

    #[test]
    fn compute_input_len() {
        use private::compute_input_len;
        assert_eq!(compute_input_len("", None), 0);
        assert_eq!(compute_input_len("Hello", None), 5);
        assert_eq!(compute_input_len("", Some("xxx")), 5);
        assert_eq!(compute_input_len("Hello", Some("0.1.2")), 12);
    }
}
