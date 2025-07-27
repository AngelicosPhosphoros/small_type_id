use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::ptr::{NonNull, null_mut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Release};

// This is an rng implementation I added
// because it is impossible to add conditional dependency to Cargo
// if any of 3 conditions (not os, feature or dev-dep) is true.
// This implementation based on Xoshiro256PlusPlus.
// This is not cryptographically secure algorithm but it fits
// our purpose or computing levels of SkipList.
fn random_bool() -> bool {
    // Code is copied from rand 0.9.2.
    fn xoshiro_256_plus_plus(s: &mut [u64; 4]) -> u64 {
        let res = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;

        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];

        s[2] ^= t;
        s[3] = s[3].rotate_left(45);

        res
    }

    struct State {
        // First 2 fields are used to return bit values
        // of last xoshiro output as bools.
        last: u64,
        unchecked_bits: u32,
        xorshiro: [u64; 4],
    }

    fn bool_from_bits(state: &mut State) -> bool {
        let State {
            last,
            unchecked_bits,
            xorshiro,
        } = state;
        if *unchecked_bits == 0 {
            *last = xoshiro_256_plus_plus(xorshiro);
            *unchecked_bits = u64::BITS;
        }
        let bit_value: bool = *last & 1 != 0;
        *unchecked_bits -= 1;
        *last >>= 1;

        bit_value
    }

    struct ShareableState(UnsafeCell<State>);
    // SAFETY: We guard static variable using `IS_LOCKED`.
    unsafe impl Sync for ShareableState {}

    // It is actually useless because this function is intented
    // to be called only from a single thread.
    static IS_LOCKED: AtomicBool = AtomicBool::new(false);
    struct UnlockOnDrop {}
    impl Drop for UnlockOnDrop {
        #[inline]
        fn drop(&mut self) {
            IS_LOCKED.store(false, Release);
        }
    }

    static STATE: ShareableState = ShareableState(UnsafeCell::new(State {
        last: 0,
        unchecked_bits: 0,
        // Since we use them only for determining height of our skiplist nodes
        // and there is no vector of attack to those nodes, we can put anything here
        // but zeros.
        #[allow(clippy::unreadable_literal)]
        xorshiro: [2847431818, 269649606, 2222596673, 2339704385],
    }));

    while IS_LOCKED.swap(true, Acquire) {
        core::hint::spin_loop();
    }
    let _unlocker = UnlockOnDrop {};

    bool_from_bits(unsafe { &mut *STATE.0.get() })
}

#[repr(C)] // repr C ensures that value and nexts[0] are close.
pub(crate) struct SkipListNode<T, const HEIGHT: usize> {
    // Option because we cannot initialize prehead.
    // For all actual values it is guaranteed to be `Some`.
    value: T,
    level: u8,
    /// 0 is lowest level (where every entry is linked),
    /// HEIGH-1 is biggest level (where only very rare linkage).
    /// This should make iterating over
    nexts: [*mut SkipListNode<T, HEIGHT>; HEIGHT],
}

/// It is intrusive skiplist.
pub(crate) struct SkipList<'element, T, const HEIGHT: usize> {
    // It points to first element but doesn't contain any.
    prehead: [*mut SkipListNode<T, HEIGHT>; HEIGHT],
    // This enables borrow checker to know that we borrow our nodes.
    _marker: PhantomData<&'element mut T>,
}

pub(crate) enum InsertResult<T> {
    /// Means that there weren't any other entry with same value.
    Unique,
    /// Reference to older duplicate entry.
    /// New entry still was inserted.
    Duplicate(T),
}

