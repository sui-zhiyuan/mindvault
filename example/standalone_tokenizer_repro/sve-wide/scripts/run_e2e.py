import argparse,json,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--rounds',type=int,default=10);p.add_argument('--aa',action='store_true');p.add_argument('--output',required=True);a=p.parse_args()
w=Path('/root/hashbrown-wide-20260924');old=Path('/root/hashbrown-sve-20260924')
variants=['neon8','neon16','sve16-fused','sve32-fused','sve32-fused-private'] if not a.aa else ['neon8','neon8-repeat']
out=w/a.output;out.mkdir(parents=True,exist_ok=False)
manifest={'variants':variants,'rounds':a.rounds,'aa':a.aa,'cpu_set':[240,242,244,246,248,250,252,254],'numa':3,'runs':[]};oracle=None
for block in range(a.rounds):
    order=variants[block%len(variants):]+variants[:block%len(variants)]
    if block%2: order=list(reversed(order))
    for label in order:
        variant='neon8' if label=='neon8-repeat' else label
        folder=out/f'{block:02d}-{label}'
        command=['python3',str(old/'scripts/run_with_vl.py'),'32','numactl','--physcpubind=240,242,244,246,248,250,252,254','--membind=3',str(w/f'venv-{variant}/bin/python'),str(old/'standalone_tokenizer_repro/run_variants.py'),'--rounds','3','--warmup','10','--samples','30','--output-dir',str(folder)]
        started=time.time();r=subprocess.run(command,capture_output=True,text=True,timeout=300)
        (out/f'{block:02d}-{label}.log').write_text(r.stdout+r.stderr)
        if r.returncode: raise SystemExit(f'{label} failed: {r.stdout} {r.stderr}')
        assert json.loads((folder/'complete.json').read_text())=={'default':0,'rayon1':0,'rayon8':0}
        for config in ('default','rayon1','rayon8'):
            data=json.loads((folder/f'{config}.json').read_text());same={k:data[k] for k in ['shape','versions','python','token_sha256','tokenizer_files_sha256']}
            if oracle is None:oracle=same
            assert same==oracle
            assert sum(not row['warmup'] for row in data['rows'])==90
        manifest['runs'].append({'block':block,'variant':label,'command':command,'elapsed':time.time()-started})
        (out/'manifest.json').write_text(json.dumps(manifest,indent=2));print(f'block {block+1}/{a.rounds} {label}: {time.time()-started:.2f}s',flush=True)
