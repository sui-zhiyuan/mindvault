import json, sys, time
from transformers import AutoTokenizer
D='/home/wheel/hb/shared/standalone_tokenizer_repro'
fx=json.load(open(f'{D}/fixture.json')); text=fx['texts'][0]
tok=AutoTokenizer.from_pretrained(f'{D}/tokenizer', local_files_only=True)
raw=tok.backend_tokenizer
N=int(sys.argv[1])
for _ in range(10): raw.encode_batch([text], add_special_tokens=False)
t=time.perf_counter()
for _ in range(N): raw.encode_batch([text], add_special_tokens=False)
dt=time.perf_counter()-t
print(f"LOOP N={N} {dt:.3f}s {dt/max(N,1)*1e3:.2f} ms/iter", flush=True)
