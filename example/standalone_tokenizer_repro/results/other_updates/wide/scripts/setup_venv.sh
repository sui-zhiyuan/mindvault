#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
variant="$1"
"$OLD/venv-build/bin/python" -m venv "$W/venv-$variant"
"$W/venv-$variant/bin/python" -m pip install -r "$OLD/artifacts/python-requirements.lock"
"$W/venv-$variant/bin/python" -m pip install --no-deps --force-reinstall "$W"/artifacts/wheels-$variant/*.whl
"$W/venv-$variant/bin/python" -m pip check
python3 "$OLD/scripts/run_with_vl.py" 32 numactl --physcpubind=0,2,4,6,8,10,12,14 --membind=0 "$W/venv-$variant/bin/python" \
  "$OLD/standalone_tokenizer_repro/run_variants.py" --rounds 1 --warmup 1 --samples 3 \
  --output-dir "$W/results/smoke-$variant"
