# 优化点汇总（本轮全部发现）

> **优化目标 = aarch64**（`192.168.41.50`）。所有收益默认指 aarch64 端到端 `tokenizer_ms` p50，相对该机 stock 基线（`neon8` wheel，`-Ctarget-cpu=generic -Ctarget-feature=+sve`）。
> 表中出现的 x86 数值**只作对照与机制验证**（原始案例差距的定义、排除编译器污染），**不是优化目标**。
> 详细原始数据见 `04_MEASUREMENTS.md` 与 `measurements/`。

## 一、速览

| # | 优化点 | 机制 | 实测收益（aarch64） | 改输出? | 结论 |
|---|---|---|---|---|---|
| **P1** | byte→char 映射 `HashMap`→`Vec<u32>` | 单调稠密映射不需要哈希 | **−23.7%~−24.1%**（x86 −18~21%） | 无 | ✅ **采纳**（需上游改 46 行） |
| **P2** | `lto="fat"` + `codegen-units=1` | 跨 crate 内联（本负载由大量小函数/迭代器适配器组成） | **−10.2%~−10.5%** | 无 | ✅ **采纳**（改构建配置即可） |
| **P3** | `-Ctarget-cpu=native` | 拿到 LSE 原子指令等 | **−0.6%~−1.1%** | 无 | ✅ 采纳（免费小收益） |
| **P4** | **P1+P2+P3 组合** | — | **−29.5%~−30.1%**（37.37→26.08 ms） | 无 | ✅ **最优配置** |
| P5 | 调用无 offsets 的 `encode_batch_fast`（内核 `OffsetType::None`） | 整段偏移计算被跳过 | **aarch64 未实测**（x86 仅作对照 −18.9%；ARM 上该组件更贵，预计同量级或更高） | **offsets 变 (0,0)** | ⚠️ 仅在不需要 offsets 时；ARM 落地前需实测 |
| P6 | 换分配器 jemalloc/mimalloc/tcmalloc | 分配器+memcpy 占 36~47% | 未实测（预估 −5~15%） | 无 | 🔬 待实测 |
| P7 | 减少临时 String/Vec、复用缓冲 | `NormalizedString::slice` 8.0%（93% 分配器）、`drop<Encoding>` 4.4%（98% 分配器） | 未实测（预估 −3~8%） | 无（需谨慎） | ⚠️ 并入 P1/P6 |
| P8 | 恒等 NFC 快路径 | NFC 相关 4.5% | ≤4.5%（未实测） | 仅当文本已 NFC 时等价 | ⚠️ 通用性差 |
| P9 | 多文档并行（batch>1 + rayon / 进程池） | batch=1 时并行池完全不参与 | 吞吐近核数倍 | 无 | ✅ 按场景（延迟场景无收益） |
| P10 | SVE 宽组（NEON8→NEON16/SVE16/SVE32） | 加宽组探测 | **负收益 +2.2%~+5.1%** | 无 | ❌ **已证伪** |
| P11 | 线程/Rayon 调参 | — | ≈0（噪声内） | 无 | ❌ 无效 |
| P12 | 改 batch 形态 / 绕过 transformers 包壳 | — | ±1%（噪声） | 无 | ❌ 不是瓶颈 |
| P13 | `-Cllvm-args=-aarch64-sve-vector-bits-min=256` | 固定 VL 让 LLVM 生成更多 SVE | ≈0（代码形态确实变了） | 无（但要求 VL≥256） | ❌ 不建议 |
| P14 | 按 `cpuinfo` 补齐 feature（i8mm/bf16/dotprod/…） | — | **0**（一条指令都没生成） | 无 | ❌ 本负载无用 |
| P15 | 替换 Oniguruma 正则 | inclusive 10.3% | 可能 −5~10% | **会改 token 边界** | ❌ 属于"换 tokenizer" |
| P16 | 硬件层（频率/绑核/NUMA） | governor=performance、绑核已做 | 均值≈0，方差↓ | 无 | ❌ 只影响方差 |

---

## 二、采纳项详解

### P1：`BytesToCharOffsetConverter` 的 `HashMap<usize,usize>` → `Vec<u32>`（46 行）

