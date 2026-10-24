# 长文本 tokenizer 的 ARM 性能案例（归档）

> 整理：2026-10-24｜材料：2026-09-24 ~ 2026-09-29
> 同一份公开 Qwen2.5-0.5B fast tokenizer、同一段 58,834 字符 / 78,960 字节的合成输入，aarch64 比 x86_64 慢约 **39%~40%**（38.60 ms vs 27.66 ms p50）。2026 年 9 月围绕这个现象做了四组彼此独立的工作，结论互相补充，也互相证伪。本目录按轮次归档，每轮的报告与原始数据放在一起。

## 目录

```text
standalone_tokenizer_repro/
├── repro/            可运行复现包：源码、tokenizer 资产、两机历史结果
├── sve-wide/         实验：标准库 hashbrown 宽组 SVE（结论：负收益，保留 NEON8）
├── optimization/     方案：HashMap→Vec + LTO + native（结论：−30%，输出逐字节不变）
├── ipc-rootcause/    分析：Top-down 根因（分配器、SipHash 写读形态、依赖链）
└── hashmap-repro/    最小 HashMap 复现：保留 HashMap，定位查询路径的 L2/MLP 差距（2026-10-24）
```

## 四组工作一览

| 轮次 | 时间 | 机器 | 回答的问题 | 主要结论 |
|---|---|---|---|---|
| `repro/` | 历史数据 | ARM-A + x86 | 差距是否可复现 | 是，三种线程配置稳定 +39%~40% |
| `sve-wide/` | 2026-09-24 | ARM-A | 加宽 hashbrown 组宽能否提速 | 不能：端到端 **+2.4%~+5.1%**；高占用率 miss 微测的收益传不到真实负载 |
| `ipc-rootcause/` | 2026-09-26 | ARM-A + EPYC 9654 | 周期到底花在哪 | 分配/释放 + offset 映射建表/哈希/查询；SipHash 的 STP/LDP 形态 **−3.70%**；换 mimalloc **−28.95%** |
| `optimization/` | 2026-09-28 | ARM-B | 能拿到多少可落地收益 | `HashMap<usize,usize>`→`Vec<u32>` **−24%**，LTO+CGU=1 **−10.3%**，native −1%，叠加 **−29.5%~−30.1%**，输出不变 |

> 两台 ARM 机器不同：ARM-A = 鲲鹏 920B / TaiShan V120（SVE VL=32B），ARM-B = HiSilicon `0xd02`（SVE 最大 256 bit）。各报告正文记录了确切主机与配置。

## 结论地图

### 为什么 ARM 慢

- 主因不是 ISA 或缓存容量：ARM 的普通 load 不天然更慢，两机在同形态微测里互有胜负（见 `ipc-rootcause/` §7）。
- 成本前两位是**高频分配/释放**（ARM 采样 36.7% 周期，AMD 29.1%）与**字符 offset 映射的建表/哈希/扩容/查询**（建表 15.6% + 查询 7.1%），两者有重叠，不能相加。
- 即使调用方不需要 `offset_mapping`，Rust 绑定仍走字符 offset 编码，持续构建 `BytesToCharOffsetConverter` 的 `HashMap<usize, usize>`。
- 在这张表上，std 默认 SipHash 的状态经栈写回、再用一条跨两个 store 来源的 `ldp` 读回；这种读写形态在 ARM 上代价高（拆 `ldr` 可拿 −3.7%）。
- 剩余差距是周期而非指令：统一分配器并关闭 THP 后，两机指令量只差 0.38%，ARM 仍多 16.08% 周期、IPC 低 13.52%。

### 什么有效

| 手段 | 收益 | 证据 |
|---|---|---|
| 换分配器（mimalloc 2.4.4） | ARM **−28.95%** [28.45,29.51] | `ipc-rootcause/` 根因 A |
| 拆 SipHash 状态读的 `ldp` | 端到端 **−3.70%** [3.01,4.43] | `ipc-rootcause/` 根因 B |
| `HashMap<usize,usize>`→`Vec<u32>`（46 行） | **−23.7%~−24.1%** | `optimization/` P1 |
| `lto="fat"` + `codegen-units=1` | **−10.2%~−10.5%** | `optimization/` P2 |
| `-Ctarget-cpu=native` | **−0.6%~−1.1%** | `optimization/` P3 |
| P1+P2+P3 组合 | **−29.5%~−30.1%**（37.37→26.08 ms） | `optimization/` P4 |

