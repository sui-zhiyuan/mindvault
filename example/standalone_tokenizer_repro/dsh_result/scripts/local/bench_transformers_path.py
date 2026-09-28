import json, statistics, time
from transformers import AutoTokenizer
from tokenizers import Tokenizer
TOKDIR='/home/suine/projects/mindvault/example/standalone_tokenizer_repro/tokenizer'
fx=json.load(open('/home/suine/projects/mindvault/example/standalone_tokenizer_repro/fixture.json'))
text=fx['texts'][0]; want=fx['token_ids'][0]
kw=fx['kwargs']
tok=AutoTokenizer.from_pretrained(TOKDIR, local_files_only=True)
raw=tok.backend_tokenizer
def bench(fn,n=30,warm=10):
    for _ in range(warm): fn()
    ts=[]
    for _ in range(n):
        t=time.perf_counter(); r=fn(); ts.append((time.perf_counter()-t)*1e3)
    return statistics.median(ts), r
p_tf,r_tf   = bench(lambda: tok([text], **kw))                      # 原始案例：transformers batch 路径
p_raw,_     = bench(lambda: raw.encode(text, add_special_tokens=False))        # 单条 + char offsets
p_rawb,_    = bench(lambda: raw.encode_batch([text], add_special_tokens=False))# batch=1 + char offsets
p_fast,_    = bench(lambda: raw.encode_batch_fast([text], add_special_tokens=False))
print(f"基准（期望 {len(want)} tokens）\n")
print(f"① transformers __call__（batch 路径 + char offsets）: {p_tf:7.3f} ms  ids正确={r_tf['input_ids'][0]==want}")
print(f"② 原始 encode（单条 + char offsets）              : {p_raw:7.3f} ms  ids正确={raw.encode(text,add_special_tokens=False).ids==want}  {100*(p_raw/p_tf-1):+.1f}%")
print(f"③ 原始 encode_batch（batch=1 + char offsets）     : {p_rawb:7.3f} ms  {100*(p_rawb/p_tf-1):+.1f}%")
print(f"④ 原始 encode_batch_fast（batch=1，无 offsets）   : {p_fast:7.3f} ms  ids正确={raw.encode_batch_fast([text],add_special_tokens=False)[0].ids==want}  {100*(p_fast/p_tf-1):+.1f}%")
