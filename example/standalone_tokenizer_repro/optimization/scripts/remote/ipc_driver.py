import json, sys, time
from transformers import AutoTokenizer
D='/home/wheel/hb/shared/standalone_tokenizer_repro'
fx=json.load(open(f'{D}/fixture.json')); text=fx['texts'][0]; kw=fx['kwargs']
tok=AutoTokenizer.from_pretrained(f'{D}/tokenizer', local_files_only=True)
N=int(sys.argv[1])
for _ in range(10): tok([text], **kw)
t=time.perf_counter()
for _ in range(N): tok([text], **kw)
dt=time.perf_counter()-t
print(f"LOOP N={N} {dt:.3f}s {dt/max(N,1)*1e3:.2f} ms/iter", flush=True)