### 什么无效或被证伪

- **SVE/NEON 加宽组探测**：ARM-A 上 +2.4%~+5.1%，ARM-B 复现 +2.2%~+3.3%；微测里高占用率 miss 变快，但真实负载没有收益。见 `sve-wide/`。
- 线程/Rayon 调参、改 batch 形态、绕过 transformers 包壳、绑核：均在噪声内。
- 加大 glibc tcache、`-aarch64-sve-vector-bits-min=256`、按 cpuinfo 补 feature：无效或只有代码形态变化。

### 两份“优化”结论的关系

- `ipc-rootcause/` 在 §11 提出“把稠密映射改成连续 `Vec<usize>`”是**方向性建议，未实测**；
- `optimization/` 两天后把它做成 46 行补丁，实测 −24%，并补上 LTO/native；
- 最大杠杆的判断不同：前者指向**分配器**（mimalloc −29%），后者指向**数据结构 + 构建配置**（−30%，未换分配器）。二者不冲突，但**没有在同一轮里组合实测**。

## 阅读顺序

1. 想复现现象 → [`repro/README.md`](repro/README.md)
2. 想直接落地 → [`optimization/README.md`](optimization/README.md)，先看 `01_OPTIMIZATION_POINTS.md`
3. 想知道为什么慢 → [`ipc-rootcause/README.md`](ipc-rootcause/README.md)
4. 想避免走弯路 → [`sve-wide/wide_sve_results.md`](sve-wide/wide_sve_results.md)

## 归档整理说明

本目录由两个来源合并后整理：master 上的 325 行总结报告，以及 `feat/tokenizer-repro` 分支上的完整实验树（含复现包、`dsh_result`、SVE 实验与原始测量）。整理动作：

- **去重**：`feat/tokenizer-repro` 解包时已删除压缩包（合并后不再保留 tar 副本）；删除 `results/other_updates/standalone_tokenizer_repro/` 整份拷贝（与复现包逐字节相同）、`results/other_updates/opt_schemes/`（1002 个文件，除一份更短的旧稿外全部与 `optimization/` 重复）、重复的 SVE 报告与 perf 文件、550 个 0 字节日志。`opt_schemes` 的旧稿内容已被 `optimization/02_OPTIMIZATION_REPORT.md` 覆盖。
- **重排**：原来平铺在 `results/other_updates/` 下、与 `dsh_result/` 并列的各轮材料按轮次归位，报告、脚本、原始数据放进同一目录。
- **改引用**：文档里指向旧路径的引用已更新到新位置。

## 数据缺口与边界

1. `ipc-rootcause/` 只有总结报告；正文引用的子报告（`x86_compare`、`hb_rootcause`、`growth_mechanism`、`e2e_pair_load`、`residual_mechanisms`、`allocator_trace`、`mimalloc_*.asm.txt` 等）**不在本归档**，相关相对链接不可达。
2. 两轮 ARM 实验用了不同机器，stock 基线相差约 2.6%，跨轮的百分比不能直接相减或叠加。
3. 各轮统计口径不同（采样周期占比 / 配对 p50 / 微测 cycles-per-op），不要互相换算。
4. `optimization/` 的分配器方案（P6）未实测；`ipc-rootcause/` 的 mimalloc/LDP 方案没有和 P1/LTO 组合测过。

## 复现

见 [`repro/README.md`](repro/README.md)。复现包自带固定 tokenizer、输入 fixture 与两机历史结果，离线可跑（需先装 Python 依赖）。

## 许可

`repro/tokenizer/` 来自 Qwen2.5-0.5B，Apache-2.0，许可证随包提供（[`repro/tokenizer/LICENSE`](repro/tokenizer/LICENSE)）。
