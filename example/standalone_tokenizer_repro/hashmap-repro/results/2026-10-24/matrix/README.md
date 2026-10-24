# reserve × hasher 矩阵与 hash-only 隔离（2026-10-24）

## 目的

1. 把「扩容/迁移」从「插入」里拆出来：`--reserve` 预分配后，build 只剩插入 + 探测；
2. 把「SipHash 算术」从「HashMap 探测/访存」里拆出来：`hash_only.rs` 只调 `RandomState::hash_one`，不建表。

n=78,960，warmup=3，15 reps 中位数；ARM 固定 2.9 GHz，x86 按 4.45 GHz 标定折算（估计值）。

## 矩阵

| hasher | reserve | ARM build (ms / cyc) | x86 build (ms / cyc) | ARM query (ms / cyc) | x86 query (ms / cyc) | query 周期比 |
|---|---|---:|---:|---:|---:|---:|
| std | no | 3.601 / 10.44M | 2.425 / 10.79M | 1.728 / 5.01M | 0.844 / 3.76M | 1.33 |
| std | yes | 1.378 / 3.99M | 0.771 / 3.43M | 1.943 / 5.64M | 0.801 / 3.57M | 1.58 |
| mix | no | 2.815 / 8.16M | 2.004 / 8.92M | 0.803 / 2.33M | 0.280 / 1.25M | 1.87 |
| mix | yes | 0.636 / 1.84M | 0.386 / 1.72M | 0.786 / 2.28M | 0.264 / 1.17M | 1.94 |
| identity | no | 2.196 / 6.37M | 1.443 / 6.42M | 0.220 / 0.64M | 0.117 / 0.52M | 1.22 |
| identity | yes | 0.649 / 1.88M | 0.374 / 1.66M | 0.220 / 0.64M | 0.116 / 0.52M | 1.23 |

## 派生量（每操作周期）

| 量 | ARM | x86 |
|---|---:|---:|
| hash_only：每次 `hash_one(usize)` | 5.94 ns = **17.2 cyc** | 4.15 ns = **18.5 cyc**（估） |
| 插入（std, reserve）：每次 | 50.6 cyc | 43.5 cyc |
| 查询（std, reserve）：每次 | 71.4 cyc | 45.1 cyc |
| 查询（mix, reserve）：每次 | 28.9 cyc | 14.9 cyc |
| 查询（identity, reserve）：每次 | 8.1 cyc | 6.6 cyc |
| 扩容+迁移（std：noreserve−reserve） | 6.45M cyc | 7.36M cyc（估） |

## 能说什么

1. **SipHash 算术本身不是差距来源**：hash-only 每次 17.2 vs 18.5 cyc，ARM 不慢（甚至略低）。
2. **插入阶段只多约 7 cyc/次**（50.6 vs 43.5），与 hash 成本同量级，差距有限。
3. **查询阶段是主要差距**：std 71.4 vs 45.1 cyc（+26），mix 28.9 vs 14.9（约 1.9×），identity 8.1 vs 6.6（约 1.2×）。
   - 扣掉 hash-only 的 17 cyc，std 探测/访存约 **54 vs 27 cyc**，接近 2×；
   - 顺序布局（identity）几乎持平，随机布局（mix/std）差距大 → 指向**随机桶访问的延迟/重叠**，而不是哈希计算。
4. **扩容/迁移没有显示 ARM 更贵**（6.45M vs 7.36M cyc，估计口径）。旧报告 H2 预测“`--reserve` 使 ARM build 降幅大于 x86”，在这条最小路径上未复现；注意 x86 频率为估计值，且 reserve 也改变了表的布局。

## 不能说什么

- x86 的 cycles 是「墙钟 × 4.45 GHz」估算，误差约 ±3%，没有 PMU 交叉验证；
- 单进程、单轮 reps，未做多进程交错与 p95；
- `identity`/`mix` 改变桶布局，不能把差异全部归给“哈希”或“访存”之一；
- 这是最小用例，不代表真实 tokenizer（约 51,200 次 get、建表后穿插 BPE/正则）。

## 数据

- `arm/`、`x86/`：六组矩阵 JSON；
- `../arm/hash_only.json`、`../x86/hash_only.json`：hash-only 原始样本；
- 命令见 [`../commands.md`](../commands.md)。

## token-like 查询模式（2026-10-24 追加）

真实 `convert()` 每个 token 取 start/end 两次，token 平均间隔约 3.1 字节。用 `--query-file` 生成 52,640 个偏移（每 3 字节取 start 与 start+2），5 个独立进程 × 15 reps：

| hasher | ARM (cyc/q) | x86 (cyc/q, 估) | 比值 |
|---|---:|---:|---:|
| std | 71.2 | 48.0 | 1.48 |
| mix | 30.6 | 16.3 | 1.88 |
| identity | 8.8 | 6.6 | 1.35 |

与 stride=1 的结论一致：查询比约 1.5×（std）/ 1.9×（mix），顺序布局的 identity 仅 1.35×。说明差距不随查询次数或“顺序 vs 均匀步进”改变，主要跟随桶布局的随机性。

## 方法备注：per-query 指令数的口径（2026-10-24 追加）

- 用 `--query-file X --query-repeat 10` 让同一份 52,640 个偏移重放 10 次，避免构造 4 MB 的偏移向量挤占缓存；perf 的空查询/满查询两次运行相减得到每次 get 的周期与指令数。
- ARM 实测：526,400 次 get ≈ **63 cyc/get、96–110 instr/get、IPC ≈ 1.6**（空查询基线约 4.99M cycles/轮）。
- x86 没有 PMU；Valgrind Ir 不能直接换算成每次 get 的 guest 指令数——解析 52,640 个十进制偏移的指令开销与查询本身同量级，且逐位宽不同会变化，无法用「空文件」干净扣除。因此 **x86 的 instructions/IPC 仍标记为缺失**，只能给「墙钟 × 标定频率」的估算周期。
- 之前把 Valgrind Ir 差值直接除以查询数得到的 342 Ir/get 含解析开销，已作废，不再引用。
