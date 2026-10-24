# ARM vs x86：HashMap 查询路径为什么慢（结论报告 v1）

> 日期：2026-10-24｜约束：**不把 HashMap 改成 Vec**｜机器：ARM `192.168.41.50`（2.9 GHz 固定）vs WSL x86（i7-12700，~4.45 GHz 标定）
> 状态：**结论报告 v1**。x86 无硬件 PMU，IPC 只有 ARM 侧实测；其余结论均有原始数据支撑。

## 结论摘要

1. **差距在查询探测路径**，不在 SipHash 算术（hash-only 17.2 vs 18.5 cyc）、不在插入（50.6 vs 43.5）、也不在扩容迁移（6.45M vs 7.36M cyc）。
2. **表能驻留 L2 时差距很小**（1.11–1.17×）；**表超过约 1.3 MiB L2 后升到 1.39–1.46×**（n=65k/131k）。
3. **机制**：每查询 L2 refill 从 0.65 增到 1.36；TLB walk ≤0.05/query（排除）；THP 未覆盖该表；ARM 的并发 miss 扩展略弱（K=16 时 9.5× vs 11.2×），与 L2 miss 成本叠加。
4. **IPC**：同一 get 路径 ARM 92 instr / 64.7 cyc（IPC 1.42），x86 127 instr / 48.9 cyc（IPC ≈2.6）；ARM 指令更少但 IPC 更低。
5. **真实负载占比**：ARM 上 offset 表相关约 **8%–12%**（`encode` 36.29 ms vs `encode_batch_fast` 31.97 ms）；它解释查询路径为何慢，但不单独解释 38% 的端到端差距。

## 1. 问题与复现

真实热点：`tokenizers` 的 `BytesToCharOffsetConverter` 对 UTF-8 **每个字节**建一张 `HashMap<usize,usize>`（本输入 78,960 条），再对**每个 token** 调 `convert()`；`convert` 做 2 次 `get`（start/end），25,600 tokens ≈ **51,200 次 get**。`encode()` 硬编码 `OffsetType::Char`（`tokenizers/src/tokenizer/mod.rs:881`），所以即使不请求 offsets 也会建表。

最小复现 [`hashmap-repro/main.rs`](../../main.rs)：稠密插入 0..n，再按偏移查询；三个 hasher：

- `std`：`RandomState`（SipHash-1-3），真实配置；
- `mix`：splitmix 风格 usize 哈希，近随机桶布局，用来扣除 SipHash 算术；
- `identity`：hash=key，顺序布局退化，只作局部性上界。

另有 `hash_only.rs`（只算哈希）与 `chase.rs`（依赖加载延迟曲线）。

## 2. 频率与 IPC 口径

- ARM 固定 2.9 GHz（performance、boost=0），`perf` 可读 cycles/instructions。
- x86 无 PMU（`dmesg: unsupported p6 CPU model 151 no PMU driver`），**cycles = 墙钟 × 标定频率**（mul 链 + L1 追逐，逐 CPU 4.35–4.58 GHz，取 4.45）；**instructions/IPC 缺失**。
- 曾用 Valgrind Ir 估算 x86 指令数，但 52,640 个十进制偏移的解析开销与查询同量级，无法干净扣除，**作废**。

## 3. 证据

### 3.1 SipHash 算术：两机持平

| hash-only（每次 `hash_one(usize)`） | ARM | x86（估） |
|---|---:|---:|
| 时间 | 5.94 ns | 4.15 ns |
| 周期 | **17.2** | **18.5** |

### 3.2 插入/扩容：接近，扩容不更贵

| n=78,960 | ARM | x86（估） |
|---|---:|---:|
| 插入（reserve） | 50.6 cyc/次 | 43.5 |
| 扩容+迁移（noreserve−reserve） | 6.45M cyc | 7.36M |

旧报告“每次迁移 ARM 更贵（LDP 形态）”在本最小用例**未复现**。

### 3.3 查询：主要差距

| 配置（reserve） | ARM cyc/q | x86 cyc/q（估） | 比值 |
|---|---:|---:|---:|
| std | 68.0 | 48.9 | 1.39 |
| mix | 26.1 | 15.5 | 1.68 |
| identity | 8.7 | 6.5 | 1.33 |

- 52,640 次 token-like get：66.8 vs 45.9 cyc/q（1.46）；
- 3 个独立进程、每进程 15 reps，p50 波动 1%–4%。

### 3.4 表大小与负载因子

