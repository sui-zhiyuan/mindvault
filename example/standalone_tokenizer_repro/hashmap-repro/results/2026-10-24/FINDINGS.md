# ARM vs x86：HashMap 查询路径为什么慢（中间报告）

> 日期：2026-10-24｜约束：**不把 HashMap 改成 Vec**｜机器：ARM `192.168.41.50`（2.9 GHz 固定）vs WSL x86（i7-12700，~4.45 GHz 标定）
> 状态：**中间报告，未定论**。x86 无硬件 PMU，IPC 只有 ARM 侧实测；其余结论均有原始数据支撑。

## 1. 问题与复现

真实热点：`tokenizers` 的 `BytesToCharOffsetConverter` 对 UTF-8 **每个字节**建一张 `HashMap<usize,usize>`（本输入 78,960 条），再对**每个 token** 调 `convert()`；`convert` 做 2 次 `get`（start/end），25,600 tokens ≈ **51,200 次 get**。`encode()` 硬编码 `OffsetType::Char`（`tokenizers/src/tokenizer/mod.rs:881`），所以即使不请求 offsets 也会建表。

最小复现 [`hashmap-repro/main.rs`](../main.rs)：稠密插入 0..n，再按偏移查询；三个 hasher：

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
| 4,096 | 96 KiB | 36.0 | 32.5 | **1.11** |
| 65,536 | 1.5 MiB | 66.1 | 47.6 | **1.39** |
| 131,072 | 3 MiB | 75.0 | 51.5 | **1.46** |

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

- hashbrown 查询路径（526,400 次 get，perf）：约 **63 cycles、96–110 instructions、IPC ≈ 1.6**；
- 每查询 L2D_CACHE_REFILL：std ≈ 2.0、identity ≈ 0.83；L1D_TLB_REFILL：std ≈ 3.15、identity ≈ 0.37；
- 后端 stall 占周期约 60%。

## 4. 能说的与不能说的

**能说**

1. 差距在**查询探测路径**：SipHash 算术、插入、扩容迁移都不是主因。
2. 差距随**桶布局随机性**（identity 1.35× < std 1.48× < mix 1.88×）和**表 footprint**增大。
3. ARM 的依赖加载延迟、内存层级与 x86 相当或更优，不能拿“load 慢”解释。
4. 96 KiB 表（完全驻留 L2）时只慢 1.11×，核心探测代码差距很小；差距主要出现在表超过 L2 之后（1.4–1.5×），指向 L2/L3/TLB 与多流随机访问，而不是“ARM 的 ALU/load 更慢”。

**不能说**

- 不能给出跨架构 IPC 对比：x86 无 PMU，instructions/IPC 缺失；
- 不能把差距全归给 TLB：TLB refill 计数高，但 chase 曲线不支持“内存延迟更差”；
- 不能外推到真实 tokenizer 全流程：本复现只覆盖 offset 映射这一张表（旧报告称 offset/hashbrown 路径并集约占 ARM 采样周期 25%），NFC/正则/BPE/分配器不在内；
- 不能声称已在生产配置下验证（单机、热表、单线程）。

## 5. 尚待解释

- 缓存驻留时的固定 ~1.3×：候选是 hash→地址→ctrl→key 的依赖链在 ARM 上重叠更差（IPC 低）、或 hashbrown NEON8 与 x86 SSE2 的探测代码差异、或分支/停顿。需要指令级归因（perf annotate / SPE，若可用）。
- 大表时 TLB 与 footprint 的独立贡献：需要控制页大小（THP/大页）对比。
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
