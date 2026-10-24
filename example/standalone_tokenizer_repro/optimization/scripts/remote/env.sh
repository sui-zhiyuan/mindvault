export HB_WORK=/home/wheel/hb/shared
export CARGO_HOME="$HB_WORK/.cargo"
export RUSTUP_HOME="$HB_WORK/.rustup"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_BUILD_JOBS=12
export RUSTFLAGS="-C target-cpu=generic -C target-feature=+sve -C force-frame-pointers=yes"
