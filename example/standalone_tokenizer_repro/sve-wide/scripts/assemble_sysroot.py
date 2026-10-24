import hashlib,json,shutil,sys,time
from pathlib import Path
work=Path('/root/hashbrown-wide-20260924')
variant=sys.argv[1]
assert variant.replace('-','').isalnum()
source=work/f'target-std-{variant}'/'aarch64-unknown-linux-gnu/dist/deps'
dest=work/'sysroots'/variant/'lib/rustlib/aarch64-unknown-linux-gnu/lib'
buildlog=work/'artifacts'/f'stdlib-{variant}-build.jsonl'
if buildlog.exists():
    files=[]
    for line in buildlog.read_text().splitlines():
        entry=json.loads(line)
        if entry.get('reason')=='compiler-artifact':
            files.extend(Path(f) for f in entry['filenames'] if Path(f).suffix in ('.rlib','.so') and '/aarch64-unknown-linux-gnu/dist/' in f)
else:
    files=list(source.glob('*.rlib'))+list(source.glob('*.so'))
assert any(p.name=='libstd.rlib' or p.name.startswith('libstd-') for p in files)
if dest.exists():
    assert dest.resolve().is_relative_to(work/'sysroots')
    dest.rename(dest.with_name(f'lib.previous-{time.time_ns()}'))
dest.mkdir(parents=True)
for p in files: shutil.copy2(p,dest/p.name)
manifest={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(dest.iterdir()) if p.is_file()}
(work/'artifacts'/f'sysroot-{variant}.json').write_text(json.dumps(manifest,indent=2))
print('Assembled',variant,len(manifest),'libraries')
