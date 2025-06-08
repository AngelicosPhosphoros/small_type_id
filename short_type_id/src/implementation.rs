use core::num::NonZeroU32;
use core::ptr;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed};

use crate::{hex, TypeId};

#[doc(hidden)]
pub mod private {
    use super::*;

    pub struct TypeEntry {
        pub(super) type_id: TypeId,
        #[cfg(feature = "debug_type_name")]
        pub(super) type_name: &'static str,
        pub(super) next: AtomicPtr<TypeEntry>,
    }

    impl TypeEntry {
        pub const fn new(module_and_name: &'static str) -> TypeEntry {
            Self {
                type_id: compute_id(module_and_name),
                #[cfg(feature = "debug_type_name")]
                type_name: module_and_name,
                next: AtomicPtr::new(ptr::null_mut()),
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
            let p = entry as *const TypeEntry as *mut _;
            match LAST_ADDED_TYPE.compare_exchange_weak(next, p, AcqRel, Relaxed) {
                Ok(_) => break,
                Err(p) => next = p,
            }
        }

        // This code tests that we don't have registered any duplicates.
        // Unfortunately, it runs in quadratic time.
        #[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
        unsafe {
            let start = LAST_ADDED_TYPE.load(Acquire);
            let mut it_slow = start;
            while it_slow.is_null() {
                let typeid = (*it_slow).type_id;
                let mut it_fast = (*it_slow).next.load(Relaxed);
                while it_fast.is_null() {
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

    pub const fn compute_id(name_with_module_path: &str) -> TypeId {
        let hash = murmur_v3(name_with_module_path.as_bytes(), MURMUR_SEED);
        let val = if hash == 0 { 1 } else { hash } & 0x7FFF_FFFF_u32;
        TypeId(NonZeroU32::new(val).unwrap())
    }
}

const MURMUR_SEED: u32 = 0xF1D4B28B;
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
        h = h.wrapping_mul(5).wrapping_add(0xE6546B64);
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

    let len = src.len() as u32;
    h ^= len;
    h ^= h >> 16;
    h = u32::wrapping_mul(h, 0x85ebca6b);
    h ^= h >> 13;
    h = u32::wrapping_mul(h, 0xc2b2ae35);
    h ^= h >> 16;

    h
}

const fn murmur_32_scramble(k: u32) -> u32 {
    k.wrapping_mul(0xcc9e2d51)
        .rotate_left(15)
        .wrapping_mul(0x1b873593)
}

#[cfg(all(not(feature = "unsafe_remove_duplicate_checks"), unix))]
mod unix {
    #[repr(C)]
    pub(super) struct File(core::ffi::c_void);
    pub(super) const STDERR_FILENO: i32 = 2;
}
#[cfg(all(not(feature = "unsafe_remove_duplicate_checks"), unix))]
extern "C" {
    fn fdopen(fd: i32, mode: *const u8) -> *mut unix::File;
    fn fwrite(buffer: *const u8, elem_size: usize, len: usize, file: *mut unix::File) -> usize;
    fn fflush(file: *mut unix::File) -> i32;
    fn abort() -> !;
}

#[cfg(windows)]
mod win {
    pub(super) type Handle = u32;
    pub(super) const STD_ERROR_HANDLE: Handle = 4294967284u32;
    pub(super) const PROCESS_TERMINATE_ACCESS: u32 = 1;
}
#[cfg(windows)]
#[link(name = "Kernel32", kind = "dylib")]
extern "system" {
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
                s.len() as _,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
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
                false as _,
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

    // This checks that we can compute const hash in compile time.
    #[allow(unused)]
    const MY_HASH: TypeId = private::compute_id(concat!(module_path!(), "::", "MyType"));

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
    }

    #[test]
    fn compute_id() {
        use private::compute_id;

        let _my_hash = MY_HASH;

        // This would be used for testing duplicate lookup in types.
        assert_eq!(compute_id("assaulted"), compute_id("nonescape"));
        assert_eq!(compute_id("assaulted").as_u32(), 0x3BD11B2D);

        assert_eq!(compute_id("usize").as_u32(), 0x3CAC743E);

        // Check that we do not generate zeros.
        assert_eq!(murmur_v3(b"sascmxrw", MURMUR_SEED), 0);
        assert_ne!(compute_id("sascmxrw").as_u32(), 0);
        assert_eq!(compute_id("sascmxrw").as_u32(), 1);

        assert_eq!(u32::MAX >> 31, 1);
        assert_eq!(compute_id("assaulted").as_u32() >> 31, 0);
        assert_eq!(compute_id("usize").as_u32() >> 31, 0);
        assert_eq!(compute_id("sascmxrw").as_u32() >> 31, 0);
    }
}
