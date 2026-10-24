# 成本结构：钱花在哪里（两台机器、两条独立剖析路径）

## 一、编码阶段 self 时间占比

| 成本项 | `192.168.41.50`（本次，5451 个编码样本） | `192.168.41.51`（归档，stock perf） | 可优化性 |
|---|---:|---:|---|
| **分配器 + memcpy**（`_int_free`/`malloc`/`_int_malloc`/`realloc`/`malloc_consolidate`/`unlink_chunk`/`cfree`/`__memcpy_generic`） | **46.8%** | **40.7%** | 部分可（P1/P6/P7） |
| **hashbrown 簇**（`insert` + `reserve_rehash` + `hash_one`） | **14.2%** | **16.0%** | **可（P1）** |
| `BytesToCharOffsetConverter::convert` | 5.3% | 5.6% | **可（P1）** |
| Oniguruma 正则 `match_at` | 4.5%（inclusive 10.3%） | 5.8% | 不可（改了就换 tokenizer） |
| Unicode NFC / `unicode_normalization_alignments` | 4.5% | ~2.4% | 有限（P8） |
| `NormalizedString::slice`（93% 是分配器） | 8.0% | — | P7 |
| `drop_in_place<Encoding>`（98% 是分配器） | 4.4% | — | P7 |
| `utils::cache::Cache::get` | 2.3% | 2.4% | 低 |
| `…Model>::tokenize`（BPE 调用，inclusive BPE 9.8%） | 2.2% | 2.6% | 空间小 |

`.50` 的 self 热点 top（同名合并）：`_int_free` 12.55%、`__GI___libc_malloc` 12.14%、`BytesToCharOffsetConverter::convert` 5.27%、`__memcpy_generic` 5.03%、`_int_malloc` 4.97%、`hashbrown::HashMap::insert` 4.88%、`hashbrown::RawTable::reserve_rehash` 4.82%、`match_at` 4.51%、`core::hash::BuildHasher::hash_one` 4.49%、`malloc_consolidate` 4.35%。

## 二、inclusive 结构（.50）

| inclusive | 函数 | 备注 |
|---:|---|---|
| 79.0% | `TokenizerImpl::encode_single_sequence` | 编码主体 |
| **35.0%** | `PreTokenizedString::into_encoding` | 其中 **HashMap 构建 15.1%**、字节↔字符换算 6.2% ← **P1 的靶心** |
| 16.0% | `PreTokenizedString::split`（预分词） | 其中 Oniguruma 10.3%、NFC 4.5% |
| 12.1% | `PreTokenizedString::tokenize` | 其中 BPE 9.8% |
| 3.1% | `utils::cache::Cache::get` | |

## 三、归因推理：为什么"加宽向量"必然失败

1. 时间的主要成分是**分配、哈希、指针追逐、分支**（分配器 47% + 哈希 15% + 偏移换算 6%），不是规则的数值循环；
2. 因此它对**单核 IPC** 敏感，而对**向量宽度**不敏感——这正是"服务器 ARM 慢 39%"的形态；
3. 归档的 SVE 宽组实验把力气花在"高占用率 miss 的探测次数"上（微测确实快 18~65%），但这个负载的 hashbrown 时间花在**建表（insert/rehash）**，宽组在这里只会增加每次插入的代价 → 两机端到端都变慢（P10 证伪）。
4. 反过来说：**能改的是"少做这些事"**——P1 让 78,960 次哈希插入与 ~1.8 MB 临时内存消失，立刻拿到 −24%。

## 四、执行模型（决定了哪些优化无效）

- 任意时刻只有 1 个 encode 在跑：`ThreadPoolExecutor(8)` 串行提交、实际只创建 1 个 worker；batch=1 使 Rust 侧 `par_iter` 只有 1 个元素。
- **进程 CPU / wall ≈ 0.998** → 工作全部由调用线程完成（若调用线程在自旋等 worker 会接近 2.0）。
- `executor_thread_cpu_ms ≈ tokenizer_ms`、`queue_ms ≤ 0.06 ms` → 排队可忽略。
- Rayon 池虽然被**惰性创建**（默认 = CPU 数，.50 上是 320 线程；`RAYON_NUM_THREADS` 可改），但**不参与计算**。
- 结论：P11（线程调参）、P9（单文档并行）在这条路径上无效/无关。

## 五、原始材料

- `.50` 的 perf 原始：`measurements/perf/`（火焰图 SVG + 折叠栈 + 归因报告）
- `.51` 的 perf 报告（归档，仅作对照）：`measurements/perf/archive-51-report-demangled.txt`
