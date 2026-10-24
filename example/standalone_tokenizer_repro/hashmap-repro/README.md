# hashmap-repro — 最小 HashMap 复现

对应案例：[长文本 tokenizer 的 ARM 性能差距](../README.md)。
目标：在不把 `HashMap` 改成 `Vec` 的前提下，复现并定位 ARM 相对 x86 的查询路径差距。

## 文件

| 文件 | 作用 |
|---|---|
| `main.rs` | 主基准：稠密插入 `0..n`，再按偏移查询；`std` / `mix` / `identity` 三种 hasher；`--reserve`、`--stride`、`--capacity`、`--query-file`、`--query-repeat` |
| `hash_only.rs` | 只调 `RandomState::hash_one`，不含表，用来隔离 SipHash 算术 |
| `chase.rs` | 单链指针追逐，测各工作集的依赖加载延迟 |
| `chase_mlp.rs` | K 条独立链，测并发 miss 扩展（MLP） |
| `freq_chain.c` | 无 PMU 时的频率标定（mul 链 + L1 追逐；add 链在 WSL 上异常，已弃用） |
| `results/` | 原始 JSON、perf 输出、各轮 README 与结论 |

## 构建与运行

```bash
rustc -O main.rs -o hr
rustc -O hash_only.rs -o ho
rustc -O chase.rs -o chase
rustc -O chase_mlp.rs -o cmlp

# 主基准（ARM 绑 CPU240，x86 绑 CPU2）
./hr --n 78960 --reps 20 --warmup 3 --hasher std --reserve
./hr --n 78960 --reps 20 --warmup 3 --hasher std --reserve --query-file results/2026-10-24/queries_tokenlike.txt

# hash 单独成本
./ho --n 78960 --reps 15

# 依赖加载延迟 / MLP
./chase --sizes 2,4096,65536,262144,1048576 --iters 3000000
./cmlp --n 262144 --chains 1,2,4,8,16 --iters 500000
```

两侧统一 rustc 1.98.0（ARM 用 `rustup run 1.98.0 rustc`）。

## 结论（2026-10-24）

- **SipHash 算术持平**：hash-only 17.2（ARM）vs 18.5（x86 估）cyc。
- **插入 / 扩容不是主因**：插入 50.6 vs 43.5 cyc；扩容+迁移 6.45M vs 7.36M cyc。
- **查询是差距所在**：std 68.0 vs 48.9 cyc/q（1.39×）、mix 1.68×、identity 1.33×。
- **拐点在 L2**：表 ≤1.1 MiB 时 1.11–1.17×；≥2.2 MiB 时 1.39–1.46×。
- **机制**：L2 refill/query 从 0.65 涨到 1.36；TLB walk ≤0.05/query（排除）；THP 未覆盖该表；ARM 并发 miss 扩展略弱（K=16 时 9.5× vs 11.2×）。
- **真实 tokenizer（ARM）**：`encode` 36.29 ms vs `encode_batch_fast` 31.97 ms → offset 表相关约 **11.9%**，与最小用例同量级。

完整报告与证据：[`results/2026-10-24/FINDINGS.md`](results/2026-10-24/FINDINGS.md)。

## 局限

- x86 是 WSL2，无硬件 PMU：cycles 为「墙钟 × 标定频率」估算，**instructions/IPC 缺失**；
- 单机、热表、单线程；未做真实 tokenizer 的指令级插桩；
- L2 miss 成本与 MLP 各占多少尚未精确拆账。
