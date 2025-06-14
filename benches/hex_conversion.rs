use core::fmt::Write as _;
use std::hint::black_box;

use criterion::BatchSize::LargeInput;
use criterion::Criterion;

#[allow(unused_imports)]
#[path = "../small_type_id/src/hex.rs"]
mod hex;

const NUMS_TO_TEST: &[u32] = &[0, 0xFF, 0x1234, 0x1234ABCD];

fn convert_naive(mut num: u32) -> ([u8; 8], usize) {
    let mut res = [0; 8];
    let mut pos = 0;
    loop {
        let digit = (num & 0xF) as u8;
        num >>= 4;

        res[pos] = digit + if digit < 10 { b'0' } else { b'A' };
        pos += 1;
        if num == 0 {
            break;
        }
    }
    res[..pos].reverse();
    (res, pos)
}

fn convert_unsafe(mut num: u32) -> ([u8; 8], usize) {
    let len = 8 - num.leading_zeros() / 4;
    let len = if len == 0 { 1 } else { len as usize };
    let mut res = [0; 8];
    let mut it = unsafe { res.as_mut_ptr().add(len) };
    loop {
        unsafe {
            it = it.sub(1);
            *it = (num & 0xF) as u8;
            num >>= 4;
            if num == 0 {
                break;
            }
        }
    }
    (res, len)
}

fn criterion_benchmark(c: &mut Criterion) {
    for &num in NUMS_TO_TEST {
        fn prepare_buffer() -> String {
            String::with_capacity(16)
        }
        let mut g = c.benchmark_group(&format!("stream {:X}", num));
        g.bench_function("current_version", |b| {
            b.iter_batched(
                prepare_buffer,
                |mut s| {
                    let num = black_box(num);
                    s.push_str(hex::HexView::new(num).as_str());
                    s
                },
                LargeInput,
            );
        });
        g.bench_function("std_hex_fmt", |b| {
            b.iter_batched(
                prepare_buffer,
                |mut s| {
                    let num = black_box(num);
                    write!(&mut s, "{:X}", num).unwrap();
                    s
                },
                LargeInput,
            );
        });
        g.bench_function("naive", |b| {
            b.iter_batched(
                prepare_buffer,
                |mut s| {
                    let num = black_box(num);
                    let (buf, len) = convert_naive(num);
                    s.push_str(unsafe { str::from_utf8_unchecked(&buf[..len]) });
                    s
                },
                LargeInput,
            );
        });
        g.bench_function("unsafe", |b| {
            b.iter_batched(
                prepare_buffer,
                |mut s| {
                    let num = black_box(num);
                    let (buf, len) = convert_unsafe(num);
                    s.push_str(unsafe { str::from_utf8_unchecked(&buf[..len]) });
                    s
                },
                LargeInput,
            );
        });
    }
    for &num in NUMS_TO_TEST {
        let mut g = c.benchmark_group(&format!("to_string {:X}", num));
        g.bench_function("current_version", |b| {
            b.iter(|| {
                let num = black_box(num);
                hex::HexView::new(num).as_str().to_string()
            });
        });
        g.bench_function("std_hex_fmt", |b| {
            b.iter(|| {
                let num = black_box(num);
                format!("{:X}", num)
            });
        });
        g.bench_function("naive", |b| {
            b.iter(|| {
                let num = black_box(num);
                let (buf, len) = convert_naive(num);
                unsafe { str::from_utf8_unchecked(&buf[..len]).to_string() }
            });
        });
        g.bench_function("unsafe", |b| {
            b.iter(|| {
                let num = black_box(num);
                let (buf, len) = convert_unsafe(num);
                unsafe { str::from_utf8_unchecked(&buf[..len]).to_string() }
            });
        });
    }
}

criterion::criterion_group!(benches, criterion_benchmark);
criterion::criterion_main!(benches);
