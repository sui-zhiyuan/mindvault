#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
export RUSTUP_HOME="$OLD/.rustup"
export PATH="$OLD/.cargo/bin:$PATH"
python3 "$W/scripts/assemble_sysroot.py" stock
rustc +1.98.1 --edition=2021 --target aarch64-unknown-linux-gnu \
  --sysroot "$W/sysroots/stock" -Copt-level=3 -Ctarget-feature=+sve \
  "$W/probe/std_probe.rs" -o "$W/artifacts/std-probe-stock"
python3 "$OLD/scripts/run_with_vl.py" 32 "$W/artifacts/std-probe-stock"
ldd "$W/artifacts/std-probe-stock"
