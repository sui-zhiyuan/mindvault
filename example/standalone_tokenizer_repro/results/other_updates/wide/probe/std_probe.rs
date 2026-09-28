use std::collections::{HashMap,HashSet};
fn main() {
    let tiny=HashSet::<u8>::with_capacity(1);
    let mut map=HashMap::new();
    for i in 0..4096u64 { map.insert(i,i*13); }
    for i in 0..4096 { assert_eq!(map.get(&i),Some(&(i*13))); }
    for i in (0..4096).step_by(3) { assert_eq!(map.remove(&i),Some(i*13)); }
    map.shrink_to_fit();
    let other=map.clone();
    assert_eq!(map,other);
    println!("std HashMap probe PASS; HashSet<u8> capacity(1)={}; remaining={}",tiny.capacity(),map.len());
}