- **机制**：`into_encoding` 每次调用都会为**原文字节的每个位置**建立"字节下标 → 字符下标"映射。该映射单调且稠密（0..len-1 每个字节恰好属于一个字符），哈希表只带来常数因子更差的访存、一次扩容重哈希和 ~1.8 MB 临时内存。
- **证据**：组件级微基准（同机）：构造 21.97 → 1.07 ns/byte（**20.5×**），查询 **22.4×**，合计 2.169 → 0.104 ms（20.9×）；内存 114,688 个哈希槽（16 B/槽 ≈ 1.8 MB）→ 78,960 × 4 B = **316 KB**。
- **实测端到端**：aarch64 **−23.65% / −24.14% / −23.68%**（default/rayon1/rayon8，10 block，CI 宽 ±0.75pp，10/10 block 全胜）；x86 **−20.8%**（原案例路径）/ −18.0%（raw `encode_batch`），并用"自编 stock vs PyPI stock = +0.2%"排除了工具链污染。
- **输出**：两机各自 19 用例逐字节相同。
- **风险/成本**：改上游单文件；`Vec<u32>` 的字符下标上限 ~4.29e9（上游可改 `usize` 或加断言）；每次调用仍分配 316 KB（可后续做线程本地复用）。
- **为什么 ARM 收益更大**：被消掉的除了表本身，还有 78,960 次插入及**扩容重分配**——ARM 上分配器更贵。

### P2：`lto="fat"` + `codegen-units=1`（Cargo profile）

- **机制**：这个负载的时间散落在大量小函数上（迭代器 `fold`/`map`/`flatten`、`HashMap` 包装、边界检查），跨 crate 内联与单 CGU 能把它们折叠掉。
- **证据**：`.so` 61.9 → **43.7 MB**，符号 20,253 → **8,857**（说明内联真的发生）；端到端 **−10.50% / −10.23% / −10.32%**。
- **重要**：归档的 SVE 实验里**所有 wheel 都是默认 release（CGU=16、无 LTO）**，这 10% 一直没被拿走——即"换个构建配置"比那整轮 SVE 工作（负收益）有价值得多。
- **怎么用**：写进被构建包的 `[profile.release]`，**不要**放 `RUSTFLAGS`（会与依赖的 `-C embed-bitcode=no` 冲突，实测报错 `options -C embed-bitcode=no and -C lto are incompatible`）。
- **输出**：逐字节相同。

### P3：`-Ctarget-cpu=native`（−1%）

- **机制**：LLVM 不认识部件号 `0xd02`，`native` 只解析出 10 个 feature（`aes crc fp16 lse rand sha2 sha3 sm4 sve neon`）；实测唯一产生实质代码变化的是 **LSE 原子指令 21 → 3160**。
- **收益**：**−0.59% / −1.11% / −0.75%**（7/9 格显著）。
- **输出**：逐字节相同。
- **注意**：这是"发行版 wheel 也能受益"的项；但若目标是跨机器分发，`native` 会牺牲可移植性（应显式列出 feature 或按机型分发）。

### P4：组合 = 最优配置（−30%）

- **配置**：P1 补丁 + `lto="fat"`/`codegen-units=1` + `target-cpu=native`（+ 补齐 feature，见 P14 说明其无收益）。
- **实测**：**−30.11% / −29.53% / −29.68%**，37.37 → **26.08 ms**（default），逐 block 比值 0.688~0.714，CI 宽 ±0.6pp。
- **意义**：包内报告口径下，aarch64 从"+39~40%"变为**比 x86 stock 基线（27.66 ms）更快 5.7%**；但两侧都优化后仍残留约 +25~30% 的**架构性**差距（单核标量 IPC）。

---

## 三、有条件项详解

### P5：应用侧走无 offsets 路径（−18.9%，但丢 offsets）

- **机制**：Python 绑定的 `encode`/`encode_batch` 无条件调用 `encode_char_offsets`（`OffsetType::Char`），而 `encode_fast`/`encode_batch_fast` 用 `OffsetType::None`，**完全跳过偏移计算**。
- **实测（x86，交错配对）**：`encode_batch` 22.063 → `encode_batch_fast` **17.897 ms（−18.9%）**，token IDs 完全相同，`offsets` 全为 `(0,0)`。
- **结论**：调用方只需要 token IDs（分类/嵌入）时立刻可用，**无需改任何库**；需要 offsets 时不可用。

### P6：换分配器（未实测，目标最大）

- **依据**：perf 显示分配器 + memcpy 占 **40.7%（.51）/ 46.8%（.50）**——单项最大。
- **做法**：`LD_PRELOAD` jemalloc/mimalloc（Rust 默认用系统分配器，故会被替换），或在上游指定 `#[global_allocator]`。
- **注意**：P1 已经顺带减少了一部分分配压力，两方案收益有重叠；.50 上没有现成库、需要先准备。

### P7：减少临时分配（并入 P1/P6）

- **依据**：`NormalizedString::slice` 8.0% 中 93% 是分配器；`drop_in_place<Encoding>` 4.4% 中 98% 是分配器。
- **结论**：改动面大、收益分散，建议作为 P1 的后续（例如按 token 复用缓冲）而不是独立立项。