impl<T, const HEIGHT: usize> SkipListNode<T, HEIGHT> {
    #[must_use]
    pub(crate) const fn new(value: T) -> Self {
        const { assert!(HEIGHT > 0, "Must be nonzero") };
        Self {
            value,
            level: 0,
            nexts: [null_mut(); HEIGHT],
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn get_value(&self) -> &T {
        &self.value
    }
}

impl<T, const HEIGHT: usize> SkipList<'_, T, HEIGHT> {
    pub(crate) const fn new() -> Self {
        Self {
            prehead: [null_mut(); HEIGHT],
            _marker: PhantomData,
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        let mut p = self.prehead[0];
        core::iter::from_fn(move || unsafe {
            if p.is_null() {
                None
            } else {
                let current = &*p;
                p = current.nexts[0];
                Some(&current.value)
            }
        })
    }
}

impl<'element, T, const HEIGHT: usize> SkipList<'element, T, HEIGHT>
where
    T: Ord + Copy,
{
    // Note that entries cannot be borrowed after being inserted
    // because they are borrowed mutably.
    // This allows this function to not be unsafe.
    pub(crate) fn insert(
        &mut self,
        entry: &'element mut SkipListNode<T, HEIGHT>,
    ) -> InsertResult<T> {
        debug_assert_eq!(
            entry.nexts,
            [null_mut(); HEIGHT],
            "Must be inserted only once and only to one skiplist"
        );
        debug_assert_eq!(entry.level, 0);

        entry.level = determine_level(HEIGHT - 1);

        let mut prev_val: Option<T> = None;
        // Pointers to values that are <= than entry.
        let mut prevs: [NonNull<*mut SkipListNode<T, HEIGHT>>; HEIGHT] =
            core::array::from_fn(unsafe {
                let start = self.prehead.as_mut_ptr();
                move |i| NonNull::new(start.add(i)).unwrap()
            });
        let mut current_place: NonNull<*mut SkipListNode<T, HEIGHT>> = prevs[HEIGHT - 1];
        for level in (0..HEIGHT).rev() {
            // SAFETY: Pointers in list can be added only using `insert` call so it must be valid.
            // Unique borrow of every `entry` arg is enforced by borrow checker.
            unsafe {
                loop {
                    let nxt = *current_place.as_ptr();
                    if nxt.is_null() {
                        break;
                    }
                    if (*nxt).value > entry.value {
                        break;
                    }
                    prev_val = Some((*nxt).value);
                    current_place = NonNull::new((*nxt).nexts.as_mut_ptr().add(level)).unwrap();
                }
                prevs[level] = current_place;
                if level > 0 {
                    current_place = current_place.sub(1);
                }
            }
        }

        let res = if prev_val == Some(entry.value) {
            InsertResult::Duplicate(prev_val.unwrap())
        } else {
            InsertResult::Unique
        };

        entry.nexts = unsafe { prevs.map(|x| *x.as_ptr()) };
        let max_lvl: usize = entry.level.into();
        let p: *mut _ = entry;
        #[allow(clippy::needless_range_loop)] // Range loops are often better optimized.
        for level in 0..HEIGHT {
            if level > max_lvl {
                break;
            }
            unsafe {
                *prevs[level].as_ptr() = p;
            }
        }
        res
    }
}

fn determine_level(max: usize) -> u8 {
    debug_assert!(usize::from(u8::MAX) > max);
    let mut level = 0;
    while usize::from(level) < max && random_bool() {
        level += 1;
    }
    level
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fmt::Write as _;

    use super::*;

    /// <https://en.wikipedia.org/wiki/Heap%27s_algorithm/>
    /// It works on arrays because arrays are faster when testing with MIRI.
    fn visit_permutations<T, const N: usize>(vals: &mut [T; N], mut visitor: impl FnMut(&[T; N])) {
        fn heaps_algorithm<T, const N: usize>(
            k: usize,
            vals: &mut [T; N],
            visitor: &mut impl FnMut(&[T; N]),
        ) {
            if k == 1 {
                visitor(vals);
                return;
            }
            heaps_algorithm(k - 1, vals, visitor);
            for i in 0..k - 1 {
                let left = if k & 1 == 0 { i } else { 0 };
                vals.swap(left, k - 1);
                heaps_algorithm(k - 1, vals, visitor);
            }
        }
        if vals.is_empty() {
            visitor(vals);
            return;
        }
        heaps_algorithm(vals.len(), vals, &mut visitor);
    }

    fn print_skiplist<const HEIGHT: usize>(skiplist: &SkipList<u32, HEIGHT>) -> String {
        let node2pos: HashMap<*mut SkipListNode<u32, HEIGHT>, usize> = {
            let mut n = HashMap::new();
            let mut it = skiplist.prehead[0];
            let mut i = 0;
            while !it.is_null() {
                n.insert(it, i);
                it = unsafe { (*it).nexts[0] };
                i += 1;
            }
            n
        };
        let mut res = String::new();
        for level in (0..HEIGHT).rev() {
            let mut it = skiplist.prehead[level];
            if it.is_null() {
                continue;
            }
            let delim = "->";
            write!(&mut res, "{:02}: ", level).unwrap();
            let mut curr_pos: isize = -1;
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
            while !it.is_null() {
                let pos = node2pos[&it];
                let extend_len = (pos as isize - curr_pos - 1) as usize;
                res.extend(std::iter::repeat_n('-', (delim.len() + 2) * extend_len));
                write!(&mut res, "{}{:2}", delim, unsafe { (*it).value }).unwrap();
                it = unsafe { (*it).nexts[level] };
                curr_pos = pos as isize;
            }
            res.push('\n');
        }
        res.pop();
        res
    }

    fn is_sorted<T: Ord, const HEIGHT: usize>(skiplist: &SkipList<T, HEIGHT>) -> bool {
        let mut prev = None;
        for v in skiplist.iter() {
            if prev > Some(v) {
                return false;
            }
            prev = Some(v);
        }
        true
    }

    #[test]
    fn validate_heaps_algorithm() {
        fn factorial(n: usize) -> usize {
            (2..=n).fold(1, std::ops::Mul::mul)
        }
        // Miri is faster if we work with arrays.
        fn make_set<const N: usize>() -> HashSet<[u32; N]> {
            let mut values: [u32; N] = std::array::from_fn(|x| x.try_into().unwrap());
            let f = factorial(N);
            let mut set = HashSet::with_capacity(f + f / 4);
            visit_permutations(&mut values, |s| {
                set.insert(*s);
            });
            set
        }

        assert_eq!(make_set::<0>().len(), 1);
        assert_eq!(make_set::<1>().len(), 1);
        assert_eq!(make_set::<6>().len(), factorial(6));
        if !cfg!(miri) {
            // Those are incredibly slow under MIRI.
            std::thread::scope(|s| {
                s.spawn(|| assert_eq!(make_set::<7>().len(), factorial(7)));
                s.spawn(|| assert_eq!(make_set::<9>().len(), factorial(9)));
            });
        }
    }

    #[test]
    fn test_sorting_permutations() {
        let mut vals = [0, 1, 2, 3, 4];
        visit_permutations(&mut vals, |perm| {
            let mut nodes: Vec<SkipListNode<u32, 3>> =
                perm.iter().copied().map(SkipListNode::new).collect();
            let mut list = SkipList::new();
            for node in nodes.iter_mut() {
                list.insert(node);
            }
            assert!(is_sorted(&list));

            let mut prev_count = nodes.len();
            for level in 1..3 {
                let c = nodes.iter().filter(|x| x.level >= level).count();
                assert!(c <= prev_count);
                prev_count = c;
            }
        });
    }

    #[test]
    fn test_sorting_large() {
        {
            let nums = [
                9, 16, 13, 0, 6, 10, 14, 1, 4, 15, 17, 3, 18, 19, 5, 2, 12, 7, 8, 11,
            ];
            let mut nodes = nums.map(SkipListNode::new);
            let mut list: SkipList<u32, 4> = SkipList::new();
            for node in nodes.iter_mut() {
                list.insert(node);
            }
            let s = print_skiplist(&list);
            eprintln!("{}", s);
            // This can be flaky because rng is global.
            // assert_eq!(s,
            //     "03: -----> 1----------------------------------------->12------------------------->19\n".to_string() +
            //     "02: -----> 1----------------------------------------->12------------------------->19\n" +
            //     "01: -----> 1---------------------> 7--------->10----->12->13----->15->16--------->19\n"+
            //     "00: -> 0-> 1-> 2-> 3-> 4-> 5-> 6-> 7-> 8-> 9->10->11->12->13->14->15->16->17->18->19"
            // );
            let mut prev_count = nodes.len();
            for level in 1..4 {
                let c = nodes.iter().filter(|x| x.level >= level).count();
                assert!(c <= prev_count);
                prev_count = c;
            }
        }
        {
            let nums = [
                17, 6, 24, 47, 11, 19, 42, 13, 8, 18, 5, 12, 35, 1, 32, 23, 36, 33, 37, 43, 48, 25,
                14, 15, 26, 39, 0, 20, 16, 27, 45, 21, 10, 30, 49, 28, 3, 41, 29, 7, 2, 4, 38, 44,
                46, 34, 22, 9, 31, 40,
            ];
            let mut nodes = nums.map(SkipListNode::new);
            let mut list: SkipList<u32, 4> = SkipList::new();
            for node in nodes.iter_mut() {
                list.insert(node);
            }
            assert!(is_sorted(&list));

            let mut prev_count = nodes.len();
            for level in 1..4 {
                let c = nodes.iter().filter(|x| x.level >= level).count();
                assert!(c <= prev_count);
                prev_count = c;
            }
        }
    }

    #[test]
    fn test_rng() {
        const NUM_TESTS: usize = 400;
        let num_trues = (0..NUM_TESTS).filter(|_| random_bool()).count();
        assert!(100 < num_trues);
        assert!(num_trues < 300);
    }
}
