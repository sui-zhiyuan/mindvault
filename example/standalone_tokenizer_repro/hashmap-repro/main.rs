// Minimal HashMap repro for the ARM-vs-x86 tokenizer gap.
//
// The real hotspot is BytesToCharOffsetConverter: it inserts one entry per
// UTF-8 byte (dense 0..len keys) into HashMap<usize, usize> and then looks
// boundaries up. This binary keeps that shape and nothing else, so the cost
// measured here is hash + probe + growth + allocator, not tokenization.
//
// Three hashers, each with a different purpose:
//   std      std RandomState (SipHash-1-3), the real configuration
//   mix      a cheap splitmix-style usize hash: near-random bucket layout,
//            so (std - mix) isolates the cost of SipHash itself
//   identity hash = key: degenerate layout (sequential keys -> consecutive
//            ctrl positions), useful only as a "no hashing at all" bound
//
// Usage:  ./hashmap_repro [--n 78960] [--reps 15] [--warmup 3]
//                        [--hasher std|mix|identity] [--reserve] [--stride 1]
//                        [--query-file offsets.txt]  (whitespace-separated byte offsets)
// Output: one JSON line with build/query/drop timings (ns) per rep.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::hint::black_box;
use std::time::Instant;

#[derive(Default)]
struct MixHasher(u64);

impl Hasher for MixHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 = h;
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        x ^= x >> 29;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 32;
        self.0 = x;
    }
}

#[derive(Default)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = 0u64;
        for &b in bytes {
            h = h.wrapping_mul(131).wrapping_add(b as u64);
        }
        self.0 = h;
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.0 = i as u64;
    }
}

struct Args {
    mode: String,
    n: usize,
    reps: usize,
    warmup: usize,
    hasher: String,
    reserve: bool,
    stride: usize,
    query_file: Option<String>,
    query_repeat: usize,
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut a = Args {
        mode: "dense".into(),
        n: 78_960,
        reps: 15,
        warmup: 3,
        hasher: "std".into(),
        reserve: false,
        stride: 1,
        query_file: None,
        query_repeat: 1,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        if flag == "--reserve" {
            a.reserve = true;
            continue;
        }
        let v = it
            .next()
            .unwrap_or_else(|| die(&format!("missing value for {flag}")));
        match flag.as_str() {
            "--mode" => a.mode = v,
            "--n" => a.n = v.parse().unwrap_or_else(|_| die("bad --n")),
            "--reps" => a.reps = v.parse().unwrap_or_else(|_| die("bad --reps")),
            "--warmup" => a.warmup = v.parse().unwrap_or_else(|_| die("bad --warmup")),
            "--hasher" => a.hasher = v,
            "--stride" => a.stride = v.parse().unwrap_or_else(|_| die("bad --stride")),
            "--query-file" => a.query_file = Some(v),
            "--query-repeat" => a.query_repeat = v.parse().unwrap_or_else(|_| die("bad --query-repeat")),
            other => die(&format!("unknown flag {other}")),
        }
    }
    if a.mode != "dense" {
        die("only --mode dense is implemented");
    }
    if a.stride == 0 {
        die("--stride must be >= 1");
    }
    a
}

fn bench<S: std::hash::BuildHasher + Default>(
    a: &Args,
    queries: &[usize],
    build: &mut Vec<u128>,
    query: &mut Vec<u128>,
    drop_t: &mut Vec<u128>,
) {
    for r in 0..(a.warmup + a.reps) {
        let measured = r >= a.warmup;

        let t0 = Instant::now();
        let mut m: HashMap<usize, usize, S> = if a.reserve {
            HashMap::with_capacity_and_hasher(a.n, S::default())
        } else {
            HashMap::with_hasher(S::default())
        };
        for k in 0..a.n {
            m.insert(k, k);
        }
        let t1 = Instant::now();

        let mut acc = 0usize;
        for _ in 0..a.query_repeat {
            for &k in queries {
                acc = acc.wrapping_add(*black_box(m.get(&k)).unwrap_or(&0));
            }
        }
        let t2 = Instant::now();

        drop(m);
        let t3 = Instant::now();

        black_box(acc);
        if measured {
            build.push(t1.duration_since(t0).as_nanos());
            query.push(t2.duration_since(t1).as_nanos());
            drop_t.push(t3.duration_since(t2).as_nanos());
        }
    }
}

fn main() {
    let a = parse_args();
    let queries: Vec<usize> = match &a.query_file {
        Some(path) => {
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("read {path}: {e}")));
            text.split_whitespace()
                .map(|t| t.parse().unwrap_or_else(|_| die(&format!("bad offset {t}"))))
                .collect()
        }
        None => (0..a.n).step_by(a.stride).collect(),
    };
    let mut build = Vec::new();
    let mut query = Vec::new();
    let mut drop_t = Vec::new();
    match a.hasher.as_str() {
        "std" => bench::<std::collections::hash_map::RandomState>(&a, &queries, &mut build, &mut query, &mut drop_t),
        "mix" => bench::<BuildHasherDefault<MixHasher>>(&a, &queries, &mut build, &mut query, &mut drop_t),
        "identity" => bench::<BuildHasherDefault<IdHasher>>(&a, &queries, &mut build, &mut query, &mut drop_t),
        other => die(&format!("unknown hasher {other}")),
    }
    let js = |v: &[u128]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    println!(
        "{{\"arch\":\"{}\",\"mode\":\"{}\",\"hasher\":\"{}\",\"n\":{},\"reps\":{},\"warmup\":{},\"reserve\":{},\"stride\":{},\"queries\":{},\"build_ns\":[{}],\"query_ns\":[{}],\"drop_ns\":[{}]}}",
        std::env::consts::ARCH,
        a.mode,
        a.hasher,
        a.n,
        a.reps,
        a.warmup,
        a.reserve,
        a.stride,
        queries.len(),
        js(&build),
        js(&query),
        js(&drop_t)
    );
}
