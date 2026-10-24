// Minimal HashMap repro for the ARM-vs-x86 tokenizer gap.
//
// The real hotspot is BytesToCharOffsetConverter: it inserts one entry per
// UTF-8 byte (dense 0..len keys) into HashMap<usize, usize> and then looks
// boundaries up. This binary keeps that shape and nothing else, so the cost
// measured here is hash + probe + growth + allocator, not tokenization.
//
// Build:  rustc -O main.rs -o hashmap_repro
// Usage:  ./hashmap_repro --mode dense --n 78960 --reps 20 --hasher std
//         --hasher std|identity   identity isolates the hash function cost
//         --reserve               pre-size the table (no rehash growth)
//         --stride 1              query stride (1 = every key)

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::hint::black_box;
use std::time::Instant;

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

type IdMap = HashMap<usize, usize, BuildHasherDefault<IdHasher>>;

struct Args {
    mode: String,
    n: usize,
    reps: usize,
    warmup: usize,
    hasher: String,
    reserve: bool,
    stride: usize,
}

fn parse_args() -> Args {
    let mut a = Args {
        mode: "dense".into(),
        n: 78_960,
        reps: 20,
        warmup: 3,
        hasher: "std".into(),
        reserve: false,
        stride: 1,
    };
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i + 1 < argv.len() {
        let v = &argv[i + 1];
        match argv[i].as_str() {
            "--mode" => a.mode = v.clone(),
            "--n" => a.n = v.parse().unwrap(),
            "--reps" => a.reps = v.parse().unwrap(),
            "--warmup" => a.warmup = v.parse().unwrap(),
            "--hasher" => a.hasher = v.clone(),
            "--stride" => a.stride = v.parse().unwrap(),
            "--reserve" => {
                a.reserve = true;
                i -= 1; // flag without value
            }
            other => panic!("unknown arg {}", other),
        }
        i += 2;
    }
    a
}

fn run_std(a: &Args, build: &mut Vec<u128>, query: &mut Vec<u128>) {
    for _ in 0..a.reps {
        let t0 = Instant::now();
        let mut m: HashMap<usize, usize> = if a.reserve {
            HashMap::with_capacity(a.n)
        } else {
            HashMap::new()
        };
        for k in 0..a.n {
            m.insert(k, k);
        }
        let t1 = Instant::now();
        let mut acc = 0usize;
        let mut k = 0;
        while k < a.n {
            acc = acc.wrapping_add(*black_box(m.get(&k)).unwrap_or(&0));
            k += a.stride;
        }
        let t2 = Instant::now();
        black_box(acc);
        build.push(t1.duration_since(t0).as_nanos());
        query.push(t2.duration_since(t1).as_nanos());
    }
}

fn run_id(a: &Args, build: &mut Vec<u128>, query: &mut Vec<u128>) {
    for _ in 0..a.reps {
        let t0 = Instant::now();
        let mut m: IdMap = if a.reserve {
            HashMap::with_capacity_and_hasher(a.n, Default::default())
        } else {
            HashMap::default()
        };
        for k in 0..a.n {
            m.insert(k, k);
        }
        let t1 = Instant::now();
        let mut acc = 0usize;
        let mut k = 0;
        while k < a.n {
            acc = acc.wrapping_add(*black_box(m.get(&k)).unwrap_or(&0));
            k += a.stride;
        }
        let t2 = Instant::now();
        black_box(acc);
        build.push(t1.duration_since(t0).as_nanos());
        query.push(t2.duration_since(t1).as_nanos());
    }
}

fn main() {
    let a = parse_args();
    let mut build = Vec::new();
    let mut query = Vec::new();
    if a.hasher == "identity" {
        run_id(&a, &mut build, &mut query);
    } else {
        run_std(&a, &mut build, &mut query);
    }
    let arch = std::env::consts::ARCH;
    let mode = a.mode.clone();
    let js = |v: &[u128]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    println!(
        "{{\"arch\":\"{arch}\",\"mode\":\"{mode}\",\"hasher\":\"{}\",\"n\":{},\"reps\":{},\"warmup\":{},\"reserve\":{},\"stride\":{},\"build_ns\":[{}],\"query_ns\":[{}]}}",
        a.hasher, a.n, a.reps, a.warmup, a.reserve, a.stride, js(&build), js(&query)
    );
}
