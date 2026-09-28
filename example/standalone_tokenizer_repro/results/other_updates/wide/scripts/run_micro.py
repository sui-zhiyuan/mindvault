import argparse,json,subprocess,time
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--variants',nargs='+',default=['neon8','neon16','sve16','sve32']);p.add_argument('--rounds',type=int,default=3);p.add_argument('--mode',choices=['legacy','extended','insert'],default='legacy');p.add_argument('--output',required=True);a=p.parse_args()
w=Path('/root/hashbrown-wide-20260924');out=w/a.output;out.mkdir(parents=True,exist_ok=False)
manifest={'variants':a.variants,'rounds':a.rounds,'mode':a.mode,'runs':[]}
for block in range(a.rounds):
    order=a.variants[block%len(a.variants):]+a.variants[:block%len(a.variants)]
    if block%2: order=list(reversed(order))
    for variant in order:
        cmd=['python3','/root/hashbrown-sve-20260924/scripts/run_with_vl.py','32','numactl','--physcpubind=240','--membind=3',str(w/'artifacts'/f'micro-{variant}'),a.mode,variant]
        started=time.time();r=subprocess.run(cmd,capture_output=True,text=True)
        (out/f'{block:02d}-{variant}.jsonl').write_text(r.stdout);(out/f'{block:02d}-{variant}.log').write_text(r.stderr)
        assert r.returncode==0,r.stderr
        rows=[json.loads(s) for s in r.stdout.splitlines()];assert len(rows)=={'legacy':12,'extended':18,'insert':8}[a.mode]
        manifest['runs'].append({'block':block,'variant':variant,'seconds':time.time()-started,'command':cmd})
        (out/'manifest.json').write_text(json.dumps(manifest,indent=2));print(block,variant,f'{time.time()-started:.2f}s',flush=True)
