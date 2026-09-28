#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
export RUSTUP_HOME="$OLD/.rustup"
export CARGO_HOME="$OLD/.cargo"
export PATH="$OLD/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=12
export CARGO_TARGET_DIR="$W/target-library-tests"
python3 - <<'PY'
from pathlib import Path
import shutil
w=Path('/root/hashbrown-wide-20260924')
shutil.copytree(w/'hashbrown',w/'hashbrown-tests',dirs_exist_ok=True)
p=w/'hashbrown-tests/Cargo.toml'
# Criterion is only used by benchmarks, not by the library tests being run.
p.write_text('\n'.join(s for s in p.read_text().splitlines() if not s.startswith('criterion ='))+'\n')
PY
STOCK="$RUSTUP_HOME/toolchains/1.98.1-aarch64-unknown-linux-gnu/lib/rustlib/src/rust/library/vendor"
cat > "$W/test-deps.toml" <<EOF
[patch.crates-io]
foldhash = { path = "$STOCK/foldhash-0.2.0" }
EOF
for variant in "$@"; do
  stripped="${variant%-private}"
  backend="${stripped%-fused}"
  extra=""
  if [ "$stripped" != "$backend" ]; then extra="--cfg hb_fused_insert"; fi
  if [ "$variant" != "$stripped" ]; then extra="$extra --cfg hb_private_scratch"; fi
  export RUSTFLAGS="-Ctarget-feature=+sve --cfg hb_backend=\"$backend\" $extra -Aunexpected_cfgs"
  python3 "$OLD/scripts/run_with_vl.py" 32 cargo +1.98.1 test --offline \
    --manifest-path "$W/hashbrown-tests/Cargo.toml" --config "$W/test-deps.toml" --lib \
    > "$W/logs/library-tests-$variant.log" 2>&1
  tail -3 "$W/logs/library-tests-$variant.log"
done
