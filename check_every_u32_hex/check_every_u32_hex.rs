#![allow(clippy::uninlined_format_args, clippy::collapsible_if)]

use core::fmt::Write as _;
use core::num::NonZeroUsize;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::time::Instant;

#[path = "../small_type_id/src/hex.rs"]
mod hex;

fn main() {
    println!("Starting checking values in multi thread...");
    if let Some(flags) = option_env!("RUSTFLAGS") {
        if flags.contains("-Zsanitizer=address") || flags.contains("-Z sanitizer=address") {
            println!("  Using Address Sanitizer");
        }
    }
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
        .map(NonZeroUsize::get)
        .unwrap_or(1) as u64;
    let chunk_size = ((u64::from(u32::MAX) + 1) / num_cores) + 1;
    let counter = AtomicU64::new(0);
    let counter = &counter;
    thread::scope(move |scope| {
        for i in 0..num_cores {
            let start: u64 = i * chunk_size;
            let end: u32 = start
                .saturating_add(chunk_size)
                .min(u32::MAX.into())
                .try_into()
                .unwrap();
            let start: u32 = start.try_into().unwrap();
            println!("Thread {:2}: checking range {:X}..={:X}", i, start, end);
            scope.spawn(move || check_range(start, end, counter));
        }
    });
}

const REPORT_ADD_LIMIT: u64 = 1_000_000;
const REPORT_LOG_LIMIT: u64 = 100_000_000;

fn check_range(start: u32, end: u32, counter: &AtomicU64) {
    let mut s = String::with_capacity(16);
    let mut i: u64 = start.into();
    let end: u64 = end.into();
    let mut report_counter = 0;
    while i <= end {
        let v: u32 = i.try_into().unwrap();

        s.clear();
        write!(&mut s, "{:X}", v).unwrap();
        let hx = hex::HexView::new(v);
        let hs = hx.as_str();

        assert_eq!(s, hs, "Hex views not match for {}", v);

        i += 1;
        report_counter += 1;
        if report_counter == REPORT_ADD_LIMIT {
            update_counter(counter, report_counter);
            report_counter = 0;
        }
    }
    update_counter(counter, report_counter);
}

fn update_counter(counter: &AtomicU64, num: u64) {
    let old_count = counter.fetch_add(num, Relaxed);
    if (old_count + num) / REPORT_LOG_LIMIT != old_count / REPORT_LOG_LIMIT {
        println!("Processed {:10} nums...", old_count + num);
    }
}
