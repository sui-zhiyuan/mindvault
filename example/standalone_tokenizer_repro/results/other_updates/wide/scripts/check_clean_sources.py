import hashlib,json,tarfile
from pathlib import Path
w=Path('/root/hashbrown-wide-20260924')
manifest={};allowed=[]
with tarfile.open(w/'artifacts/tokenizers-clean-v0.21.4.tar.gz') as t:
    for m in t.getmembers():
        if not m.isfile(): continue
        actual=(w/'tokenizers-clean'/m.name).read_bytes()
        expected=t.extractfile(m).read()
        if m.name.endswith('Cargo.lock'):
            if actual!=expected: allowed.append(m.name)
        else:
            assert actual==expected, f'Upstream file changed: {m.name}'
            manifest[m.name]=hashlib.sha256(actual).hexdigest()
(w/'artifacts/tokenizer-source-proof.json').write_text(json.dumps({'unchanged_files':manifest,'build_lockfile_updates':allowed},indent=2))
hb={str(p.relative_to(w/'hashbrown')):hashlib.sha256(p.read_bytes()).hexdigest() for p in (w/'hashbrown/src').rglob('*.rs')}
frozen=w/'artifacts/hashbrown-final-source.json'
if frozen.exists(): assert json.loads(frozen.read_text())==hb,'Hashbrown changed after final freeze'
else: frozen.write_text(json.dumps(hb,indent=2))
stock=Path('/root/hashbrown-sve-20260924/.rustup/toolchains/1.98.1-aarch64-unknown-linux-gnu/lib/rustlib/src/rust/library')
count=0
for base in ['std','core','alloc','proc_macro','test','panic_unwind','unwind']:
    for p in (stock/base).rglob('*.rs'):
        assert p.read_bytes()==(w/'rust-source/library'/p.relative_to(stock)).read_bytes(),str(p)
        count+=1
(w/'artifacts/stdlib-source-proof.json').write_text(json.dumps({'unchanged_rust_files':count,'functional_patch':'hashbrown only'},indent=2))
print('PASS: tokenizer upstream unchanged:',len(manifest),'files; build lock updates:',allowed)
