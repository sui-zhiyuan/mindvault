use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::time::Instant;
use std::hint::black_box;

type Offsets = (usize, usize);

struct HashMapConv { map: HashMap<usize, usize> }
impl HashMapConv {
    fn new(sequence: &str) -> Self {
        Self { map: sequence.char_indices().enumerate().flat_map(|(i, (b, c))| {
            let mut n = 0;
            std::iter::repeat_with(move || { let o = (b + n, i); n += 1; o }).take(c.len_utf8())
        }).collect() }
    }
    #[inline] fn convert(&self, o: Offsets) -> Option<Offsets> {
        match (self.map.get(&o.0), self.map.get(&o.1)) {
            (Some(s), Some(e)) => Some((*s, *e)),
            (Some(s), None) => { let l = self.map.get(&(o.1 - 1)).copied().unwrap_or(s + 1); Some((*s, l + 1)) }
            _ => None,
        }
    }
}
struct VecConv { map: Vec<u32> }
impl VecConv {
    fn new(sequence: &str) -> Self {
        let mut map = vec![0u32; sequence.len()];
        for (i, (b, c)) in sequence.char_indices().enumerate() {
            for n in 0..c.len_utf8() { map[b + n] = i as u32; }
        }
        Self { map }
    }
    #[inline] fn convert(&self, o: Offsets) -> Option<Offsets> {
        let len = self.map.len();
        if o.0 >= len { return None; }
        let start = self.map[o.0] as usize;
        let end = match self.map.get(o.1) {
            Some(&e) => e as usize,
            None => match self.map.get(o.1.wrapping_sub(1)) { Some(&l) => l as usize + 1, None => start + 1 },
        };
        Some((start, end))
    }
}
fn main() {
    let text = std::fs::read_to_string("/tmp/offset-text.txt").unwrap();
    let n = text.len();
    // 模仿 25600 个 token 的 offsets（每个 token 平均覆盖 n/25600 字节）
    let step = n / 25600;
    let offsets: Vec<Offsets> = (0..25600).map(|i| { let s = i*step; (s, (s+step).min(n)) }).collect();
    let iters = 20;
    // HashMap + 默认 hasher（stock）
    let mut t_build_hm = f64::MAX; let mut t_look_hm = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now(); let mut c = None;
        for _ in 0..iters { c = Some(HashMapConv::new(&text)); black_box(&c); }
        t_build_hm = t_build_hm.min(t.elapsed().as_secs_f64()/iters as f64);
        let conv = c.unwrap();
        let t = Instant::now(); let mut acc = 0usize;
        for _ in 0..iters { for &o in &offsets { if let Some((s,e)) = conv.convert(black_box(o)) { acc = acc.wrapping_add(s+e); } } }
        t_look_hm = t_look_hm.min(t.elapsed().as_secs_f64()/iters as f64); black_box(acc);
    }
    // Vec
    let mut t_build_v = f64::MAX; let mut t_look_v = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now(); let mut c = None;
        for _ in 0..iters { c = Some(VecConv::new(&text)); black_box(&c); }
        t_build_v = t_build_v.min(t.elapsed().as_secs_f64()/iters as f64);
        let conv = c.unwrap();
        let t = Instant::now(); let mut acc = 0usize;
        for _ in 0..iters { for &o in &offsets { if let Some((s,e)) = conv.convert(black_box(o)) { acc = acc.wrapping_add(s+e); } } }
        t_look_v = t_look_v.min(t.elapsed().as_secs_f64()/iters as f64); black_box(acc);
    }
    println!("文本 {} 字节 / {} tokens", n, offsets.len());
    println!("构造 HashMap : {:8.3} ms/call  ({:6.2} ns/byte)", t_build_hm*1e3, t_build_hm*1e9/n as f64);
    println!("构造 Vec<u32>: {:8.3} ms/call  ({:6.2} ns/byte)   加速 {:.1}x", t_build_v*1e3, t_build_v*1e9/n as f64, t_build_hm/t_build_v);
    println!("查询 HashMap : {:8.3} ms/call", t_look_hm*1e3);
    println!("查询 Vec     : {:8.3} ms/call   加速 {:.1}x", t_look_v*1e3, t_look_hm/t_look_v);
    println!("合计 HashMap : {:8.3} ms   Vec: {:8.3} ms   合计加速 {:.1}x", (t_build_hm+t_look_hm)*1e3, (t_build_v+t_look_v)*1e3, (t_build_hm+t_look_hm)/(t_build_v+t_look_v));
}