n 扫描（std, reserve）：

| n | 表约 | ARM cyc/q | x86 cyc/q | 比值 |
|---:|---:|---:|---:|---:|
| 4,096 | 136 KiB | 36.0 | 32.5 | **1.11** |
| 16,384 | 544 KiB | 38.9 | 34.1 | **1.14** |
| 32,768 | 1.1 MiB | 45.4 | 40.8 | **1.11** |
| 49,152 | 1.1 MiB（负载更高） | 50.2 | 42.9 | 1.17 |
| 65,536 | 2.2 MiB | 66.1 | 47.6 | **1.39** |
| 131,072 | 4.4 MiB | 75.0 | 51.5 | **1.46** |

拐点在 **1.1 MiB（1.17×）到 2.2 MiB（1.39×）之间**，与 ARM 每核 L2 约 1.3 MiB 吻合：表能驻留 L2 时差距 ~1.1×，超出后跳到 1.4–1.5×。

负载因子扫描（n=32,768）：0.70→0.95 每查询周期几乎不变，0.30/0.50（表更大）跳到 1.19–1.24。
→ **探测长度不是主因；表 footprint 越大，差距越大。** 96 KiB（L1/L2 驻留）时只有 1.11×，说明核心探测代码本身差距很小；1.4–1.5× 主要出现在表超出 L2 之后。

> 版本一致性：早期 n 扫描用的是 stride 循环版本（n=4096 时 ARM 41.8 cyc/q）；改成 `Vec` 偏移迭代后同一条件下为 36.0 cyc/q。v2 数据集统一用新二进制重测，旧数字不再引用（见 [`v2/README.md`](v2/README.md)）。

### 3.5 依赖加载延迟：两机相当（修正过一次）

| 工作集 | ARM cyc/load | x86 cyc/load |
|---|---:|---:|
| 16 B | 5.0 | 4.9 |
| 32 KiB | 5.4 | 4.9 |
| 512 KiB | 14.6 | 18.5 |
| 2 MiB | 43.6 | 44.7 |
| 8 MiB | 82.0 | 121.2 |

早先版本用带边界检查的 `table[black_box(p)]`，在 ARM 上读出 ~11 cycles，**已撤回**（见 [`chase/README.md`](chase/README.md)）。裸指针链显示两机 L1/L2 延迟相当，**“ARM load 天生慢”不成立**。

### 3.6 ARM 侧 IPC 与事件

用 `--query-repeat 10 / 20` 两次运行相减（同一偏移表，解析与建表开销抵消）：

| 每 get | ARM（perf） | x86（Ir 差值 / 估算周期） |
|---|---:|---:|
| cycles | **64.7**（实测） | **48.9**（墙钟 × 4.45 GHz） |
| instructions | **92.1**（实测） | **127.1**（Valgrind Ir 差值，guest 指令） |
| IPC | **1.42** | **≈2.6** |

- ARM 每条 get 的**指令更少**（92 vs 127），但**周期更多**（64.7 vs 48.9），所以 **IPC 低约 1.8×**：差距是执行效率/停顿，不是“干了更多活”。
- x86 IPC 依赖频率标定（±3%）：f=4.2/4.6 GHz 时 IPC≈2.5–2.8。
- 每查询 L2D_CACHE_REFILL：std ≈ 2.0、identity ≈ 0.83；L1D_TLB_REFILL：std ≈ 3.15、identity ≈ 0.37；后端 stall 占周期约 60%。

### 3.7 ARM 事件归因：L2 refill 随表增大，TLB walk 可忽略

perf（13 轮 = 3 warmup + 10 reps；std，reserve）：

| n | L2D refill / (插入+查询) | L3D refill / op | DTLB walk / op |
|---:|---:|---:|---:|
| 4,096 | 0.33 | 0.065 | 0.025 |
| 78,960 | 0.68 | 0.047 | 0.006 |

- 计数区域是 13 轮（3 warmup + 10 reps），包含插入与查询；按“每操作”归一（除以 13×(n 插入 + n 查询)）。
- **TLB walk 可忽略**（每查询约 0.01–0.05）；L3 refill 也低（≤0.07/op）；
- 每操作 L2 refill 从 0.33 增到 0.68，与“表超过 L2 后差距从 1.11× 升到 1.46×”同步；
- THP=always，但本进程哈希表映射的 `AnonHugePages: 0`，**表并未被大页覆盖**，不存在 ARM 大页优势。

