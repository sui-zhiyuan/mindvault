#!/usr/bin/env bash
set -euo pipefail
export TC=/home/wheel/hb/toolchain-1.98.1
export PATH="$TC/bin:$PATH"
export RUSTC="$TC/bin/rustc"
W=/home/wheel/hb/wide
OLD=/home/wheel/hb/shared
variant="$1"
export RUSTUP_HOME="$OLD/.rustup"
export CARGO_HOME="$OLD/.cargo"
export RUSTUP_TOOLCHAIN=1.98.1
export PATH="$TC/bin:$OLD/.cargo/bin:$PATH"
export CARGO="$TC/bin/cargo"
export CARGO_BUILD_JOBS=12
export CARGO_TARGET_DIR="$W/target-wheel-$variant"
export PYO3_PYTHON="$OLD/venv-build/bin/python"
export CARGO_PROFILE_RELEASE_DEBUG=1
export RUSTFLAGS="--sysroot=$W/sysroots/$variant -Ctarget-cpu=generic -Ctarget-feature=+sve -Cforce-frame-pointers=yes"
unset RUSTC_BOOTSTRAP
cd "$W/tokenizers-clean/bindings/python"
if ! test -f "$W/artifacts/clean-wheel-Cargo.lock"; then
  cp "$OLD/artifacts/wheel-Cargo.lock" Cargo.lock
  "$TC/bin/cargo" metadata --offline --format-version 1 > "$W/artifacts/clean-metadata.json"
  cp Cargo.lock "$W/artifacts/clean-wheel-Cargo.lock"
fi
cmp Cargo.lock "$W/artifacts/clean-wheel-Cargo.lock"
mkdir -p "$W/artifacts/wheels-$variant"
"$OLD/venv-build/bin/maturin" build --release --locked --offline --target aarch64-unknown-linux-gnu \
  --interpreter "$PYO3_PYTHON" --out "$W/artifacts/wheels-$variant" --compatibility linux
cmp Cargo.lock "$W/artifacts/clean-wheel-Cargo.lock"
