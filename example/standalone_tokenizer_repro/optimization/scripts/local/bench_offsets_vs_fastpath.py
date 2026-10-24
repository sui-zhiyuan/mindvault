import json, statistics, time
from tokenizers import Tokenizer
TOK='/home/suine/projects/mindvault/example/standalone_tokenizer_repro/tokenizer/tokenizer.json'
FIX='/home/suine/projects/mindvault/example/standalone_tokenizer_repro/fixture.json'
tok=Tokenizer.from_file(TOK)
fx=json.load(open(FIX)); text=fx['texts'][0]; want=fx['token_ids'][0]
print('方法:', [m for m in dir(tok) if 'encode' in m])
def bench(fn,n=30,warm=10):
    for _ in range(warm): fn()
    ts=[]
    for _ in range(n):
        t=time.perf_counter(); r=fn(); ts.append((time.perf_counter()-t)*1e3)
    return statistics.median(ts), r
k=dict(add_special_tokens=False)
p_char,e_char = bench(lambda: tok.encode(text, **k))
p_fast,e_fast = bench(lambda: tok.encode_fast(text, **k)) if hasattr(tok,'encode_fast') else (float('nan'),None)
p_bchar,_ = bench(lambda: tok.encode_batch([text], **k))
p_bfast,_ = bench(lambda: tok.encode_batch_fast([text], **k))
print(f"\n输入: {len(text)} 字符 / {len(text.encode())} 字节 / 期望 {len(want)} tokens\n")
print(f"encode            (char offsets) : {p_char:7.3f} ms   ids 正确={e_char.ids==want}")
if e_fast is not None:
    print(f"encode_fast       (无 offsets)   : {p_fast:7.3f} ms   ids 正确={e_fast.ids==want}  相对 char: {100*(p_fast/p_char-1):+.1f}%")
print(f"encode_batch      (char offsets) : {p_bchar:7.3f} ms")
print(f"encode_batch_fast (无 offsets)   : {p_bfast:7.3f} ms  相对 batch char: {100*(p_bfast/p_bchar-1):+.1f}%")
print(f"\noffsets: char 路径前 3 个 = {e_char.offsets[:3]}")
e2 = e_fast if e_fast is not None else tok.encode_batch_fast([text], **k)[0]
print(f"offsets: fast 路径前 3 个 = {e2.offsets[:3]}   (len={len(e2.offsets)})")
