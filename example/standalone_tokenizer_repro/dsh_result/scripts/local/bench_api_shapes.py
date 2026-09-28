import json, statistics, time
from transformers import AutoTokenizer
TOKDIR='/home/suine/projects/mindvault/example/standalone_tokenizer_repro/tokenizer'
fx=json.load(open('/home/suine/projects/mindvault/example/standalone_tokenizer_repro/fixture.json'))
text=fx['texts'][0]; kw=fx['kwargs']
tok=AutoTokenizer.from_pretrained(TOKDIR, local_files_only=True); raw=tok.backend_tokenizer
M=[('① transformers __call__', lambda: tok([text], **kw)),
   ('② raw encode (单条,char)', lambda: raw.encode(text, add_special_tokens=False)),
   ('③ raw encode_batch (1,char)', lambda: raw.encode_batch([text], add_special_tokens=False)),
   ('④ raw encode_batch_fast (无 offsets)', lambda: raw.encode_batch_fast([text], add_special_tokens=False))]
for _,f in M:
    for _ in range(10): f()
rounds={n:[] for n,_ in M}
for r in range(12):
    order=M[r%len(M):]+M[:r%len(M)]
    if r%2: order=list(reversed(order))
    for n,f in order:
        ts=[]
        for _ in range(5):
            t=time.perf_counter(); f(); ts.append((time.perf_counter()-t)*1e3)
        rounds[n].append(statistics.median(ts))
base=None
print(f"{'方法':38s} {'p50(轮中位数)':>14s}  相对①")
for n,_ in M:
    v=statistics.median(rounds[n]); lo,hi=min(rounds[n]),max(rounds[n])
    if base is None: base=v; rel='—'
    else: rel=f"{100*(v/base-1):+6.1f}%"
    print(f"{n:38s} {v:10.3f} ms   {rel}   （轮间 {lo:.1f}~{hi:.1f}）")
