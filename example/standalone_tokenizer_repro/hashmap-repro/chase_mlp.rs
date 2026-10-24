// Independent pointer chains: measure how many concurrent L2/L3 misses a core
// can overlap (memory-level parallelism).
//
// Usage: ./chase_mlp [--n 262144] [--chains 1,2,4,8] [--iters 1000000]
// Each chain walks a random permutation of n usize entries; the chains are
// independent, so ns/load falls with the number of chains until the core
// saturates its outstanding-miss budget.
//
// Output: JSON {"arch":..,"n":..,"chains":{..},"ns_per_load":{..}}

use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mut n: usize = 262_144;
    let mut chains: Vec<usize> = vec![1, 2, 4, 8];
    let mut iters: usize = 1_000_000;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--n" => n = it.next().unwrap().parse().unwrap(),
            "--chains" => {
                chains = it
                    .next()
                    .unwrap()
                    .split(',')
                    .map(|s| s.parse().unwrap())
                    .collect()
            }
            "--iters" => iters = it.next().unwrap().parse().unwrap(),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2)
            }
        }
    }

    // one random permutation cycle over n entries
    let mut perm: Vec<usize> = (0..n).collect();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for i in (1..n).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (state >> 33) as usize % (i + 1);
        perm.swap(i, j);
    }
    let mut table = vec![0usize; n];
    for i in 0..n {
        table[perm[i]] = perm[(i + 1) % n];
    }
    let ptr = table.as_ptr();

    let mut pairs = Vec::new();
    for &k in &chains {
        let stride = n / k.max(1);
        let mut p: Vec<usize> = (0..k).map(|i| perm[i * stride]).collect();
        let t = Instant::now();
        for _ in 0..iters {
            for slot in p.iter_mut() {
                *slot = unsafe { *ptr.add(*slot) };
            }
        }
        let ns = t.elapsed().as_nanos() as f64 / (iters as f64 * k as f64);
        black_box(&p);
        pairs.push((k, ns));
    }

    let ks = pairs
        .iter()
        .map(|(k, _)| k.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let ns = pairs
        .iter()
        .map(|(_, v)| format!("{v:.3}"))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"arch\":\"{}\",\"n\":{},\"chains\":[{}],\"ns_per_load\":[{}]}}",
        std::env::consts::ARCH,
        n,
        ks,
        ns
    );
}
