import json,math,statistics,sys
from pathlib import Path
sys.path.insert(0,'/home/wheel/hb/shared/scripts')
from paired_stats import summarize
root=Path(sys.argv[1]);baseline=sys.argv[2] if len(sys.argv)>2 else 'neon8'
m=json.loads((root/'manifest.json').read_text());assert len(m['runs'])==len(m['variants'])*m['rounds']
data={};oracle=None;calls=0
for v in m['variants']:
    data[v]=[]
    for block in range(m['rounds']):
        d=root/f'{block:02d}-{v}';assert json.loads((d/'complete.json').read_text())=={'default':0,'rayon1':0,'rayon8':0}
        row={}
        for config in ('default','rayon1','rayon8'):
            x=json.loads((d/f'{config}.json').read_text());assert len(x['rows'])==120
            same={k:x[k] for k in ('shape','versions','python','token_sha256','tokenizer_files_sha256')}
            if oracle is None:oracle=same
            assert same==oracle
            measured=[r for r in x['rows'] if not r['warmup']];assert len(measured)==90;calls+=90
            for key,s in x['summary'].items():
                vals=sorted(r[key] for r in measured)
                assert math.isclose(statistics.median(vals),s['p50_ms'],abs_tol=1e-10)
                assert math.isclose(vals[math.ceil(.95*len(vals))-1],s['p95_ms'],abs_tol=1e-10)
            for n in range(3):
                assert sum(r['round']==n and not r['warmup'] for r in x['rows'])==30
                assert sum(r['round']==n and r['warmup'] for r in x['rows'])==10
            row[config]=x
        data[v].append(row)
report={}
for v in m['variants']:
    if v==baseline:continue
    report[v]={}
    for config in ('default','rayon1','rayon8'):
        report[v][config]={}
        for metric in ('tokenizer_ms','total_ms','queue_ms','executor_thread_cpu_ms'):
            pairs=[(a[config]['summary'][metric]['p50_ms'],b[config]['summary'][metric]['p50_ms']) for a,b in zip(data[baseline],data[v])]
            r=summarize(pairs);r['baseline_median']=r.pop('neon_median');r['candidate_median']=r.pop('sve_median');report[v][config][metric]=r
        r=report[v][config]['tokenizer_ms'];lo,hi=r['time_reduction_ci95_pct']
        print(v,config,f"{r['baseline_median']:.4f}->{r['candidate_median']:.4f} ms",f"time change {-r['time_reduction_pct']:+.2f}% [{-hi:+.2f}, {-lo:+.2f}]")
(root/f'summary-vs-{baseline}.json').write_text(json.dumps(report,indent=2))
(root/'validation.json').write_text(json.dumps({'validated':True,'rounds':m['rounds'],'variants':m['variants'],'formal_calls':calls,'oracle':oracle},indent=2))
print('Validated',calls,'formal calls')
