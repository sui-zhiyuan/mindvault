"""Run each Rayon configuration in a fresh process; return nonzero on failure."""
import argparse, os, sys, json, subprocess
from pathlib import Path
p = Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument('--rounds', type=int, default=3)
parser.add_argument('--warmup', type=int, default=10)
parser.add_argument('--samples', type=int, default=30)
parser.add_argument('--output-dir', type=Path, default=p/'output')
a = parser.parse_args()
if min(a.rounds, a.samples) < 1 or a.warmup < 0:
    parser.error('rounds/samples must be positive; warmup must be nonnegative')
a.output_dir.mkdir(parents=True, exist_ok=True)
results = {}
for variant, threads in [('default', None), ('rayon1', '1'), ('rayon8', '8')]:
    env = os.environ.copy()
    env.pop('RAYON_NUM_THREADS', None)
    env.pop('TOKENIZERS_PARALLELISM', None)
    env['HF_HUB_OFFLINE'] = '1'
    env['TRANSFORMERS_OFFLINE'] = '1'
    if threads is not None:
        env['RAYON_NUM_THREADS'] = threads
    cmd = [sys.executable, str(p/'test_tokenizer.py'), '--tokenizer', str(p/'tokenizer'),
           '--fixture', str(p/'fixture.json'), '--output', str(a.output_dir/(variant+'.json')),
           '--variant', variant, '--rounds', str(a.rounds), '--warmup', str(a.warmup),
           '--samples', str(a.samples)]
    with (a.output_dir/(variant+'.log')).open('w') as f:
        result = subprocess.run(cmd, env=env, stdout=f, stderr=subprocess.STDOUT)
    results[variant] = result.returncode
    print(variant, 'PASS' if result.returncode == 0 else 'FAIL', flush=True)
    if result.returncode:
        break
(a.output_dir/'complete.json').write_text(json.dumps(results, indent=2)+'\n')
sys.exit(0 if len(results) == 3 and all(v == 0 for v in results.values()) else 1)
