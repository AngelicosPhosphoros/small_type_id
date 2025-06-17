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

    #[must_use]
    pub const fn compute_id(module_name_version: &[u8]) -> TypeId {
        let hash = xxh32(module_name_version, 0);
        let val = if hash == 0 { 1 } else { hash } & 0x7FFF_FFFF_u32;
        TypeId(NonZeroU32::new(val).unwrap())
    }

    /// Useful for concatenating byte slices in compile time.
    /// This exists solely to be able to compile Rust code without cargo
    /// which prevents us fron getting current package version using [env!()][1] macro
    /// with [CARGO_PKG_VERSION][2] environment variable.
    ///
    /// [1]: https://doc.rust-lang.org/std/macro.env.html
    /// [2]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    #[must_use]
    pub const fn concat_bytes<const SUM_LEN: usize>(s0: &[u8], s1: &[u8]) -> [u8; SUM_LEN] {
        let mut res = [0; SUM_LEN];
        let (a, b) = res.split_at_mut(s0.len());
        a.copy_from_slice(s0);
        b.copy_from_slice(s1);
        res
    }
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
    const MY_HASH: TypeId = private::compute_id(concat!(module_path!(), "::", "MyType").as_bytes());
    #[allow(unused)]
    const MY_HASH_AND_CRATE: TypeId = {
        const CRATE_VERSION: &str = if let Some(x) = Some("0.1.1") { x } else { "" };
        if CRATE_VERSION.is_empty() {
            private::compute_id(concat!(module_path!(), "::", "MyType").as_bytes())
        } else {
            const SUM_LEN: usize =
                concat!(module_path!(), "::", "MyType", "::").len() + CRATE_VERSION.len();
            let concatenated: [u8; SUM_LEN] = private::concat_bytes(
                concat!(module_path!(), "::", "MyType", "::").as_bytes(),
                CRATE_VERSION.as_bytes(),
            );
            private::compute_id(&concatenated)
        }
    };
    #[allow(unused)]
    const MY_HASH_AND_NO_CRATE: TypeId = {
        const CRATE_VERSION: &str = if let Some(x) = None { x } else { "" };
        if CRATE_VERSION.is_empty() {
            private::compute_id(concat!(module_path!(), "::", "MyType").as_bytes())
        } else {
            const SUM_LEN: usize =
                concat!(module_path!(), "::", "MyType", "::").len() + CRATE_VERSION.len();
            let concatenated: [u8; SUM_LEN] = private::concat_bytes(
                concat!(module_path!(), "::", "MyType", "::").as_bytes(),
                CRATE_VERSION.as_bytes(),
            );
            private::compute_id(&concatenated)
        }
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
        assert_eq!(MY_HASH.as_u32(), 0x78DF6782);
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

        let _my_hash = MY_HASH;

        // This would be used for testing duplicate lookup in types.
        assert_eq!(compute_id(b"hogtied"), compute_id(b"scouriness"));
        assert_eq!(compute_id(b"hogtied").as_u32(), 0x5034ABE3);

        assert_eq!(compute_id(b"usize").as_u32(), 0x7847CC2E);

        // Check that we do not generate zeros.
        assert_eq!(xxh32(b"AasZkWq", 0), 0);
        assert_eq!(xxh32(b"RalEB24", 0), 0);
        assert_ne!(compute_id(b"AasZkWq").as_u32(), 0);
        assert_ne!(compute_id(b"RalEB24").as_u32(), 0);
        assert_eq!(compute_id(b"AasZkWq").as_u32(), 1);
        assert_eq!(compute_id(b"RalEB24").as_u32(), 1);

        assert_eq!(u32::MAX >> 31, 1);
        assert_eq!(compute_id(b"assaulted").as_u32() >> 31, 0);
        assert_eq!(compute_id(b"usize").as_u32() >> 31, 0);
        assert_eq!(compute_id(b"AasZkWq").as_u32() >> 31, 0);
        assert_eq!(compute_id(b"RalEB24").as_u32() >> 31, 0);
    }

    #[test]
    fn test_concat_strs() {
        let hello_world = const { private::concat_bytes::<12>(b"Hello ", b"world!") };
        assert_eq!(hello_world, *b"Hello world!");
        let world = const { private::concat_bytes::<6>(b"", b"world!") };
        assert_eq!(world, *b"world!");
        let hello = const { private::concat_bytes::<6>(b"Hello ", b"") };
        assert_eq!(hello, *b"Hello ");
    }

    #[test]
    #[should_panic]
    fn too_short_not_ok() {
        let _ = private::concat_bytes::<11>(b"Hello ", b"world!");
    }

    #[test]
    #[should_panic]
    fn too_large_not_ok() {
        let _ = private::concat_bytes::<13>(b"Hello ", b"world!");
    }
}
