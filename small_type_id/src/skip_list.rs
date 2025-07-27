use core::marker::PhantomData;
use core::ptr::{NonNull, null_mut};

use rand::Rng;

#[repr(C)] // repr C ensures that value and nexts[0] are close.
pub struct SkipListNode<T, const HEIGHT: usize> {
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
    pub(crate) const fn new(value: T) -> Self {
        const { assert!(HEIGHT > 0, "Must be nonzero") };
        Self {
            value,
            level: 0,
            nexts: [null_mut(); HEIGHT],
        }
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
        rng: &mut impl Rng,
    ) -> InsertResult<T> {
        debug_assert_eq!(
            entry.nexts,
            [null_mut(); HEIGHT],
            "Must be inserted only once and only to one skiplist"
        );
        debug_assert_eq!(entry.level, 0);

        entry.level = determine_level(rng, HEIGHT - 1);

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

fn determine_level(rng: &mut impl Rng, max: usize) -> u8 {
    debug_assert!(usize::from(u8::MAX) > max);
    let mut level = 0;
    while level < max as u8 && rng.random_bool(0.5) {
        level += 1;
    }
    level
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fmt::Write as _;

    use rand::SeedableRng as _;
    use rand::rngs::SmallRng;

    use super::*;

    /// https://en.wikipedia.org/wiki/Heap%27s_algorithm
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
            while !it.is_null() {
                let pos = node2pos[&it];
                let extend_len = (pos as isize - curr_pos - 1) as usize;
                res.extend(std::iter::repeat('-').take((delim.len() + 2) * extend_len));
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
            prev = Some(v)
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
            let mut set = HashSet::with_capacity_and_hasher(f + f / 4, Default::default());
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
            let mut rng = SmallRng::seed_from_u64(64646997);
            let mut list = SkipList::new();
            for node in nodes.iter_mut() {
                list.insert(node, &mut rng);
            }
            assert!(is_sorted(&list));
            let mut prev = None;
            for &v in list.iter() {
                assert!(prev < Some(v), "{:?} >= {:?} ({:?})", prev, v, perm);
                prev = Some(v);
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
            let mut rng = SmallRng::seed_from_u64(64646997);
            for node in nodes.iter_mut() {
                list.insert(node, &mut rng);
            }
            let s = print_skiplist(&list);
            eprintln!("{}", s);
            assert_eq!(s,
                "03: -> 0-----> 2-> 3----------------------------->11----------------------------->19\n".to_string() +
                "02: -> 0-----> 2-> 3-------------> 7------------->11->12------------------------->19\n" +
                "01: -> 0-> 1-> 2-> 3-> 4---------> 7------------->11->12----->14----->16----->18->19\n" +
                "00: -> 0-> 1-> 2-> 3-> 4-> 5-> 6-> 7-> 8-> 9->10->11->12->13->14->15->16->17->18->19");
        }
        {
            let nums = [
                17, 6, 24, 47, 11, 19, 42, 13, 8, 18, 5, 12, 35, 1, 32, 23, 36, 33, 37, 43, 48, 25,
                14, 15, 26, 39, 0, 20, 16, 27, 45, 21, 10, 30, 49, 28, 3, 41, 29, 7, 2, 4, 38, 44,
                46, 34, 22, 9, 31, 40,
            ];
            let mut nodes = nums.map(SkipListNode::new);
            let mut list: SkipList<u32, 4> = SkipList::new();
            let mut rng = SmallRng::seed_from_u64(64646997);
            for node in nodes.iter_mut() {
                list.insert(node, &mut rng);
            }
            assert!(is_sorted(&list));
        }
    }
}
