import json,statistics,sys,math
from pathlib import Path
sys.path.insert(0,'/home/wheel/hb/shared/scripts')
from paired_stats import summarize
root=Path(sys.argv[1]);baseline=sys.argv[2] if len(sys.argv)>2 else 'neon8'
m=json.loads((root/'manifest.json').read_text());data={}
for v in m['variants']:
    data[v]=[{r['case']:r for r in map(json.loads,(root/f'{i:02d}-{v}.jsonl').read_text().splitlines())} for i in range(m['rounds'])]
report={}
for variant in m['variants']:
    if variant==baseline: continue
    report[variant]={}
    for case in data[baseline][0]:
        pairs=[(statistics.median(a[case]['ns']),statistics.median(b[case]['ns'])) for a,b in zip(data[baseline],data[variant])]
        r=summarize(pairs);r['baseline_median']=r.pop('neon_median');r['candidate_median']=r.pop('sve_median')
        report[variant][case]=r
    changes=[-r['time_reduction_pct'] for r in report[variant].values()]
    print(variant,'vs',baseline,'range',round(min(changes),2),round(max(changes),2),'% time change')
    for case,r in report[variant].items():
        if 'string' in case or '_p85_miss' in case or case in ('lookup_foldhash_random','lookup_std_random'):
            print(case,round(r['baseline_median'],3),'->',round(r['candidate_median'],3),round(-r['time_reduction_pct'],2),'%')
(root/f'summary-vs-{baseline}.json').write_text(json.dumps(report,indent=2))
