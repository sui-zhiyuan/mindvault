# 出处与归属：哪些来自他人归档，哪些来自本轮

> 目的：把"归档已有的诊断"与"本轮提出的方案/实测"分清，避免把别人的观察或自己的工作记错账。
> 核对方式：对 `results/other_updates/`（他人归档）全文检索关键词，逐条对到原文行号。

## 一、他人归档（`results/other_updates/`）已有的内容

| 内容 | 出处 | 说明 |
|---|---|---|
| **诊断**：hashbrown 的 `reserve_rehash`/`insert` 成本主要来自 `BytesToCharOffsetConverter` 逐 UTF-8 字节建表 | `wide/progress.md:12` | 与本轮独立剖析结论一致 |
| **观测**：malloc/free、正则、offset 转换也占明显成本 | `wide_sve_results.md:65` | 仅作**归因提示**，原文明确写"不等同于精确可优化时间比例"；**未**据此提出任何分配器方案 |
| **实验**：std 内 hashbrown 宽组（NEON8/NEON16/SVE16/SVE32/private）的微测与端到端 | `wide/`、`wide_sve_results.md` | 本轮在 .50 上复现了它的主结论（SVE 更慢），并补齐了它未测的项 |
| **结论**：建议保留 NEON8 | `wide_sve_results.md` | 与本轮复现一致 |
| **脚本**：配对 bootstrap（`paired_stats.py`）、`run_with_vl.py`、微基准 `micro` crate、`run_e2e.py`/`run_micro.py` 框架 | `wide/scripts/`、`shared/scripts/` | 本轮的配对设计、VL 固定方式、微测用例均是复用/扩展这套 |

检索结果：归档里搜 `jemalloc|mimalloc|tcmalloc|global_allocator|LD_PRELOAD` = **0 命中**；搜 `Vec<u32>`/前缀和/数组替代 = **0 命中**；构建脚本一律 `-Ctarget-cpu=generic -Ctarget-feature=+sve`，**没有** LTO / native / llvm-args。

## 二、本轮提出并验证的内容

| 优化点 | 归属 | 证据 |
|---|---|---|
| **P1 修复**：`HashMap<usize,usize>` → `Vec<u32>`（去掉哈希表） | **本轮** | 归档有同样的诊断，但改进方向停留在 hashbrown 内部（融合插入/宽组）；本轮改为换数据结构，实测 aarch64 −23.7~24.1% |
| **P2**：`lto="fat"` + `codegen-units=1` | **本轮** | 归档所有 wheel 都是默认 release profile；本轮实测 aarch64 −10.2~10.5% |
| **P3/P13/P14**：`-Ctarget-cpu=native`、固定 VL 的 `llvm-args`、按 cpuinfo 补齐 feature | **本轮** | 归档只有 `generic+sve`；本轮给出各自实测结论（−1% / ≈0 且有风险 / 0） |
| **P5**：无 offsets 快路径（`encode_fast` → `OffsetType::None`） | **本轮** | 读 tokenizers Python 绑定源码发现；x86 对照 −18.9%（aarch64 未实测） |
| **P6**：换分配器（jemalloc/mimalloc/tcmalloc） | **本轮** | 出自本轮自己的 perf 剖析（分配器+memcpy = 46.8%）与通用工程实践；归档仅观测到同方向成本 |
| **P7/P8/P9**：减少临时分配 / NFC 恒等快路径 / 多文档并行 | **本轮** | 基于本轮剖析 |
| **P11/P12/P16 负结果**：线程调参 / API 形态 / 绑核无效 | **本轮** | 交错配对与执行模型测量（进程 CPU/wall ≈ 0.998） |
| **P10 证伪**：SVE 宽组 | **归档提出，本轮两机证伪** | .50 端到端 +2.2~3.3%（9/9 格显著），与归档 .51 的 +2.4~5.1% 同向 |
| **全部实测数字、复现脚本扩展、等价性 19 用例、指令普查** | **本轮** | 见 `04_MEASUREMENTS.md` 与 `measurements/` |

## 三、一句话

他人归档的贡献是**「定位到 hashbrown 建表 + 做完一轮 SVE 宽组实验（结论：保留 NEON8）」**；**方案层面的东西**（去哈希表、LTO、native、无 offsets 路径、分配器、以及全部实测与证伪）**都出自本轮**。
