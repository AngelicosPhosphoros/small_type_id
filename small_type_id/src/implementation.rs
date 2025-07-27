use core::num::NonZeroU32;

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

    #[repr(C)]
    #[cfg_attr(
        any(
            feature = "dont_use_link_section",
            not(any(target_os = "windows", target_os = "linux"))
        ),
        derive(Copy, Clone)
    )]
    pub struct TypeEntry {
        pub(super) type_id: TypeId,
        #[cfg(feature = "debug_type_name")]
        pub(super) type_name: &'static str,
    }

    impl TypeEntry {
        #[must_use]
        pub const fn new(type_name: &'static str, type_id: TypeId) -> TypeEntry {
            // To suppress warning if name is not used.
            let _ = type_name;

            Self {
                type_id,
                #[cfg(feature = "debug_type_name")]
                type_name,
            }
        }
    }

    #[cfg(not(feature = "unsafe_dont_register_types"))]
    #[cfg(any(
        feature = "dont_use_link_section",
        not(any(target_os = "windows", target_os = "linux"))
    ))]
    pub use with_ctors_per_entry::LinkedNode;

    /// # SAFETY
    /// Must be called exactly once per entry during initialization sequence.
    #[cold]
    #[cfg(any(
        feature = "dont_use_link_section",
        not(any(target_os = "windows", target_os = "linux"))
    ))]
    #[cfg(not(feature = "unsafe_dont_register_types"))]
    pub unsafe fn register_type(entry: &'static with_ctors_per_entry::LinkedNode) {
        unsafe {
            with_ctors_per_entry::register_type(entry);
        }
    }

    /// Use [`compute_input_len`] to compute `TOTAL_INPUT_LEN`.
    #[must_use]
    pub const fn compute_id<const TOTAL_INPUT_LEN: usize>(
        module_and_name: &str,
        crate_version: Option<&str>,
        seed: u32,
    ) -> TypeId {
        let hash = if let Some(crate_version) = crate_version {
            assert!(TOTAL_INPUT_LEN == module_and_name.len() + crate_version.len() + 2);
            let mut concatenated = [0; TOTAL_INPUT_LEN];
            // Need to use split_at_mut because slice[x..y] syntax doesn't work in const fns.
            let (head, tail) = concatenated.split_at_mut(module_and_name.len());
            let (delim, version) = tail.split_at_mut(2);

            head.copy_from_slice(module_and_name.as_bytes());
            delim.copy_from_slice(b"::");
            version.copy_from_slice(crate_version.as_bytes());

            xxh32(&concatenated, seed)
        } else {
            assert!(TOTAL_INPUT_LEN == module_and_name.len());
            xxh32(module_and_name.as_bytes(), seed)
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
    pub use crate::private_macro_link_section_name as link_section_name;
    pub use crate::private_macro_register_type_id as register_type_id;
    pub use crate::private_macro_small_type_id_version as small_type_id_version;
}

pub(crate) fn pub_iter_registered_types() -> impl Iterator<Item = crate::TypeEntry> {
    #[cfg(feature = "unsafe_dont_register_types")]
    {
        core::iter::empty()
    }
    #[cfg(not(feature = "unsafe_dont_register_types"))]
    {
        #[cfg(any(
            feature = "dont_use_link_section",
            not(any(target_os = "windows", target_os = "linux"))
        ))]
        let refs = with_ctors_per_entry::iter_registered_types();
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        #[cfg(not(feature = "dont_use_link_section"))]
        let refs = with_link_section::iter_registered_types();

        refs.map(|e| crate::TypeEntry {
            type_id: e.type_id,
            #[cfg(feature = "debug_type_name")]
            debug_type_name: e.type_name,
        })
    }
}

