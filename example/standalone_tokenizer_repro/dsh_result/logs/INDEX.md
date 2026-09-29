# 过程日志索引

全部为本轮实验的原始日志（未经整理），按类别列出。

## ARM（192.168.41.50）— 端到端与微测

| 文件 | 内容 |
|---|---|
| `e2e-opt.log` | 方案 P1 补丁 vs stock：10 block × 2 变体（20 次进程，每行含耗时） |
| `e2e-feat.log` | 编译参数变体 v1-native / v2-feat / v3-featvl：10 block × 4 变体（40 次） |
| `e2e-best.log` | neon8 vs opt-feat（P1+native）vs v4-lto（native+LTO）：10 block × 3 变体（30 次） |
| `e2e-optlto.log` | **最优配置** neon8 vs opt-lto（P1+native+LTO）：10 block × 2 变体（20 次） |
| `e2e.log` | SVE 宽组复现：A/A 3 block×2 + 10 block × 5 后端（56 次进程） |
| `micro-run.log` | 微测 legacy/extended/insert 三档 × 10 轮 × 5 后端 |
| `micro-insert2.log` | insert 档在机器空闲时重跑（用于与归档对账，见 04_MEASUREMENTS §三） |
| `wheels.log` | 5 个 SVE 变体的 wheel 构建 + venv 创建 + 冒烟 |
| `opt-wheel.log` | 方案 P1 补丁 wheel 构建 |
| `build-lto.log` | **LTO 构建日志：含 `-Clto` 放进 RUSTFLAGS 的报错与最终用 Cargo profile 的正确做法** |

## x86（本机）— 基准与构建

| 文件 | 内容 |
|---|---|
| `x86-build-patched-wheel.log` | 本机 maturin 构建补丁 wheel |
| `../measurements/local_x86/ab_stock_vs_patched.txt` | stock vs 补丁 交错配对原始输出（6 轮） |
| `../measurements/local_x86/ab3.txt` | PyPI stock / 自编 stock / 自编补丁 三方交错配对原始输出（5 轮） |
| `../measurements/local_x86/dump_{stock,patched}_x86.json` | 19 用例输出等价性导出（两者逐字节相同） |

## 编译参数

| 文件 | 内容 |
|---|---|
| `VARIANT_FLAGS.md` | **各变体的精确 RUSTFLAGS / Cargo profile / 源码**（复现必读） |
| `b-v1.log` / `b-v2.log` / `b-v3.log` | v1-native / v2-feat / v3-featvl 的构建日志（v3 含唯一的 LLVM 后端参数） |

## IPC 测量（详见 `06_IPC_ANALYSIS.md`）

| 文件 | 内容 |
|---|---|
| `../measurements/ipc/` | perf stat 原始计数、双事件采样数据、按函数 IPC 归因输出 |
| `../scripts/remote/perf_loop.sh` | 一键复测（三组 perf stat + 扣启动段） |
| `../scripts/remote/ipc_agg.py` | 双事件按符号聚合 IPC |

## 环境搭建（要点，日志未全量保留）

- 私有工具链：`rustup toolchain install 1.98.1 --profile minimal --component rust-src`（走 rsproxy 镜像），随后整份拷贝到 `~/hb/toolchain-1.98.1` 以**彻底脱离 rustup**——因为 rustup 管理的副本在并发/被中断的安装下损坏过两次，且 shim 会触发慢速在线补装，污染构建。
- 定制 sysroot：`build_sysroots.sh <变体>`（`RUSTC_BOOTSTRAP=1` 编 std，`[patch.crates-io] hashbrown = { path = ... }`），产物经 `std_probe` 验证 `HashSet<u8>::capacity(1)` = 7/14/14/28/28。
- 依赖：`cargo fetch --locked`（rsproxy 镜像）后离线构建；`clean-wheel-Cargo.lock` 全程未变。
