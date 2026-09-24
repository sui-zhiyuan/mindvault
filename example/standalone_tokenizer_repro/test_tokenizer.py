"""Standalone synthetic-shape tokenizer regression test and reproducible benchmark.
No inference service imports, model weights, or network requests.
Run each RAYON_NUM_THREADS variant in a fresh process (Rayon initializes once).
"""
import argparse, asyncio, hashlib, importlib.metadata, json, math, os, platform
import statistics, time, unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

ARGS = None
FIXTURE = None
TOKENIZER = None

def fingerprint(ids):
    return hashlib.sha256(json.dumps(ids, separators=(',', ':')).encode()).hexdigest()

def stats(values):
    v = sorted(values)
    return dict(n=len(v), p50_ms=statistics.median(v), p95_ms=v[math.ceil(.95*len(v))-1], min_ms=v[0], max_ms=v[-1])

class TestSyntheticTokenizer(unittest.TestCase):
    def test_synthetic_shape_and_tokens(self):
        self.assertEqual(len(FIXTURE['texts']), 1)
        self.assertEqual(FIXTURE['kwargs'], dict(add_special_tokens=False, padding=False,
            return_attention_mask=False, return_token_type_ids=False, truncation=False))
        self.assertTrue(TOKENIZER.is_fast)
        actual = TOKENIZER(FIXTURE['texts'], **FIXTURE['kwargs']).input_ids
        self.assertEqual(actual, FIXTURE['token_ids'])

async def benchmark():
    rows = []
    # Uses an 8-worker encode executor; sequential requests, no artificial concurrency.
    with ThreadPoolExecutor(max_workers=8, thread_name_prefix='encode_thread') as pool:
        loop = asyncio.get_running_loop()
        for round_id in range(ARGS.rounds):
            for i in range(ARGS.warmup + ARGS.samples):
                start = time.perf_counter_ns()
                def encode():
                    entered = time.perf_counter_ns()
                    cpu_start = time.thread_time_ns()
                    ids = TOKENIZER(FIXTURE['texts'], **FIXTURE['kwargs']).input_ids
                    cpu_end = time.thread_time_ns()
                    ended = time.perf_counter_ns()
                    return ids, entered, ended, (cpu_end-cpu_start)/1e6
                ids, entered, ended, cpu_ms = await loop.run_in_executor(pool, encode)
                finished = time.perf_counter_ns()
                # Validation outside timed interval; a mismatch invalidates the run.
                if ids != FIXTURE['token_ids']:
                    raise AssertionError('Token IDs differ from captured synthetic fixture')
                rows.append(dict(round=round_id, index=i, warmup=i<ARGS.warmup,
                    queue_ms=(entered-start)/1e6, tokenizer_ms=(ended-entered)/1e6,
                    total_ms=(finished-start)/1e6, executor_thread_cpu_ms=cpu_ms))
                await asyncio.sleep(.01)
    measured = [r for r in rows if not r['warmup']]
    keys = ['queue_ms', 'tokenizer_ms', 'total_ms', 'executor_thread_cpu_ms']
    files = {str(f.relative_to(Path(ARGS.tokenizer))): hashlib.sha256(f.read_bytes()).hexdigest()
             for f in Path(ARGS.tokenizer).rglob('*') if f.is_file()}
    result = dict(variant=ARGS.variant, platform=platform.machine(), python=platform.python_version(),
        versions={k:importlib.metadata.version(k) for k in ['transformers', 'tokenizers']},
        env={k:os.getenv(k) for k in ['RAYON_NUM_THREADS','TOKENIZERS_PARALLELISM']},
        executor_max_workers=8,
        shape=dict(batch=len(FIXTURE['texts']), chars=[len(x) for x in FIXTURE['texts']],
                   utf8_bytes=[len(x.encode()) for x in FIXTURE['texts']],
                   token_lengths=[len(x) for x in FIXTURE['token_ids']]),
        token_sha256=fingerprint(FIXTURE['token_ids']), tokenizer_files_sha256=files,
        summary={k:stats([r[k] for r in measured]) for k in keys},
        rounds=[{k:stats([r[k] for r in measured if r['round']==n]) for k in keys} for n in range(ARGS.rounds)], rows=rows)
    Path(ARGS.output).write_text(json.dumps(result, indent=2))
    print('BENCH_RESULT', json.dumps({k:result[k] for k in ['variant','shape','summary']}), flush=True)

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tokenizer', required=True)
    parser.add_argument('--fixture', required=True)
    parser.add_argument('--output', required=True)
    parser.add_argument('--variant', default='default')
    parser.add_argument('--rounds', type=int, default=3)
    parser.add_argument('--warmup', type=int, default=10)
    parser.add_argument('--samples', type=int, default=30)
    ARGS=parser.parse_args()
    FIXTURE=json.loads(Path(ARGS.fixture).read_text())
    from transformers import AutoTokenizer
    TOKENIZER=AutoTokenizer.from_pretrained(ARGS.tokenizer, local_files_only=True)
    suite=unittest.defaultTestLoader.loadTestsFromTestCase(TestSyntheticTokenizer)
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():raise SystemExit(1)
    asyncio.run(benchmark())
