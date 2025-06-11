use core::num::NonZeroU32;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed};

use crate::{TypeId, hex};

// Functions and types used in macro generated code.
#[doc(hidden)]
pub mod private {
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
            let mut it_slow: *const TypeEntry = ptr::from_ref(entry);
            while !it_slow.is_null() {
                let typeid = (*it_slow).type_id;
                let mut it_fast: *const TypeEntry = (*it_slow).next.load(Relaxed);
                while !it_fast.is_null() {
                    if (*it_fast).type_id == typeid {
                        handle_duplicate_typeid(
                            typeid,
                            #[cfg(feature = "debug_type_name")]
                            (*it_fast).type_name,
                            #[cfg(feature = "debug_type_name")]
                            (*it_slow).type_name,
                        );
                    }
                    it_fast = (*it_fast).next.load(Relaxed);
                }
                it_slow = (*it_slow).next.load(Relaxed);
            }
        }
    }

    #[must_use]
    pub const fn compute_id(module_name_version: &[u8]) -> TypeId {
        let hash = murmur_v3(module_name_version, MURMUR_SEED);
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

const MURMUR_SEED: u32 = 0xF1D4_B28B;
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

#[must_use]
const fn murmur_v3(src: &[u8], seed: u32) -> u32 {
    let mut h: u32 = seed;
    let mut i = 0;
    while i + 4 <= src.len() {
        // Read next 4 byte number as little endian.
        let mut k: u32 = 0;
        let mut j = 0;
        while j < 4 {
            k |= (src[i] as u32) << (8 * j);
            i += 1;
            j += 1;
        }

        h ^= murmur_32_scramble(k);
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xE654_6B64);
    }
    if i < src.len() {
        // Read next bytes as little endian number until end.
        let mut k: u32 = 0;
        let mut j = 0;
        while i < src.len() {
            k |= (src[i] as u32) << (8 * j);
            i += 1;
            j += 1;
        }

        h ^= murmur_32_scramble(k);
    }

    assert!(src.len() <= u32::MAX as usize);
    #[allow(clippy::cast_possible_truncation)]
    let len = src.len() as u32;

    h ^= len;
    h ^= h >> 16;
    h = u32::wrapping_mul(h, 0x85eb_ca6b);
    h ^= h >> 13;
    h = u32::wrapping_mul(h, 0xc2b2_ae35);
    h ^= h >> 16;

    h
}

#[must_use]
const fn murmur_32_scramble(k: u32) -> u32 {
    k.wrapping_mul(0xcc9e_2d51)
        .rotate_left(15)
        .wrapping_mul(0x1b87_3593)
}

#[cfg(all(not(feature = "unsafe_remove_duplicate_checks"), unix))]
mod unix {
    #[repr(C)]
    pub(super) struct File(core::ffi::c_void);
    pub(super) const STDERR_FILENO: i32 = 2;
}
#[cfg(all(not(feature = "unsafe_remove_duplicate_checks"), unix))]
unsafe extern "C" {
    fn fdopen(fd: i32, mode: *const u8) -> *mut unix::File;
    fn fwrite(buffer: *const u8, elem_size: usize, len: usize, file: *mut unix::File) -> usize;
    fn fflush(file: *mut unix::File) -> i32;
    fn abort() -> !;
}

#[cfg(windows)]
mod win {
    pub(super) type Handle = u32;
    pub(super) const STD_ERROR_HANDLE: Handle = 0xFFFF_FFF4;
    pub(super) const PROCESS_TERMINATE_ACCESS: u32 = 1;
}
#[cfg(windows)]
#[link(name = "Kernel32", kind = "dylib")]
unsafe extern "system" {
    fn GetStdHandle(handle: win::Handle) -> win::Handle;
    fn WriteFile(
        file_handle: win::Handle,
        buffer: *const u8,
        len: u32,
        bytes_written: *mut u32,
        overlapping: *mut (),
    ) -> i32;
    fn GetCurrentProcessId() -> u32;
    fn OpenProcess(desired_acces: u32, inherit_handle: i32, process_id: u32) -> win::Handle;
    fn TerminateProcess(handle: win::Handle, exit_code: u32) -> i32;
}

