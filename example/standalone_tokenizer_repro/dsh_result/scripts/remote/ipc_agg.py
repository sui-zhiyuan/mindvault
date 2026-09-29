import re, collections, os, sys
line_re=re.compile(r'^\s*(?P<comm>\S+)\s+(?P<pid>\d+)\s+[\d.]+:\s+(?P<period>\d+)\s+(?P<ev>\S+)\s+(?P<ip>[0-9a-f]+)\s+(?P<rest>.*?)\s*\((?P<dso>[^()]*)\)\s*$')
def leaves(path):
    c=collections.Counter()
    for ln in open(path, errors='replace'):
        m=line_re.match(ln)
        if not m: continue
        sym=re.sub(r'\+0x[0-9a-f]+$','',m.group('rest')).strip()
        c[(sym, os.path.basename(m.group('dso')))]+=1
    return c
cyc=leaves('/tmp/cycles.txt'); ins=leaves('/tmp/instructions.txt')
TC=sum(cyc.values()) or 1; TI=sum(ins.values()) or 1
print(f"perf record -c 1000000 -e cycles:u / -e instructions:u  （同一负载、同一进程形态）")
print(f"样本数 cycles={TC} instructions={TI}；下表 IPC = instr_samples/cycle_samples（周期相同，故等于事件数之比）\n")
print(f"{'函数':44s} {'cycles%':>8s} {'IPC':>6s} {'instr%':>8s}")
for (sym,dso),n in cyc.most_common(16):
    i=ins.get((sym,dso),0); print(f"{sym[:42]:44s} {100*n/TC:7.2f}% {(i/n if n else 0):6.2f} {100*i/TI:7.2f}%")
def fam(sym,dso):
    s=sym.lower()
    if 'malloc' in s or 'free' in s or 'unlink_chunk' in s or 'malloc_consolidate' in s or 'realloc' in s: return 'glibc 分配器'
    if 'hashbrown' in s or 'hash_one' in s or 'reserve_rehash' in s: return 'hashbrown/哈希'
    if 'memcpy' in s or 'memmove' in s or 'memset' in s: return 'memcpy/memset'
    if 'match_at' in s or 'onig' in s: return 'Oniguruma 正则'
    if 'unicode' in s or 'decompose' in s: return 'Unicode 规范化'
    if 'bpe' in s or 'word::' in sym or 'Cache' in sym: return 'BPE/词缓存'
    if 'convert' in s or 'into_encoding' in s or 'encode_single_sequence' in s: return 'tokenizer 编码主体'
    if dso.startswith('libpython') or 'PyEval' in sym or 'PyObject' in sym: return 'CPython 解释器/对象分配'
    if dso.startswith('libc'): return 'libc 其它'
    if dso.startswith('tokenizers'): return 'tokenizers 其它(Rust)'
    return f'其它({dso})'
agg=collections.Counter(); aggi=collections.Counter()
for k,n in cyc.items(): agg[fam(*k)]+=n
for k,n in ins.items(): aggi[fam(*k)]+=n
print(f"\n{'分类':28s} {'cycles%':>8s} {'IPC':>6s} {'instr%':>8s}")
for k,n in agg.most_common():
    if 100*n/TC < 0.05: continue
    print(f"{k:28s} {100*n/TC:7.2f}% {(aggi[k]/n if n else 0):6.2f} {100*aggi[k]/TI:7.2f}%")
