> 注意：本文是最早的 baseline（旧二进制、旧口径）。后续修正与最终数值见 [`v2/README.md`](v2/README.md) 与 [`FINDINGS.md`](FINDINGS.md)。

# HashMap 最小复现：2026-10-24 基线（已按复核意见修正）

## 已核验的源码事实

来源：`sve-wide/sources/tokenizers-clean-v0.21.4.tar.gz`（本归档自带，v0.21.4 原版）。

- `tokenizers/src/tokenizer/mod.rs:881`：`encode_single_sequence(sequence, 0, OffsetType::Char)` —— **Char 偏移无条件启用**，调用方不请求 `offset_mapping` 也会建表。
- `tokenizer/pre_tokenizer.rs:136` `into_encoding` → `:148` `BytesToCharOffsetConverter::new(&self.original)`：对 UTF-8 **每个字节**插入一条 `HashMap<usize,usize>`，本输入 78,960 条。
- `:183` 对**每个 token** 调一次 `convert`；`convert`（`:290`）做 **2 次 `get`**（start/end），末尾 token 走 3 次。
- 因此 25,600 tokens ≈ **51,200 次 get**，不是 25,600；`--stride 3`（26,320 次）只有真实查询量的一半左右，且访问模式不同。
- Rust std SipHash-1-3 的 round 是 **add / rotate_left / xor**（`library/core/src/hash/sip.rs:78-92`），没有乘法；旧报告“64 位乘/移/异或链”的表述需更正。

## 方法

| | ARM | x86 |
|---|---|---|
| 主机 | 鲲鹏 920B / TaiShan V120，固定 2.9 GHz | WSL2，i7-12700，无硬件 PMU |
| 频率 | `scaling_cur_freq = 2900000`，performance，boost=0 | mul/chase 标定 4.35–4.58 GHz（逐 CPU；取 4.45） |
| 周期 | `perf stat` 实测 cycles/instructions | cycles ≈ 墙钟 × 标定频率（**估计值，非 PMU 实测**） |
| 工具链 | rustc 1.98.0（`rustup run 1.98.0`） | rustc 1.98.0 |
| 参数 | n=78,960，warmup=3，15 reps | n=78,960，warmup=3，20 reps × 2 轮 |

`--hasher`：`std` = RandomState(SipHash-1-3)；`mix` = splitmix 风格 usize 哈希（近随机桶布局，用于扣除 SipHash 算术）；`identity` = hash(key)=key（顺序布局退化，仅作上界参考）。

## 数据（中位数）

| hasher | 阶段 | ARM ms | ARM cycles | x86 ms | x86 cycles(est) | ARM/x86 |
|---|---:|---:|---:|---:|---:|---:|
| std | build | 4.045 | 11.73 M | 2.479 | 11.03 M | 1.06 |
| std | query | 2.008 | 5.82 M | 0.869 | 3.87 M | **1.51** |
| std | drop | 0.316 | 0.92 M | 0.118 | 0.53 M | 1.74 |
| mix | build | 2.815 | 8.16 M | 1.804 | 8.03 M | 1.02 |
| mix | query | 0.831 | 2.41 M | 0.305 | 1.36 M | **1.78** |
| identity | build | 2.190 | 6.35 M | 1.510 | 6.72 M | 0.95 |
| identity | query | 0.220 | 0.64 M | 0.121 | 0.54 M | 1.18 |

SipHash 增量（std − mix）：build ARM 3.57 M / x86 3.01 M；query ARM 3.41 M / x86 2.51 M。

ARM `perf stat` IPC（10 reps 区域，含进程启动）：std **1.79**，mix **1.57**。x86 无 PMU，IPC 只能估计。

## 当前可支撑的结论

1. **插入/扩容/分配路径在这条最小用例里不是差距来源**：三种 hasher 的 build 周期比在 0.95–1.06 之间。注意这不否定真实 tokenizer 里分配器的重要性——真实负载的分配量（NFC/正则/临时缓冲）远大于本用例。
2. **差距集中在查询路径，且与桶布局有关**：随机布局的 mix 为 1.78×，SipHash 的 std 为 1.51×，顺序布局的 identity 只有 1.18×。
3. **SipHash 算术本身两机接近**（增量 3.57 vs 3.01 M；3.41 vs 2.51 M cycles），所以“ARM 慢”不能主要归给 SipHash 的算术链；更像是**随机访存/探测/依赖链**在 ARM 上更贵。
4. 以上均为最小用例结论，未验证真实 tokenizer（78,960 插入 + ~51,200 查询 + NFC/正则/BPE）。

## 未解释/待核实

- mix 的 1.78× 具体机制（L1 未命中 vs 依赖链 vs dTLB）——需要 `perf stat` 分组事件与 ARM 侧逐阶段计数。
- x86 cycles/IPC 为估计值：频率标定 ±3%，且无法用 PMU 交叉验证。
- 真实查询模式尚未使用：当前 stride 与 token 边界分布不同，需用 fixture 真实 token 字节偏移驱动查询。
- 两机 cache 层级不同（ARM L1d 64 KB；i7-12700 L1d 48 KB），需在解释中区分。
- 旧报告中的函数 IPC 来自采样点数比（非函数门控硬计数），只能作热点参考；其“非停顿 IPC≈4.9”“49 cycles/miss”等推导不成立，本基线不引用。

## 原始数据

- `arm/`：`arm_{std,mix,identity}.json`、`p_{std,mix}.txt`（perf 原始输出）、`env.txt`
- `x86/`：`{std,mix,identity}_r{1,2}.json`、`calib_by_cpu.txt`、`env.txt`
- 复现命令见 [`commands.md`](commands.md)。
