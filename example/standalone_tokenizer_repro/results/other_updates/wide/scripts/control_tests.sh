#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
export RUSTUP_HOME="$OLD/.rustup"
export PATH="$OLD/.cargo/bin:$PATH"
backend="$1"
extra=()
suffix=""
if [ "${2:-}" = fused ]; then extra=(--cfg hb_fused_insert); suffix=-fused; fi
if [ "${3:-}" = private ]; then extra+=(--cfg hb_private_scratch); suffix="$suffix-private"; fi
rustc +1.98.1 --test --edition=2024 "$W/probe/control_tests.rs" \
  --cfg "hb_backend=\"$backend\"" "${extra[@]}" -Ctarget-feature=+sve -Copt-level=2 \
  -o "$W/artifacts/control-$backend$suffix"
python3 "$OLD/scripts/run_with_vl.py" 32 "$W/artifacts/control-$backend$suffix" --test-threads=1