#[cold]
#[inline(never)]
fn handle_duplicate_typeid(
    type_id: TypeId,
    #[cfg(feature = "debug_type_name")] type_name1: &str,
    #[cfg(feature = "debug_type_name")] type_name2: &str,
) -> ! {
    let hex_val = hex::HexView::new(type_id.as_u32());
    // Safety: well, we just call libc or WinAPI functions.
    // This code runs before main so we cannot run code from stdlib so we can't really synchronize access to stderr.
    // It probably the only running thread in application.
    // Anyway, this function ends by terminates current process so any memory unsafety would end here.
    unsafe {
        #[cfg(unix)]
        let stderr: *mut unix::File = fdopen(unix::STDERR_FILENO, b"a\0".as_ptr());
        #[cfg(windows)]
        let stderr: win::Handle = GetStdHandle(win::STD_ERROR_HANDLE);
        #[cfg(unix)]
        let eprint_str = |s: &str| {
            fwrite(s.as_ptr(), 1, s.len(), stderr);
        };
        #[cfg(windows)]
        let eprint_str = |s: &str| {
            WriteFile(
                stderr,
                s.as_ptr(),
                s.len().try_into().unwrap(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
        };

        eprint_str("short_type_id: Found duplicate type_id ");
        eprint_str(hex_val.as_str());
        #[cfg(not(feature = "debug_type_name"))]
        {
            eprint_str(
                r#". Consider enabling "debug_type_name" feature to display conflicting type names"#,
            );
        }
        #[cfg(feature = "debug_type_name")]
        {
            eprint_str(" for types ");
            eprint_str(type_name1);
            eprint_str(" and ");
            eprint_str(type_name2);
        }
        eprint_str(".\n");

        #[cfg(unix)]
        {
            fflush(stderr);
            abort();
        }
        #[cfg(windows)]
        {
            let current_process_id = GetCurrentProcessId();
            let current_process_handle = OpenProcess(
                win::PROCESS_TERMINATE_ACCESS,
                false.into(),
                current_process_id,
            );
            TerminateProcess(current_process_handle, 2);
            unreachable!();
        }
    }
}

#[cfg(test)]
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
            murmur_v3(b"short_type_id::implementation::tests::MyType", MURMUR_SEED),
            0x6FFDD6CA
        );
        assert_eq!(
            murmur_v3(
                b"short_type_id::implementation::tests::MyType::0.1.1",
                MURMUR_SEED
            ),
            0x611E8FFD
        );
        assert_eq!(MY_HASH.as_u32(), 0x6FFDD6CA);
        assert_eq!(MY_HASH_AND_CRATE.as_u32(), 0x611E8FFD);
        assert_eq!(MY_HASH_AND_NO_CRATE.as_u32(), 0x6FFDD6CA);
    }

    #[test]
    fn murmur() {
        assert_eq!(murmur_v3("test".as_bytes(), 0), 0xba6bd213);
        assert_eq!(murmur_v3("test".as_bytes(), 0x9747b28c), 0x704b81dc);
        assert_eq!(murmur_v3("Hello, world!".as_bytes(), 0), 0xc0363e43);
        assert_eq!(
            murmur_v3("Hello, world!".as_bytes(), 0x9747b28c),
            0x24884cba
        );
        assert_eq!(
            murmur_v3("The quick brown fox jumps over the lazy dog".as_bytes(), 0),
            0x2e4ff723
        );
        assert_eq!(
            murmur_v3(
                "The quick brown fox jumps over the lazy dog".as_bytes(),
                0x9747b28c
            ),
            0x2fa826cd
        );
        assert_eq!(
            murmur_v3(b"assaulted", MURMUR_SEED),
            murmur_v3(b"nonescape", MURMUR_SEED)
        );
        // Same as test in extra_tests/duplicate_type_ids_handling
        assert_eq!(
            murmur_v3(b"duplicate_type_ids_handling::uaaaaa58::0.0.0", MURMUR_SEED),
            murmur_v3(b"duplicate_type_ids_handling::iaaaac3b::0.0.0", MURMUR_SEED),
        )
    }

    #[test]
    fn compute_id() {
        use private::compute_id;

        let _my_hash = MY_HASH;

        // This would be used for testing duplicate lookup in types.
        assert_eq!(compute_id(b"assaulted"), compute_id(b"nonescape"));
        assert_eq!(compute_id(b"assaulted").as_u32(), 0x3BD11B2D);

        assert_eq!(compute_id(b"usize").as_u32(), 0x3CAC743E);

        // Check that we do not generate zeros.
        assert_eq!(murmur_v3(b"sascmxrw", MURMUR_SEED), 0);
        assert_ne!(compute_id(b"sascmxrw").as_u32(), 0);
        assert_eq!(compute_id(b"sascmxrw").as_u32(), 1);

        assert_eq!(u32::MAX >> 31, 1);
        assert_eq!(compute_id(b"assaulted").as_u32() >> 31, 0);
        assert_eq!(compute_id(b"usize").as_u32() >> 31, 0);
        assert_eq!(compute_id(b"sascmxrw").as_u32() >> 31, 0);
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
