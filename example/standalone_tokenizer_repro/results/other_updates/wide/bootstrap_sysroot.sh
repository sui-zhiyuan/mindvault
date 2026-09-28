#!/usr/bin/env bash
set -euo pipefail
export OLD=/root/hashbrown-sve-20260924
export WIDE=/root/hashbrown-wide-20260924
export RUSTUP_HOME="$OLD/.rustup"
export CARGO_HOME="$WIDE/.cargo-sysroot"
export PATH="$OLD/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=12
export RUSTC_BOOTSTRAP=1
mkdir -p "$WIDE/logs" "$WIDE/artifacts" "$CARGO_HOME"
STOCK="$RUSTUP_HOME/toolchains/1.98.1-aarch64-unknown-linux-gnu"
if ! test -d "$WIDE/rust-source/library"; then
  mkdir -p "$WIDE/rust-source"
  cp -a "$STOCK/lib/rustlib/src/rust/library" "$WIDE/rust-source/library"
fi
cat > "$CARGO_HOME/config.toml" <<EOF
[source.crates-io]
replace-with = "rust-vendor"
[source.rust-vendor]
directory = "$STOCK/lib/rustlib/src/rust/library/vendor"
[net]
offline = true
EOF
export CARGO_TARGET_DIR="$WIDE/target-std-stock"
export RUSTFLAGS="-Zforce-unstable-if-unmarked -Cembed-bitcode=yes -Ctarget-cpu=generic -Ctarget-feature=+sve -Cforce-frame-pointers=yes"
cargo +1.98.1 build --manifest-path "$WIDE/rust-source/library/sysroot/Cargo.toml" \
  --profile dist --target aarch64-unknown-linux-gnu --features backtrace --locked -v
