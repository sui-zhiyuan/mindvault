// Pointer-chase latency across working-set sizes.
//
// Builds a random single cycle over N usize entries and follows it; the
// dependent loads cannot overlap, so ns/load is the load-to-use latency at
// that footprint (L1/L2/L3/DRAM).
//
// Usage: ./chase [--sizes 4096,16384,65536,262144,1048576,4194304,16777216] [--iters 5000000]
// Output: JSON {"arch":..,"sizes":{..},"ns_per_load":{..}}

use std::hint::black_box;
use std::time::Instant;

fn main() {
    let mut sizes: Vec<usize> = vec![4096, 16384, 65536, 262144, 1048576, 4194304, 16777216];
    let mut iters: usize = 5_000_000;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--sizes" => {
                let v = it.next().unwrap();
                sizes = v.split(',').map(|s| s.parse().unwrap()).collect();
            }
            "--iters" => iters = it.next().unwrap().parse().unwrap(),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2)
            }
        }
    }

    let mut out = Vec::new();
    for &n in &sizes {
        let mut perm: Vec<usize> = (0..n).collect();
        // deterministic LCG shuffle
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
        let mut p = perm[0];
        let ptr = table.as_ptr();
        let t = Instant::now();
        for _ in 0..iters {
            // p is always < n by construction; avoid the bounds check so the
            // measured chain is one dependent load (a checked index adds a
            // second dependent branch that distorted an earlier version).
            p = unsafe { *ptr.add(p) };
        }
        let ns = t.elapsed().as_nanos() as f64 / iters as f64;
        black_box(p);
        out.push((n, ns));
    }

    let sizes_js = out
        .iter()
        .map(|(n, _)| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let ns_js = out
        .iter()
        .map(|(_, ns)| format!("{ns:.3}"))
        .collect::<Vec<_>>()
        .join(",");
    println!(
        "{{\"arch\":\"{}\",\"sizes\":[{}],\"ns_per_load\":[{}]}}",
        std::env::consts::ARCH,
        sizes_js,
        ns_js
    );
}