### 3.8 并发 miss（MLP）：ARM 扩展略弱

2 MiB 随机环、K 条独立链（[`mlp/README.md`](mlp/README.md)）：

| K | ARM cyc/load | x86 cyc/load | 加速比 ARM / x86 |
|---:|---:|---:|---:|
| 1 | 53.9 | 39.0 | 1.00 / 1.00 |
| 4 | 14.7 | 12.2 | 3.66 / 3.19 |
| 16 | 5.7 | 3.5 | 9.48 / 11.21 |

单链延迟 cycles 只差 ~1.2–1.4×，但高并发时 ARM 的扩展差 ~15%，与查询在 L2 之外 1.4–1.5× 的方向一致；两个因素叠加，暂不能精确拆账。

### 3.9 真实 tokenizer 的占比（ARM 实测）

同一 fixture，stock wheel（tokenizers 0.21.4 / transformers 4.48.3），ARM 2.9 GHz：

| 调用 | p50 | tokens | 说明 |
|---|---:|---:|---|
| `encode(text)` | **36.29 ms** | 25,600 | 走 `OffsetType::Char`，建表 + 逐 token 转换 |
| `encode_batch_fast([text])` | **31.97 ms** | 25,600（ids 相同） | 跳过 offset 转换路径 |

差值 **4.32 ms ≈ encode 的 11.9%**，是真实负载里 offset 映射相关开销的上界。最小复现的模型估算为：插入 78,960 × 50.6 cyc + 查询 51,200 × 68 cyc ≈ 7.5M cycles ≈ 2.6 ms，与同一量级（真实负载还叠加缓存污染与 offset 传播）。

含义：HashMap offset 表在 ARM 上约占 encode 的 **8%–12%**；这解释了最小用例能复现“查询路径更慢”，但**不能单独解释 38% 的端到端差距**——其余来自分配器、NFC/正则/BPE 等阶段（旧报告采样也指向同一方向）。

原始输出：[`real/arm.json`](real/arm.json)；脚本 [`real/bench.py`](real/bench.py)。

## 4. 能说的与不能说的

**能说**

1. 差距在**查询探测路径**：SipHash 算术、插入、扩容迁移都不是主因。
2. 差距随**桶布局随机性**（identity 1.35× < std 1.48× < mix 1.88×）和**表 footprint**增大。
3. ARM 的依赖加载延迟、内存层级与 x86 相当或更优，不能拿“load 慢”解释。
4. 96 KiB 表（完全驻留 L2）时只慢 1.11×，核心探测代码差距很小；差距主要出现在表超过 L2 之后（1.4–1.5×），指向 L2/L3/TLB 与多流随机访问，而不是“ARM 的 ALU/load 更慢”。

**不能说**

- 不能给出跨架构 IPC 对比：x86 无 PMU，instructions/IPC 缺失；
- 不能把差距全归给 TLB：TLB refill 计数高，但 chase 曲线不支持“内存延迟更差”；
- 不能外推到真实 tokenizer 全流程：本复现只覆盖 offset 映射这一张表；ARM 实测该表相关开销约占 encode 的 8%–12%（`encode` 36.29 ms vs `encode_batch_fast` 31.97 ms），NFC/正则/BPE/分配器不在内；
- 不能声称已在生产配置下验证（单机、热表、单线程）。

## 5. 尚待解释

- 超出 L2 后的 1.4–1.5×：已定位到「L2 refill 增多 + 并发 miss 扩展略弱」，但两者各占多少未拆开；TLB walk 已排除。
- 是否需要更大工作集/不同 L2 容量机器复核“L2 边界”拐点。
- x86 侧 IPC：需要可读 PMU 的环境，否则报告中固定标注缺失。

## 6. 复现

```bash
# 构建（两侧均 rustc 1.98.0）
rustc -O main.rs -o hr && rustc -O hash_only.rs -o ho && rustc -O chase.rs -o chase

# 基准（ARM 绑 CPU240，x86 绑 CPU2）
./hr --n 78960 --reps 20 --warmup 3 --hasher std --reserve
./hr --n 78960 --reps 20 --warmup 3 --hasher std --reserve --query-file queries_tokenlike.txt
./ho --n 78960 --reps 15
./chase --sizes 2,4096,65536,262144,1048576 --iters 3000000
```

原始数据：[`arm/`](arm/)、[`x86/`](x86/)、[`matrix/`](matrix/)、[`nscan/`](nscan/)、[`load/`](load/)、[`chase/`](chase/)。
