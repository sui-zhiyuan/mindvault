use std::collections::{HashMap, HashSet, hash_map::DefaultHasher};
use std::hash::{BuildHasher, BuildHasherDefault};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

static DROPS: AtomicUsize = AtomicUsize::new(0);
struct DropType(usize);
impl Drop for DropType { fn drop(&mut self) { DROPS.fetch_add(self.0, Ordering::Relaxed); } }
#[derive(Clone)]
struct RandomKeys(usize);
impl Iterator for RandomKeys {
    type Item=usize;
    fn next(&mut self)->Option<usize> {
        self.0=self.0.wrapping_add(1).wrapping_mul(3_787_392_781); Some(self.0)
    }
}
fn output(name:&str,len:usize,capacity:usize,queries:usize,values:&[f64]) {
    println!("{{\"case\":\"{name}\",\"len\":{len},\"capacity\":{capacity},\"queries_per_sample\":{queries},\"ns\":{values:?}}}");
}
fn legacy<S:BuildHasher+Default,I:Iterator<Item=usize>+Clone>(name:&str,keys:I,miss:bool,loops:usize) {
    let mut map:HashMap<usize,DropType,S>=HashMap::with_hasher(S::default());
    for k in keys.clone().take(1000) {map.insert(k,DropType(k));}
    assert_eq!(map.len(),1000);
    assert!(keys.clone().take(1000).all(|k|map.contains_key(&k)));
    assert!(keys.clone().skip(1000).take(1000).all(|k|!map.contains_key(&k)));
    let mut next=keys.clone().skip(1000);
    let mut ns=Vec::new();
    for sample in 0..7 {
        let t=Instant::now();
        if miss {for _ in 0..loops {for k in (&mut next).take(1000) {black_box(map.get(&k));}}}
        else {for _ in 0..loops {for k in keys.clone().take(1000) {black_box(map.get(&k));}}}
        let v=t.elapsed().as_nanos() as f64/(loops*1000) as f64;
        if sample>=2 {ns.push(v);}
    }
    output(name,map.len(),map.capacity(),loops*1000,&ns);
}
fn integer_stress(buckets:usize,pct:usize,miss:bool,tombstone:bool,budget:usize) {
    let target=buckets*pct/100;
    let mut map=HashMap::with_capacity_and_hasher(buckets*7/8,ahash::RandomState::with_seeds(1,2,3,4));
    for k in 0..target {map.insert(k as u64,k as u64);}
    if tombstone { for k in (0..target).step_by(3) {map.remove(&(k as u64));} }
    let queries:Vec<_>=(0..target).filter(|i| !tombstone || i%3!=0)
        .map(|i|if miss {(i+target) as u64}else{i as u64}).collect();
    assert!(queries.iter().all(|k|map.contains_key(k)!=miss));
    let loops=(budget/queries.len()).max(1);
    let mut ns=Vec::new();
    for sample in 0..7 {
        let t=Instant::now();
        for _ in 0..loops {for k in &queries {black_box(map.get(k));}}
        let v=t.elapsed().as_nanos() as f64/(loops*queries.len()) as f64;
        if sample>=2 {ns.push(v);}
    }
    output(&format!("ahash_u64_b{buckets}_p{pct}_{}_{}",if miss{"miss"}else{"hit"},if tombstone{"deleted"}else{"fresh"}),map.len(),map.capacity(),loops*queries.len(),&ns);
}
fn strings(n:usize,miss:bool,budget:usize) {
    let mut map=HashMap::with_capacity_and_hasher(n,ahash::RandomState::with_seeds(1,2,3,4));
    for k in 0..n {map.insert(format!("piece-中文-{k:08}"),k as u32);}
    let queries:Vec<_>=(0..n).map(|k|format!("piece-中文-{:08}",k+if miss {n}else{0})).collect();
    assert!(queries.iter().all(|k|map.contains_key(k.as_str())!=miss));
    let loops=(budget/n).max(1);let mut ns=Vec::new();
    for sample in 0..7 {
        let t=Instant::now();for _ in 0..loops {for k in &queries {black_box(map.get(k.as_str()));}}
        let v=t.elapsed().as_nanos() as f64/(loops*n) as f64;if sample>=2 {ns.push(v);}
    }
    output(&format!("ahash_string_n{n}_{}",if miss{"miss"}else{"hit"}),map.len(),map.capacity(),loops*n,&ns);
}
fn insert_reuse(n:usize) {
    let mut map:HashMap<usize,usize,BuildHasherDefault<DefaultHasher>>=HashMap::with_capacity_and_hasher(n,Default::default());
    let loops=8;let mut ns=Vec::new();
    for sample in 0..7 {
        let t=Instant::now();
        for _ in 0..loops {map.clear();for i in 0..n {black_box(map.insert(i,i));}black_box(&map);}
        let v=t.elapsed().as_nanos() as f64/(loops*n) as f64;if sample>=2 {ns.push(v);}
        assert_eq!(map.len(),n);
    }
    output(&format!("insert_reuse_std_n{n}"),n,map.capacity(),loops*n,&ns);
}
fn grow_insert(n:usize) {
    let loops=8;let mut ns=Vec::new();let mut capacity=0;
    for sample in 0..7 {
        let t=Instant::now();
        for _ in 0..loops {
            let mut map:HashMap<usize,usize,BuildHasherDefault<DefaultHasher>>=HashMap::default();
            for i in 0..n {black_box(map.insert(i,i));}capacity=map.capacity();black_box(&map);
        }
        let v=t.elapsed().as_nanos() as f64/(loops*n) as f64;if sample>=2 {ns.push(v);}
    }
    output(&format!("grow_insert_std_n{n}"),n,capacity,loops*n,&ns);
}
fn offsets<S:BuildHasher+Default>(name:&str,text:&str) {
    let loops=8;let mut ns=Vec::new();let mut capacity=0;let mut len=0;
    for sample in 0..7 {
        let t=Instant::now();
        for _ in 0..loops {
            let map:HashMap<usize,usize,S>=text.char_indices().enumerate().flat_map(|(i,(b,c))| {
                let mut n=0;std::iter::repeat_with(move || {let o=(b+n,i);n+=1;o}).take(c.len_utf8())
            }).collect();
            len=map.len();capacity=map.capacity();black_box(&map);
        }
        let v=t.elapsed().as_nanos() as f64/(loops*text.len()) as f64;if sample>=2 {ns.push(v);}
        assert_eq!(len,text.len());
    }
    output(name,len,capacity,loops*text.len(),&ns);
}
fn main() {
    let args:Vec<_>=std::env::args().collect();
    let mode=args.get(1).map(String::as_str).unwrap_or("legacy");
    let variant=args.get(2).expect("backend label required");
    let tiny=HashSet::<u8>::with_capacity(1).capacity();
    let expected=if variant=="neon8" {7}else if variant.contains("32") {28}else {14};
    assert_eq!(tiny,expected,"custom stdlib group width is not active");
    if mode=="legacy" {
        let loops=args.get(3).map(|s|s.parse().unwrap()).unwrap_or(2048);
        macro_rules! cases {($label:literal,$keys:expr)=>{{
            for miss in [false,true] {
                let prefix=if miss{"lookup_fail"}else{"lookup"};
                legacy::<foldhash::fast::FixedState,_>(&format!("{prefix}_foldhash_{}",$label),$keys,miss,loops);
                legacy::<BuildHasherDefault<DefaultHasher>,_>(&format!("{prefix}_std_{}",$label),$keys,miss,loops);
            }
        }}}
        cases!("serial",0usize..);cases!("highbits",(0usize..).map(usize::swap_bytes));cases!("random",RandomKeys(0));
    } else if mode=="extended" {
        let budget=args.get(3).map(|s|s.parse().unwrap()).unwrap_or(262144);
        for buckets in [8192,65536,524288] {for pct in [50,85] {for miss in [false,true] {integer_stress(buckets,pct,miss,false,budget);}}}
        for miss in [false,true] {integer_stress(65536,85,miss,true,budget);strings(32768,miss,budget);strings(151643,miss,budget);}
    } else if mode=="insert" {
        for n in [1000,10000,78960] {insert_reuse(n);grow_insert(n);}
        let text=std::fs::read_to_string("/root/hashbrown-wide-20260924/artifacts/offset-text.txt").unwrap();
        offsets::<BuildHasherDefault<DefaultHasher>>("byte_offset_collect_fixed",&text);
        offsets::<std::collections::hash_map::RandomState>("byte_offset_collect_random",&text);
    } else {panic!("unknown mode");
    }
    black_box(DROPS.load(Ordering::Relaxed));
}
