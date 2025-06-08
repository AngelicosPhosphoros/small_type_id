use core::convert::{Into, TryInto};
use core::fmt::Write as _;
use core::num::NonZeroUsize;
use std::thread;
use std::time::Instant;

#[path = "../../short_type_id/src/hex.rs"]
mod hex;

fn main() {
    println!("Starting checking values in multi thread...");
    let start = Instant::now();
    visit_multi_thread();
    let elapsed = start.elapsed();
    println!(
        "Checking all u32 in multiple threads took {:.3} secs",
        elapsed.as_secs_f64()
    );
}

fn visit_multi_thread() {
    let num_cores = thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get() as u64;
    let chunk_size = (u64::try_from(u32::MAX).unwrap() + 1) / num_cores + 1;
    thread::scope(move |scope| {
        for i in 0..num_cores {
            let start: u64 = i * chunk_size;
            let end: u32 = start
                .saturating_add(chunk_size)
                .min(u32::MAX.into())
                .try_into()
                .unwrap();
            let start: u32 = start.try_into().unwrap();
            scope.spawn(move || check_range(start, end));
        }
    });
}

fn check_range(start: u32, end: u32) {
    let mut s = String::with_capacity(16);
    let mut i: u64 = start.into();
    let end: u64 = end.into();
    while i <= end {
        let v = i as u32;

        s.clear();
        write!(&mut s, "{:X}", v).unwrap();
        let hx = hex::HexView::new(v);
        let hs = hx.as_str();

        assert_eq!(s, hs, "Hex views not match for {}", v);

        i += 1;
    }
}
