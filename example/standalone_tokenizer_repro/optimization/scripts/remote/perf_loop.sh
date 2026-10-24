#!/usr/bin/env bash
DRV=$1; N=$2; TAG=$3
run(){ env RAYON_NUM_THREADS=1 python3 ~/hb/shared/scripts/run_with_vl.py 32 numactl --physcpubind=240 --membind=3 \
       perf stat -x, -e "$1" -o "$3" -- ~/hb/wide/venv-neon8/bin/python "$DRV" "$2" >/dev/null 2>/dev/null; }
A="cycles,instructions,stalled-cycles-frontend,stalled-cycles-backend"
B="cycles,instructions,L1-dcache-loads,L1-dcache-load-misses,dTLB-load-misses"
C="cycles,instructions,branches,branch-misses,L1-icache-load-misses"
for g in A B C; do eval "E=\$$g"; run "$E" 1 /tmp/${TAG}_${g}1.csv; run "$E" "$N" /tmp/${TAG}_${g}${N}.csv; done
python3 - "$TAG" "$N" <<'PY'
import sys, csv
tag, N = sys.argv[1], int(sys.argv[2])
def load(p):
    d={}
    for row in csv.reader(open(p)):
        if len(row)>=3 and row[2] and not row[0].startswith('#'):
            try: d[row[2].strip()]=float(row[0])
            except ValueError: pass
    return d
res={}
for g in 'ABC':
    a=load(f'/tmp/{tag}_{g}1.csv'); b=load(f'/tmp/{tag}_{g}{N}.csv')
    for k,v in b.items(): res[k]=(v-a.get(k,0))/(N-1)
cyc=res.get('cycles',1); ins=res.get('instructions',0)
print(f"[{tag}] 每次调用（扣除启动段的 {N-1} 次平均）")
print(f"  cycles/iter {cyc:,.0f}   instructions/iter {ins:,.0f}   **IPC = {ins/cyc:.3f}**")
print(f"  分支/iter {res.get('branches',0):,.0f}  未命中 {res.get('branch-misses',0):,.0f} ({100*res.get('branch-misses',0)/max(res.get('branches',1),1):.2f}%)")
print(f"  L1d 载入/iter {res.get('L1-dcache-loads',0):,.0f}  未命中 {res.get('L1-dcache-load-misses',0):,.0f} ({100*res.get('L1-dcache-load-misses',0)/max(res.get('L1-dcache-loads',1),1):.2f}%)   dTLB 未命中 {res.get('dTLB-load-misses',0):,.0f}")
print(f"  iCache 未命中/iter {res.get('L1-icache-load-misses',0):,.0f}")
print(f"  前端停顿 {100*res.get('stalled-cycles-frontend',0)/cyc:.2f}%   后端停顿 {100*res.get('stalled-cycles-backend',0)/cyc:.2f}%")
PY
