#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
export RUSTUP_HOME="$OLD/.rustup"
export CARGO_HOME="$W/.cargo-sysroot"
export PATH="$OLD/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=12
export RUSTC_BOOTSTRAP=1
cat > "$W/stdlib-patch.toml" <<EOF
[patch.crates-io]
hashbrown = { path = "$W/hashbrown" }
EOF
for variant in "$@"; do
  stripped="${variant%-private}"
  backend="${stripped%-fused}"
  extra=""
  if [ "$stripped" != "$backend" ]; then extra="--cfg hb_fused_insert"; fi
  if [ "$variant" != "$stripped" ]; then extra="$extra --cfg hb_private_scratch"; fi
  export CARGO_TARGET_DIR="$W/target-std-$variant"
  export RUSTFLAGS="-Zforce-unstable-if-unmarked -Cembed-bitcode=yes -Ctarget-cpu=generic -Ctarget-feature=+sve -Cforce-frame-pointers=yes --cfg hb_backend=\"$backend\" $extra -Aunexpected_cfgs"
  cargo +1.98.1 build --manifest-path "$W/rust-source/library/sysroot/Cargo.toml" \
    --config "$W/stdlib-patch.toml" --profile dist --target aarch64-unknown-linux-gnu --features backtrace --message-format=json \
    > "$W/artifacts/stdlib-$variant-build.jsonl" 2> "$W/logs/stdlib-$variant.log"
  python3 "$W/scripts/assemble_sysroot.py" "$variant"
  RUSTC_BOOTSTRAP= rustc +1.98.1 --edition=2021 --target aarch64-unknown-linux-gnu \
    --sysroot "$W/sysroots/$variant" -Copt-level=3 -Ctarget-feature=+sve \
    "$W/probe/std_probe.rs" -o "$W/artifacts/std-probe-$variant"
  python3 "$OLD/scripts/run_with_vl.py" 32 "$W/artifacts/std-probe-$variant"
done
