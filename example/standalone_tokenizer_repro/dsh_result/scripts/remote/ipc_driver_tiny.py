import json, sys, time
from transformers import AutoTokenizer
D='/home/wheel/hb/shared/standalone_tokenizer_repro'
kw=json.load(open(f'{D}/fixture.json'))['kwargs']
tok=AutoTokenizer.from_pretrained(f'{D}/tokenizer', local_files_only=True)
text='a'
N=int(sys.argv[1])
for _ in range(10): tok([text], **kw)
t=time.perf_counter()
for _ in range(N): tok([text], **kw)
dt=time.perf_counter()-t
print(f"LOOP N={N} {dt:.3f}s {dt/max(N,1)*1e3:.3f} ms/iter", flush=True)
