#!/usr/bin/env bash
set -euo pipefail
src="$1"; tag="$2"; shift 2
EXTRA="$*"
W=/home/wheel/hb/wide; OLD=/home/wheel/hb/shared
export TC=/home/wheel/hb/toolchain-1.98.1
export PATH="$TC/bin:$PATH"; export RUSTC="$TC/bin/rustc"
export CARGO_HOME="$OLD/.cargo"; export CARGO="$TC/bin/cargo"; export CARGO_BUILD_JOBS=12
export CARGO_TARGET_DIR="$W/target-wheel-$tag"; export PYO3_PYTHON="$OLD/venv-build/bin/python"
export CARGO_PROFILE_RELEASE_DEBUG=1
export RUSTFLAGS="--sysroot=$W/sysroots/neon8 $EXTRA -Cforce-frame-pointers=yes"
unset RUSTC_BOOTSTRAP
cd "$W/$src/bindings/python"
mkdir -p "$W/artifacts/wheels-$tag"
"$OLD/venv-build/bin/maturin" build --release --locked --offline --target aarch64-unknown-linux-gnu \
  --interpreter "$PYO3_PYTHON" --out "$W/artifacts/wheels-$tag" --compatibility linux