#[cfg(not(feature = "dont_use_link_section"))]
#[cfg(not(feature = "unsafe_dont_register_types"))]
#[cfg(any(target_os = "windows", target_os = "linux"))]
mod with_link_section {
    use core::hint::black_box;
    use core::mem::MaybeUninit;

    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub(super) fn iter_registered_types()
    -> impl Iterator<Item = &'static private::TypeEntry> + Clone {
        // Use MaybeUninit because elements in a section may be zeroed padding.
        #[cfg(target_os = "windows")]
        let (start_ptr, end_ptr): (
            *const MaybeUninit<private::TypeEntry>,
            *const MaybeUninit<private::TypeEntry>,
        ) = {
            // We use 1 item for the first element to avoid linking errors
            // if there is no entries available.
            #[unsafe(link_section=concat!("smltidrs_small_type_id_rs$", private::small_type_id_version!(), "_a"))]
            #[used]
            static START: [MaybeUninit<private::TypeEntry>; 1] = [MaybeUninit::zeroed()];
            #[unsafe(link_section=concat!("smltidrs_small_type_id_rs$", private::small_type_id_version!(), "_c"))]
            #[used]
            static STOP: [MaybeUninit<private::TypeEntry>; 0] = [];

            // Use black_box to prevent provenance based code eliminations.
            let start_ptr = unsafe { black_box(START.as_ptr().add(1)) };
            let end_ptr = black_box(STOP.as_ptr());

            (start_ptr, end_ptr)
        };
        #[cfg(target_os = "linux")]
        let (start_ptr, end_ptr): (
            *const MaybeUninit<private::TypeEntry>,
            *const MaybeUninit<private::TypeEntry>,
        ) = {
            #[unsafe(link_section = private::link_section_name!())]
            #[used]
            static AVOID_REMOVAL: MaybeUninit<private::TypeEntry> = MaybeUninit::zeroed();
            unsafe extern "Rust" {
                #[link_name = concat!("__start_smltidrs_small_type_id_rs", private::small_type_id_version!())]
                //#[link_name = concat!("__start_smltidrs_small_type_id_rs")]
                static START: MaybeUninit<private::TypeEntry>;
                #[link_name = concat!("__stop_smltidrs_small_type_id_rs", private::small_type_id_version!())]
                //#[link_name = concat!("__stop_smltidrs_small_type_id_rs")]
                static STOP: MaybeUninit<private::TypeEntry>;
            }
            // Use black_box to prevent provenance based code eliminations.
            let start_ptr = black_box(&raw const START);
            let end_ptr = black_box(&raw const STOP);

            (start_ptr, end_ptr)
        };

        // SAFETY: We can assume that all entries in link section are ours
        // because our link section name is very specific (and sorting order includes our version)
        // so other entries may appear only if someone insert them deliberately and using unsafe.
        // We cannot really defend against this.
        unsafe {
            let mut it: *const u8 = start_ptr.cast();
            core::iter::from_fn(move || {
                // Skip padding that may be generated by incremental linker.
                // See https://devblogs.microsoft.com/oldnewthing/20181109-00/?p=100175
                // https://devblogs.microsoft.com/oldnewthing/20190114-00/?p=100695
                while it != end_ptr.cast() {
                    let first_val: u32 = *it.cast();
                    if first_val != 0 {
                        break;
                    }
                    // This is a padding.
                    it = it.add(align_of::<private::TypeEntry>());
                }
                if it == end_ptr.cast() {
                    None
                } else {
                    let p: *const private::TypeEntry = it.cast();
                    it = it.add(size_of::<private::TypeEntry>());
                    Some(&*p)
                }
            })
        }
    }

    #[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
    fn check_registered_entries() {
        let mut buffer = [0_u32; 2048];
        let mut it = iter_registered_types();
        loop {
            #[cfg(feature = "debug_type_name")]
            let original_start = it.clone();

            let mut len = 0;
            for entry in it.by_ref() {
                buffer[len] = entry.type_id.as_u32();
                len += 1;
                if len == buffer.len() {
                    break;
                }
            }
            if len == 0 {
                break;
            }
            let known_types = &mut buffer[..len];

            known_types.sort_unstable();

            for w in known_types.windows(2) {
                let [a, b] = w[..] else { unreachable!() };
                if a == b {
                    let tid = TypeId(NonZeroU32::new(a).unwrap());
                    handle_duplicate_typeid(
                        tid,
                        #[cfg(feature = "debug_type_name")]
                        original_start,
                    );
                }
            }

            for t in it.clone() {
                let inner = t.type_id.as_u32();
                if known_types.binary_search(&inner).is_ok() {
                    handle_duplicate_typeid(
                        t.type_id,
                        #[cfg(feature = "debug_type_name")]
                        original_start,
                    );
                }
            }
        }
    }

    // Note `check_registered_entries` is not inside
    // because `cargo fmt` often fails with declarative macro invokations.
    #[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
    ctor::declarative::ctor! {
        #[ctor]
        unsafe fn check_registered_entries_(){
            check_registered_entries();
        }
    }
}

