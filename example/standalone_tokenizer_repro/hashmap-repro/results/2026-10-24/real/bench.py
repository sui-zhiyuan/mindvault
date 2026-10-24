"""Real-tokenizer check: encode() vs encode_batch_fast() on the fixed fixture.

encode() uses OffsetType::Char, so it builds BytesToCharOffsetConverter and
converts offsets; encode_batch_fast skips that path. The delta is an upper
bound on the offset-map share of the real encode.

Run with a Python that has tokenizers==0.21.4 and the tokenizer assets.
"""
import json, time, statistics
from tokenizers import Tokenizer

tok = Tokenizer.from_file("tokenizer/tokenizer.json")
text = json.load(open("fixture.json"))["texts"][0]

def bench(fn, n=30):
    for _ in range(5):
        fn()
    ts = []
    for _ in range(n):
        t = time.perf_counter_ns()
        ids = fn()
        ts.append(time.perf_counter_ns() - t)
    return statistics.median(ts) / 1e6, ids

ids_a = None
def a():
    global ids_a
    ids_a = tok.encode(text).ids
    return ids_a

p50_a, ids_a = bench(a)
print(json.dumps({"mode": "encode", "p50_ms": p50_a, "ntok": len(ids_a), "text_len": len(text)}))

if hasattr(tok, "encode_batch_fast"):
    p50_b, ids_b = bench(lambda: tok.encode_batch_fast([text])[0].ids)
    print(json.dumps({"mode": "encode_batch_fast", "p50_ms": p50_b,
                      "ntok": len(ids_b), "same_ids": ids_b == ids_a}))
else:
    print(json.dumps({"mode": "encode_batch_fast", "available": False}))
