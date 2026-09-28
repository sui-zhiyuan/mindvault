# 各变体精确编译参数（复现用）

所有变体都由 `scripts/remote/build_feat_wheel.sh <tag> "<EXTRA>"` 构建，RUSTFLAGS 统一为：

```
RUSTFLAGS = "--sysroot=$W/sysroots/neon8 <EXTRA> -Cforce-frame-pointers=yes"
```

（`$W=/home/wheel/hb/wide`；`--sysroot` 指向为 neon8 后端组装的定制 std，注入改过的 hashbrown。）

| 变体 | `<EXTRA>`（除 sysroot/frame-pointers 外） | 源码 | 额外 |
|---|---|---|---|
| `neon8`（基线） | `-Ctarget-cpu=generic -Ctarget-feature=+sve` | `tokenizers-clean`（stock） | 归档脚本的原始 flags |
| `v1-native` | `-Ctarget-cpu=native` | stock | |
| `v2-feat` | `-Ctarget-cpu=native -Ctarget-feature=+i8mm,+bf16,+dotprod,+rdm,+rcpc,+fcma,+fhm,+flagm,+flagm2,+sb,+ssbs,+dit,+ecv` | stock | 按 `cpuinfo` 补齐 |
| `v3-featvl` | v2 + `-Cllvm-args=-aarch64-sve-vector-bits-min=256` | stock | **唯一验证过的 LLVM 后端参数** |
| `v4-lto` | v2（LTO 不放在 RUSTFLAGS） | stock + `bindings/python/Cargo.toml` 增加 `[profile.release] lto="fat" codegen-units=1` | |
| `opt` | `-Ctarget-cpu=generic -Ctarget-feature=+sve` | `tokenizers-opt`（P1 补丁） | |
| `opt-feat` | v2 的 EXTRA | `tokenizers-opt` | |
| `opt-lto` | v2 的 EXTRA | `tokenizers-opt` + LTO/CGU profile | **最优配置** |

注意事项（复现时容易踩）：

- `-Clto` **不能**写进 `RUSTFLAGS`（会与依赖的 `-C embed-bitcode=no` 冲突）；必须放在**被构建包**的 `[profile.release]`。
- tokenizers 仓库**根目录没有 `Cargo.toml`**，profile 要加在 `bindings/python/Cargo.toml`。
- `-Cllvm-args` 只作用于 Rust 编译的 crate；`oniguruma` 是 C（由 `cc` crate 构建），不受其影响。
- `-Ctarget-feature=+sve` 在 Rust 里会连带启用 `fp16`（`--print cfg` 可见 `fp16 neon sve`）。