#[cfg(not(feature = "unsafe_dont_register_types"))]
#[cfg(any(
    feature = "dont_use_link_section",
    not(any(target_os = "windows", target_os = "linux"))
))]
mod with_ctors_per_entry {
    use core::cell::UnsafeCell;
    use core::sync::atomic::AtomicBool;
    use core::sync::atomic::Ordering::{Acquire, Release};

    use crate::skip_list::{InsertResult, SkipList, SkipListNode};

    #[allow(clippy::wildcard_imports)]
    use super::*;

    pub struct LinkedNode(UnsafeCell<SkipListNode<private::TypeEntry, 10>>);

    impl LinkedNode {
        #[must_use]
        pub const fn new(e: private::TypeEntry) -> Self {
            Self(UnsafeCell::new(SkipListNode::new(e)))
        }
    }

    // SAFETY: This type must be used only by private macros,
    // if they are correct. Values are modified only during init sequence
    // so they are updated only in a single thread.
    unsafe impl Sync for LinkedNode {}

    struct SkipListShared {
        list: UnsafeCell<SkipList<'static, private::TypeEntry, 10>>,
    }

    // SAFETY:
    // 1. All modifications happens only during initialization sequence.
    // 2. We additionally assert that modification runs in unique thread using atomic flag.
    unsafe impl Sync for SkipListShared {}

    static TYPE_ENTRIES: SkipListShared = SkipListShared {
        list: UnsafeCell::new(SkipList::new()),
    };
    static IS_CURRENTLY_BEING_INSERTED: AtomicBool = AtomicBool::new(false);

    pub(super) fn iter_registered_types() -> impl Iterator<Item = &'static private::TypeEntry> {
        assert!(!IS_CURRENTLY_BEING_INSERTED.load(Acquire));
        // SAFETY: Modifications should happen only during initialization sequence.
        // This method is not called when we inserting new type entry.
        // Entries in the list doesn't moved or edited when new entry is inserted
        // so resulting references are safe to access.
        unsafe { (*TYPE_ENTRIES.list.get()).iter() }
    }

    pub(super) unsafe fn register_type(entry: &'static LinkedNode) {
        struct ModifyGuard {}
        impl Drop for ModifyGuard {
            fn drop(&mut self) {
                IS_CURRENTLY_BEING_INSERTED.store(false, Release);
            }
        }

        assert!(!IS_CURRENTLY_BEING_INSERTED.swap(true, Acquire));
        let _modify_guard = ModifyGuard {};

        let list = unsafe { &mut *TYPE_ENTRIES.list.get() };

        // SAFETY: By safety contract of function.
        let entry: &mut SkipListNode<_, 10> = unsafe { &mut *entry.0.get() };
        let val = *entry.get_value();
        let insertion_res = list.insert(entry);

        match insertion_res {
            InsertResult::Unique => return,
            InsertResult::Duplicate(dup) => {
                debug_assert_eq!(val.type_id, dup.type_id);
                #[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
                handle_duplicate_typeid(
                    dup.type_id,
                    #[cfg(feature = "debug_type_name")]
                    [&dup, &val],
                )
            }
        };
    }

    impl Ord for private::TypeEntry {
        #[inline]
        fn cmp(&self, other: &Self) -> core::cmp::Ordering {
            self.type_id.cmp(&other.type_id)
        }
    }
    impl PartialOrd for private::TypeEntry {
        #[inline]
        fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }
    impl PartialEq for private::TypeEntry {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.cmp(other) == core::cmp::Ordering::Equal
        }
    }
    impl Eq for private::TypeEntry {}
}