### P8：恒等 NFC 快路径（≤4.5%，通用性差）

- **依据**：NFC/`unicode_normalization_alignments` 合计 4.5%（inclusive）。
- **风险**：只有在文本**已经是 NFC** 时跳过才等价；对含组合字符的输入会改变 token/offsets。建议上游加"先判定是否已规范化"的快路径，而不是删掉 normalizer。

### P9：多文档并行（场景相关）

- **依据**：batch=1 时 rayon 池虽然被创建，但 `par_iter` 只有 1 个元素，工作全在调用线程完成（进程 CPU/wall ≈ 0.998）。batch>1 时 rayon 按 item 并行。
- **结论**：单文档延迟场景无收益；多文档吞吐可近核数倍——这才是多核 ARM 的正确用法。

---

## 四、不采纳项与理由

| # | 为什么不采纳 | 关键证据 |
|---|---|---|
| P10 SVE 宽组 | 两机端到端都变慢 | .51 +2.4~5.1%、.50 +2.2~3.3%（9/9 格显著）；微测确实在高占用率 miss 上快 −17.8~−64.8%，但该负载的 hashbrown 时间主要在**建表**而非 miss 探测 |
| P11 线程调参 | 无影响 | `RAYON=1/8/default` 差异在噪声内；`queue_ms ≤ 0.06 ms` |
| P12 API 形态 | 不是瓶颈 | `transformers.__call__` 22.063 / raw `encode` 22.102 / raw `encode_batch` 21.888 ms（±1%） |
| P13 固定 VL llvm-args | 无收益 + 可移植性风险 | 代码形态确实变了（SVE 2207→3746、`ptrue` 161→412），但端到端 −0.39~−1.25%，与不固定时无统计差异；且要求运行期 VL≥256 |
| P14 补齐 feature | 本负载无对应模式 | 指令普查：i8mm/bf16/dotprod/crc/aes 相关指令**一条都没有**；收益与只开 native 无差异 |
| P15 换 Oniguruma | 会改变分词结果 | GPT-2 式正则定义了 token 边界；inclusive 10.3% 属于"换 tokenizer"的成本，不是可优化浪费 |
| P16 硬件/绑核 | 只影响方差 | `governor=performance` 已开；绑核只降低抖动 |

---

## 五、方法学层面的发现（比单条优化更值得记住）

1. **这个负载是纯单线程执行的**：batch=1 下即使创建了 320 线程的 Rayon 池也不参与计算（进程 CPU/wall ≈ 0.998，若在自旋等 worker 会接近 2.0）。因此任何"并行度"类优化都无效。
2. **PyPI 的 wheel 一条 SVE 指令都没有**（以 `-Ctarget-cpu=generic` 编译）；而按 `-Ctarget-feature=+sve` 重编后 SVE 指令 2207 条，**性能毫无变化**——ISA 特性不是本负载的瓶颈。
3. **同一份代码，`lto="fat"`+`codegen-units=1` 值 10%**：构建配置和算法改动是同一量级的杠杆，归档实验完全忽略了它。
4. **Python 绑定强制计算 char offsets**：即使调用方不需要 offsets（`return_offsets_mapping=False`），Rust 侧仍然建表——这是上游 API 层的可优化点（暴露 byte/no-offset 路径）。
5. **哈希表用在单调稠密映射上是纯开销**：这类"坐标换算"应该用数组/前缀和；同样的模式在别的库/语言里也常见。
6. **ARM 上的差距集中在标量代码**：分配器/哈希/正则/Unicode 都是指针追逐 + 分支密集，加宽向量帮不上；能改的是"少做这些事"。

---

## 六、下一步建议（按性价比）

1. **P2（LTO/CGU=1）**：零风险、改构建配置、−10%。应立刻进 CI/发布流程。
2. **P1（补丁）**：−24%，写上游 PR（补丁 + 等价性测试 + 微基准）。
3. **P6（分配器）**：唯一还没实测的大项（36~47%），建议在能装 jemalloc/mimalloc 的环境上做一次配对实验。
4. **P2 的分解**：`codegen-units=1` 与 `lto=fat` 各自贡献多少（两次构建 + 两次配对即可）。
5. **P1 的延伸**：把每次调用的 316 KB 分配改成线程本地复用；以及给上游提"暴露无 offsets 编码 API"（把 P5 从 workaround 变成正式能力）。
6. **发布策略**：与其做架构特化的 SVE 宽组（P10，负收益），不如发布"LTO + 合理 feature"的 wheel；若坚持多版本分发，按 CPU 能力分档比按 VL 分档更有意义。
7. **P6（换分配器）应在目标机 aarch64 上验证**：.50 上没有现成 jemalloc/mimalloc，x86 数据对结论无参考价值。
