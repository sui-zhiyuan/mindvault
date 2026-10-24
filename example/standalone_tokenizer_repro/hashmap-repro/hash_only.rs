// Isolate SipHash-1-3 cost from HashMap layout: hash the same usize keys
// with std RandomState, no table involved.
//
// Usage: ./hash_only [--n 78960] [--reps 15] [--warmup 3]
// Output: one JSON line with per-rep ns.

use std::hash::{BuildHasher, RandomState};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mut n: usize = 78_960;
    let mut reps: usize = 15;
    let mut warmup: usize = 3;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let v = it.next().unwrap_or_else(|| {
            eprintln!("missing value for {flag}");
            std::process::exit(2)
        });
        match flag.as_str() {
            "--n" => n = v.parse().unwrap(),
            "--reps" => reps = v.parse().unwrap(),
            "--warmup" => warmup = v.parse().unwrap(),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2)
            }
        }
    }
    let s = RandomState::new();
    let mut times = Vec::new();
    for r in 0..(warmup + reps) {
        let t = Instant::now();
        let mut acc = 0u64;
        for k in 0..n {
            acc = acc.wrapping_add(black_box(s.hash_one(k)));
        }
        black_box(acc);
        if r >= warmup {
            times.push(t.elapsed().as_nanos());
        }
    }
    let js = times
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"arch\":\"{}\",\"n\":{},\"reps\":{},\"warmup\":{},\"hash_ns\":[{}]}}",
        std::env::consts::ARCH,
        n,
        reps,
        warmup,
        js
    );
}