#[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
#[cfg_attr(windows, path = "win.rs")]
#[cfg_attr(unix, path = "unix.rs")]
mod platform;

#[cfg_attr(
    not(feature = "debug_type_name"),
    allow(clippy::extra_unused_lifetimes)
)]
#[cfg(not(feature = "unsafe_remove_duplicate_checks"))]
#[cold]
#[inline(never)]
fn handle_duplicate_typeid<'a>(
    type_id: TypeId,
    #[cfg(feature = "debug_type_name")] iter_types: impl IntoIterator<Item = &'a private::TypeEntry>,
) -> ! {
    let hex_val = hex::HexView::new(type_id.as_u32());

    #[cfg(feature = "debug_type_name")]
    let (e0, e1) = {
        let mut iter_types = iter_types.into_iter().filter(|x| x.type_id == type_id);
        let e0 = iter_types.next().unwrap();
        let e1 = iter_types.next().unwrap();
        // We order this 2 entries for ease of testing.
        if e0.type_name <= e1.type_name {
            (e0, e1)
        } else {
            (e1, e0)
        }
    };

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
            platform::print_error(&mut stderr, e0.type_name);
            platform::print_error(&mut stderr, " and ");
            platform::print_error(&mut stderr, e1.type_name);
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
        private::compute_id::<INPUT_LEN>(concat!(module_path!(), "::", "MyType"), Some("0.1.1"), 0)
    };
    #[allow(unused)]
    const MY_HASH_AND_NO_CRATE: TypeId = {
        const INPUT_LEN: usize =
            private::compute_input_len(concat!(module_path!(), "::", "MyType"), None);
        private::compute_id::<INPUT_LEN>(concat!(module_path!(), "::", "MyType"), None, 0)
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
        );
    }

    #[test]
    fn compute_id() {
        use private::compute_id;

        // This would be used for testing duplicate lookup in types.
        assert_eq!(
            compute_id::<7>("hogtied", None, 0),
            compute_id::<10>("scouriness", None, 0)
        );
        assert_eq!(compute_id::<7>("hogtied", None, 0).as_u32(), 0x5034ABE3);

        assert_eq!(compute_id::<5>("usize", None, 0).as_u32(), 0x7847CC2E);
        assert_eq!(
            compute_id::<12>("usize", Some("0.0.0"), 0).as_u32(),
            0x3064D6AA
        );
        assert_eq!(
            compute_id::<12>("usize", Some("0.0.1"), 0).as_u32(),
            0x791B53F2
        );

        // Check that we do not generate zeros.
        assert_eq!(xxh32(b"AasZkWq", 0), 0);
        assert_eq!(xxh32(b"RalEB24", 0), 0);
        assert_ne!(compute_id::<7>("AasZkWq", None, 0).as_u32(), 0);
        assert_ne!(compute_id::<7>("RalEB24", None, 0).as_u32(), 0);
        assert_eq!(compute_id::<7>("AasZkWq", None, 0).as_u32(), 1);
        assert_eq!(compute_id::<7>("RalEB24", None, 0).as_u32(), 1);

        assert_ne!(
            compute_id::<7>("AasZkWq", None, 1),
            compute_id::<7>("RalEB24", None, 0)
        );

        assert_eq!(u32::MAX >> 31, 1);
        assert_eq!(compute_id::<9>("assaulted", None, 0).as_u32() >> 31, 0);
        assert_eq!(compute_id::<5>("usize", None, 0).as_u32() >> 31, 0);
        assert_eq!(compute_id::<7>("AasZkWq", None, 0).as_u32() >> 31, 0);
        assert_eq!(compute_id::<7>("RalEB24", None, 0).as_u32() >> 31, 0);
    }

    #[test]
    #[should_panic]
    fn compute_id_invalid_len_none() {
        use private::compute_id;
        let _ = compute_id::<6>("hogtied", None, 0);
    }

    #[test]
    #[should_panic]
    fn compute_id_invalid_len_some() {
        use private::compute_id;
        let _ = compute_id::<7>("hogtied", Some("a.b.c"), 0);
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
