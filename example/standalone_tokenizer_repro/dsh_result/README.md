# dsh_result — 长文本 tokenizer 优化：优化点、测试结果与过程日志

> 生成：2026-09-28
> 对象负载：`example/standalone_tokenizer_repro`（Qwen2.5-0.5B fast tokenizer，batch=1，58,834 字符 / 78,960 字节 / 25,600 tokens）
> **范围声明**：优化目标只有 **aarch64 生产环境**（`192.168.41.50`）。目录中的 x86 数值**仅是"原始案例 aarch64 比 x86 慢 39%"这一对照所需**，以及用于排除工具链污染的机制验证；**本轮不追求也不评估 x86 侧的优化量**。
>
> 本目录**自成一体**：所有结论、原始数据、脚本、日志都在这里，不依赖 `results/other_updates/`（那里是他人归档的 SVE 宽组实验，本目录只把它当作被复现/被证伪的对象）。

## 一句话结论

**能优化，且不改变任何输出**：`HashMap` → `Vec<u32>` 的 46 行补丁（−24%）+ `lto="fat"`/`codegen-units=1`（−10.3%）+ `target-cpu=native`（−1%）叠加后，aarch64 上 **−29.5%~−30.1%**（37.37 → **26.08 ms**），`ids/offsets/tokens/type_ids/special_tokens_mask` 全程逐字节不变。而"用 SVE 加宽向量"这条路已被**两机实测证伪**。

## 文件导航

| 路径 | 内容 |
|---|---|
| `01_OPTIMIZATION_POINTS.md` | **所有优化点汇总**（16 条，逐条给出机制/证据/收益/输出影响/结论） |
| `02_OPTIMIZATION_REPORT.md` | 完整优化报告（10 章：基线、方法、成本结构、方案细节、证据链、复现、产物） |
| `03_COST_STRUCTURE.md` | 成本结构：两台机器 perf 剖析对照 + inclusive 结构 + 归因推理 |
| `04_MEASUREMENTS.md` | 全部实测数字汇总（ARM / x86 / 微测 / 等价性 / 校验链） |
| `05_PROVENANCE.md` | **出处与归属**：哪些来自他人归档、哪些由本轮提出并验证 |
| `patch/pre_tokenizer_vec.patch` | 方案 P1 的补丁（单文件 46 行） |
| `scripts/local/` | x86 侧基准与等价性脚本（可直接跑） |
| `scripts/remote/` | .50 上的构建/运行/统计脚本（含参数化构建脚本） |
| `measurements/arm_e2e/` | ARM 端到端原始结果：`retest-{opt,feat,best,optlto}-e2e/` 各 10 block × 变体 × 3 配置 |
| `measurements/arm_sve_repro/` | SVE 宽组复现：`retest-e2e/`（13500 次调用）、`retest-aa/` |
| `measurements/micro/` | 微测原始数据：`retest-{legacy,extended,insert,insert2}/` |
| `measurements/local_x86/` | x86 交错配对 A/B 原始输出与等价性导出摘要 |
| `measurements/perf/` | .50 的火焰图（SVG + 折叠栈）与 perf 归因报告 |
| `logs/` | 全部过程日志：构建（含 LTO 报错）、E2E、微测、校验 |

## 环境与可复现性

| | aarch64 主实验机 | x86 对照机 |
|---|---|---|
| 主机 | `192.168.41.50`（HiSilicon `0xd02`，2×80 核 SMT2=320 线程，4 NUMA 节点，L2 1.3MB/核、L3 70MB×4） | 本机 i7-12700 / Python 3.12 |
| SVE | 最大 VL=**256 位**（可用 {128,256}，SVE1，无 SVE2） | 无 |
| OS / Python | openEuler 24.03 SP3 / 3.11.6 | — / 3.12.3 |
| 工具链 | 私有 rustup 工具链 **rustc 1.98.1 (`48a229cea`) / LLVM 22.1.8**（与归档同 commit） | rustc 1.98.0 |
| 依赖 | `transformers==4.48.3`、`tokenizers==0.21.4` | 同 |

- ARM 侧所有 wheel 都用**同一私有工具链 + 同一定制 sysroot** 构建，只改源码或编译参数 → 对照干净。
- 统计口径：每后端独立进程、`numactl --physcpubind=240..254 --membind=3`、每进程 3 轮 ×（10 预热 + 30 正式）；10 个 block、顺序轮转并反转；**进程 p50 配对** → 几何平均 + 10000 次 bootstrap（种子 `20260924`）。
- 每次调用校验完整 token IDs；另用 19 个用例导出 `ids/offsets/tokens/type_ids/special_tokens_mask` 做逐字节比对。

## 复现入口

见 `02_OPTIMIZATION_REPORT.md` 第 8 章与 `scripts/`；远端工作区为 `~/hb/{wide,shared,toolchain-1.98.1}`（含 5 套 sysroot、各变体 wheel 与 venv，可直接复跑）。
