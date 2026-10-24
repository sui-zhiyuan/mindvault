#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
OLD=/root/hashbrown-sve-20260924
export RUSTUP_HOME="$OLD/.rustup"
export CARGO_HOME="$OLD/.cargo"
export RUSTUP_TOOLCHAIN=1.98.1
export PATH="$OLD/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=12
unset RUSTC_BOOTSTRAP
for backend in "$@"; do
  export CARGO_TARGET_DIR="$W/target-micro-$backend"
  export RUSTFLAGS="--sysroot=$W/sysroots/$backend -Ctarget-cpu=generic -Ctarget-feature=+sve -Cforce-frame-pointers=yes"
  cargo +1.98.1 build --manifest-path "$W/micro/Cargo.toml" --release --offline --target aarch64-unknown-linux-gnu > "$W/logs/micro-build-$backend.log" 2>&1
  cp "$CARGO_TARGET_DIR/aarch64-unknown-linux-gnu/release/std-hashbrown-wide-micro" "$W/artifacts/micro-$backend"
done
