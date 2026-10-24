# ipc-rootcause — Tokenizer IPC 根因分析（Top-down）

> 2026-09-26｜ARM-A（鲲鹏 920B / TaiShan V120）与 EPYC 9654 对照｜主报告：[`字节tokenizer IPC根因分析.md`](字节tokenizer%20IPC根因分析.md)（325 行）
> 一句话：原始用例的主要成本是**高频分配/释放**与**字符 offset 映射的建表、哈希、扩容、查询**；已拿到端到端因果证据的两个入口是分配器替换（−28.95%）和 SipHash 状态读的 `ldp` 拆分（−3.70%）。

## 这份材料回答什么

对原始 tokenizer 用例（同一 58,834 字符输入）做自顶向下归因：先用 Top-down 分类判断方向（Retiring 45.86% / Backend Bound 41.32%），再用调用链采样定位软件热点，最后用受控微测和行为消融确认机制。它回答“为什么 ARM 慢、慢在哪些具体路径”，不负责给出完整的生产补丁。

## 结论（按证据强度）

**已确认端到端收益**

1. **分配器与堆布局**：分配/释放是最大热点（ARM 36.70% / AMD 29.12% 采样周期）。两机分配请求几乎相同（每次 295,605 次 malloc、135,244,000 字节），但 glibc 路径在 ARM 上更贵；换同版本 mimalloc 2.4.4 后 ARM 编码再降 **28.95% [28.45,29.51]**，两机指令差从 6.52% 缩到 0.37%。
2. **SipHash 元数据的写读形态**：`reserve/resize → hasher 回调 → RandomState::hash_one → SipHash::write` 中，通用 `write` 未内联，状态经栈写回（`str` + `stp`），随后一条 `ldp` 跨这两个 store 来源读取。拆成两条 `ldr` 后固定增长微测 83.42 → 60.57 cycles/迁移项；真实 wheel 只拆这一处，端到端 **−3.70% [3.01,4.43]**，重定位对照无显著变化。

**机制已确认、但端到端份额未量化**

3. **hash → 桶地址的依赖链**：hash 结果先算出新 ctrl 地址，再有候选 key 读取；诊断性预读 hash 使 ARM 增长项 67.92 → 58.73 cycles（**−13.34%**），AMD 无显著改善。oracle 预读不能部署，13.34% 不是生产收益。
4. **组合**：LDP 拆分 + mimalloc 使 ARM 编码 p50 从 38.343 ms 到 26.288 ms，配对降幅 **31.96% [31.32,32.65]**。

**剩余差距**：统一分配器并关闭 THP 后，两机指令量只差 0.38%，ARM 仍多 **16.08%** 周期、整体 IPC 低 **13.52%**（2.902 vs 3.356）。该差额尚未唯一分解到芯片内部转发、重放或执行端口。

## 已排除的解释

| 解释 | 结果 |
|---|---|
| ARM 普通 load 更慢 / 缓存更小 | 16KiB 依赖链两机约 4 周期，256KiB ARM 10.04 / AMD 14.11，互有优劣 |
| ARM 不利用内存推测 | 两机禁用相关推测后都变慢，程度随访问形态变化 |
| 加大 glibc tcache 即可 | tcache_count=4096 使两机都变慢 |
| 线程池/并行设置是主因 | 原脚本串行 await；Rayon=1 对 ARM 无显著改善 |
| 写回/THP 是主因 | 关 THP 后 mimalloc 收益仍在（周期变化 +0.87%，区间含零） |

## 与归档内其他轮次的关系

- 与 [`sve-wide/`](../sve-wide/) 同机：两边都证明“加宽 NEON/SVE 组探测”不解决这个负载。
- 报告 §11 提出的“把 byte→char 的稠密映射从 `HashMap` 换成连续 `Vec<usize>`”是**方向性建议、未实测**；两天后 [`optimization/`](../optimization/) 把它做成 46 行补丁并实测 −24%。
- 报告推荐的 mimalloc 与 `optimization/` 的 P1/LTO **没有在同一轮组合实测**。
- 报告的可识别 hash 函数 self 仅约 ARM 3.54M / AMD 4.61M 周期，因此剩余差距不能全算到 SipHash 本体。

## 证据缺口（重要）

本目录只有总结报告。正文的相对链接指向**未随包提供**的证据包：

- `../x86_compare/`、`../hb_rootcause/`、`../growth_mechanism/`、`../e2e_pair_load/`、`../residual_mechanisms/`、`../allocator_trace/`
- `mimalloc_arm_fastpath.asm.txt`、`mimalloc_x86_fastpath.asm.txt`、`mimalloc_fastpath_proof.json`、`tokenizer_topdown_report_facts.json`、`tokenizer_residual_rootcause_20260926.tar.gz`

这些链接在本归档中不可达（它们原本相对 `results/other_updates/` 编写）。引用的数值以报告正文表格为准，无法在本归档内复核到原始证据。
