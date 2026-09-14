# volc_operator_sim Pipeline B：MinHash-LSH 模糊去重

> 更新：2026-09-14

## 简介

本文分析 `volc_operator_sim` 仓库中 **候选 Pipeline B** 的完整执行链路。该 pipeline 在
[PR #2 / commit 02b65df](https://gitcode.com/XuanYuL5/volc_operator_sim/commit/02b65dfd9bdb0db26359fc903893cc84b30a4d59?ref=new_pipeline_script&prId=2)
引入，业务语义是**大规模文本语料的近似重复（near-duplicate）文档删除**，工程意图是**用 Daft 原生
表达式/关系算子实现 MinHash-LSH，物理计划中 Python UDF 节点为 0**，从而把计算压到 Rust 引擎里，
并把性能对比从"Python UDF 链"转向"Daft Rust 关系算子链"。

前提知识：MinHash 用一组哈希函数把文档的 n-gram 集合压成固定长度签名，两文档签名的一致比例近似
其 Jaccard 相似度；LSH banding 再把签名切成若干 band，只在"至少共享一个 band"的文档之间比较，
把 O(n²) 全比较降为同桶比较。相关：`docs/reports/` 下的 Pipeline A 报告（精确去重基线）。

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| LSH | Locality-Sensitive Hashing | 局部敏感哈希；相似项以高概率落入同一桶 |
| band / rows_per_band | — | 签名切分的块数 / 每块包含的哈希个数，`num_hashes = bands × rows_per_band` |
| signature | MinHash signature | 文档的固定长度近似指纹（本 pipeline 为 64 个 uint64） |
| n-gram / shingle | — | 连续 n 个 token（本 pipeline `ngram_size=5`） |
| Jaccard | Jaccard similarity | 两集合交集/并集之比，MinHash 的估计目标 |
| UDF | User-Defined Function | 以 Python 逐行执行的算子；本项目视其为性能与跨架构迁移的主要障碍 |
| barrier | — | 需要全局数据交换（shuffle）的算子边界，如 groupby / join |
| golden | golden acceptance | 功能验收合同（`configs/golden_acceptance.json`），判产物对错 |
| α / N* | contention / optimal concurrency | USL 模型参数：串行份额 / 理论最优并发数 |

## 核心内容

### 1. 代码位置与调度入口

commit `02b65df`（作者 SparrowLii，2026-08-26）新增 3 个文件，共 205 行；在当前 HEAD
`daab395` 上这三个文件**未被修改**。

| 文件 | 行数 | 作用 |
|---|---:|---|
| `runner/pipeline_b.py` | 149 | pipeline 本体：读入 → 规范化 → MinHash → LSH → 分组 → 代表选择 → 写出 |
| `tasks/pipeline_text_minhash_lsh_dedup.json` | 11 | 任务声明：输入路径、参数、输出路径、引擎覆盖、expected 合同 |
| `scripts/data/build_pipeline_b_fixture.py` | 45 | 测试输入生成器：真实文本 + 人造近似重复 |

调度路径：`runner/run_perf_suite.py:91` 依据 `task.workload_kind == "relational_minhash_lsh_dedup"`
分发到 `run_pipeline_b(task, resolved_input, cluster_profile)`。pipeline A/C 用同一套分发
（`relational_exact_dedup` / `relational_model_label_analysis`），三者的共同设计约束是
**Python 只构造 lazy plan，不做逐行业务计算**（Pipeline B 文件头注释明确写出）。

输入路径不是 pipeline 自己解析的：`run_task()` 先调 `runner/input_loader.resolve_task_input()`
把 `task.input` 解析成带 `parquet_path` / `lance_path` / `rows` / `dataset_uri` 的
`resolved_input`。注意 `runner/input_loader.py` 与 `paths.py` **在本仓库 git 历史中已被删除**
（`f76bf19`），真实运行依赖测试服上的部署环境；这也意味着该 checkout 开箱不可执行。

### 2. 数据源：契约、来源与 fixture 构造

**输入契约（两列，别的都不要）**

| 列 | 类型 | 用途 |
|---|---|---|
| `doc_id` | string | 文档主键；代表选择（min）、join 键、去重决策依据 |
| `text` | string | 已抽好正文的原始文本 |

代码在读取后立刻校验，缺列直接 `ValueError`：

```python
missing = {"doc_id", "text"} - set(_column_names(docs))
if missing:
    raise ValueError("Pipeline B schema missing: " + ", ".join(sorted(missing)))
```

**支持的输入 kind**：仅 `parquet`（`daft.read_parquet`）与 `lance`（`daft.read_lance`），其余抛
`ValueError("Pipeline B requires parquet or lance input")`。任务声明用的是 parquet：

```json
"input": {"kind": "parquet", "path": "fixtures/pipeline_b/documents.parquet", "field": "text"}
```

上游业务位置：**web 正文抽取之后、tokenizer 之前**（对标 C4 / RedPajama-V2 类预抽取文本），
即 `tasks/text_corpus_minhash_dedup.json` 描述的那条链路的 dedup 段（去 HTML → 去链接 → 空白归一
→ 去版权 → 语言识别 → 重复字符过滤 → 词数过滤 → MinHash 去重）。Pipeline B 是该链路中
**只保留 dedup 段**、并把它重写成原生关系算子的版本。

**fixture 怎么造**（`scripts/data/build_pipeline_b_fixture.py`）

1. 从 `fixtures/text_fineweb_edu_s0.lance` 只取 `text` 列（真实语料）；
2. `texts = [source_texts[i % len(source_texts)] for i in range(cap)]` —— 循环复制到 cap 行，
   cap 取 `--rows` 或环境变量 `VOLC_INPUT_ROW_CAP`，默认 1000；
3. 每 10 行追加一条**近似重复**：`{"doc_id": f"near-{i:012d}", "text": text + " small near duplicate suffix"}`；
4. 写出 `documents.parquet`，schema 显式为 `{doc_id: string, text: string}`。

这样造数据的原因（可由代码反推）：`text + suffix` 使两文档 Jaccard 相似度约 0.9，而本 pipeline 的
8×8 参数下 P(至少一个 band 相撞) ≈ 1-(1-0.9⁸)⁸ ≈ 0.95，既能稳定触发模糊合并，又不是合成假语料；
真实精确重复（Jaccard=1）则 100% 相撞。注意它是**非破坏性**脚本：只读源 lance，不改动源数据。

### 3. 处理步骤逐步拆解（业务含义 + 代码动作）

pipeline 共 7 个逻辑步骤，全部由 Daft 原生算子表达。

| # | 业务步骤 | 关键代码 | 业务含义 |
|---|---|---|---|
| 1 | 读入、行数封顶、定分区 | `_read_documents()` → `_apply_row_cap()` → `into_partitions()` | 扫描语料；`VOLC_INPUT_ROW_CAP` 让同一份 fixture 可跑样本档与规模档；`into_partitions` 决定数据并行度（见 §9） |
| 2 | 文本规范化 + 空行过滤 | `_regexp_replace(_lower(_strip(col("text"))), r"\s+", " ")` → `where(not_null & length>0)` | 抹掉大小写与空白差异，避免"同一文档因排版不同而被判为不同"，提升召回 |
| 3 | MinHash 签名 | `col("normalized_text").minhash(num_hashes=64, ngram_size=5, seed=1, hash_function="xxhash3_64")` | 把文档表示成 5-gram 集合的 64 维指纹；签名一致率 ≈ Jaccard 相似度 |
| 4 | LSH banding（展开） | 循环 8 次：`slice(start, start+rows_per_band)` + `_hash64()` + `union_all` | LSH 核心：相似文档大概率至少撞中一个 band，把全比较降为同桶比较；每文档 1 行变 8 行（**数据膨胀 8 倍**） |
| 5 | 候选分组 | `expanded.groupby("band_idx","band_hash").agg(col("doc_id").min().alias("band_rep"))` | 每个 LSH 桶选最小 `doc_id` 作为该桶候选代表（确定性保留策略） |
| 6 | 代表传播 | `expanded.join(band_representatives, on=["band_idx","band_hash"])` → `groupby("doc_id").agg(col("band_rep").min().alias("component_rep"))` | 一篇文档可能命中多个桶，取这些桶代表的最小值 → 得到一维"连通分量标签" |
| 7 | 选代表 + 写出 + 回读 | `join(labels, on="doc_id")` → `where(doc_id == component_rep)` → `select` → `explain()` 断言 → `write_parquet` → `read_parquet().count_rows()` | 每个候选组只留一篇；先落盘再回读计数，避免每个 action 重跑整条 MinHash+8-band 计划 |

**步骤 4 的 Python 循环不是 UDF**：循环次数取决于 `bands`（配置值），与数据行数无关，只展开
固定的 lazy plan 结构；`slice` 与 `hash` 都是原生表达式。步骤 5–6 是两次全局 shuffle，也是这条
pipeline 的性能主体。

**步骤 7 内置的零 UDF 断言**（本 pipeline 的设计目标即由它守护）：

```python
plan = io.StringIO()
deduped.explain(show_all=True, file=plan)
udf_markers = sum(plan.getvalue().lower().count(x) for x in ("python udf", "pyudf", "actorpoolproject"))
if udf_markers:
    raise AssertionError(f"Pipeline B physical plan contains {udf_markers} Python UDF markers")
```

它检查的是**物理计划文本里的 UDF 标记**（`python udf` / `pyudf` / `actorpoolproject`），不是
运行时采样；这也解释了为什么这几个名字被硬编码在 A/B/C 三条 pipeline 里。

### 4. 最终输出

**数据产物**：`outputs/pipeline_b_data/run-<time_ns>/deduped/**/*.parquet`
（snappy、`write_mode="append"`、**每次运行一个新目录**——既可审计，也避免覆盖已有产物）。

| 输出列 | 来源 | 说明 |
|---|---|---|
| `doc_id` | 原列 | 被保留文档的主键 |
| `normalized_text` | 步骤 2 | 规范化后文本，**原始 `text` 列没有输出** |
| `minhash_signature` | 步骤 3 | 64 维签名，保留以便复现/下游复用 |
| `component_rep` | 步骤 6 | 所属分量代表 ID（等于本行 `doc_id`） |

**指标产物**：`run_pipeline_b` 返回 dict，经 `merge_timing_metrics` 归一后由
`run_perf_suite` 序列化为结果 JSON。

| 字段 | 含义 |
|---|---|
| `elapsed_s` | `ray_init_s + pipeline_execute_s`（端到端） |
| `input_rows` / `output_rows` / `readback_rows` | 输入行数（cap 或 `resolved_input["rows"]`）/ 写出行数 / 回读行数 |
| `pipeline_ops` | 5 个算子名：`native_normalize_minhash`、`native_lsh_banding`、`candidate_groupby`、`representative_join`、`parquet_write_readback` |
| `impl` | `daft-native-minhash-lsh-zero-udf` |
| `invariants` | `{python_udf_nodes: 0, physical_plan_udf_markers: 0}` |
| `timing_breakdown` | `{engine: daft_ray, buckets: {ray_init_s, pipeline_execute_s}}` |
| `candidate_buckets` | **恒为 `None`**——预留字段，未实现 |

**没有输出**（与 Pipeline A/C 的差距）：无"被删除文档"清单、无 dedup 统计表、无 Lance 入湖、
无输入侧强校验（A 会断言 `rows == distinct doc_hash` 且 blocked hash 命中数为 0）。
`operator_timings=[]` 也是**空列表**——只有端到端耗时，没有逐算子耗时。

### 5. 应用场景

**业务场景**：预训练语料治理中的近似去重。Pipeline A 的精确去重只能删"逐字相同"的文档；真实
web 语料里更常见的是转载、模板微调、行尾差异、爬取重复导致的**近似重复**——这类重复会
（1）让模型记忆/复述训练样本、损害泛化，(2) 浪费 token 预算，(3) 使下游评测被重复样本污染。
MinHash-LSH 是工业界（C4、RedPajama、Data-Juicer）处理该问题的标准手段。

**工程场景（这条 pipeline 的真实动机）**：

1. **UDF → Rust 迁移验证**：背景读数见 `docs/reports/PIPELINE_LANGUAGE_PERF_METHOD_CORRECTION_2026-08-24.md`
   —— Pipeline A 的 Rust exclusive CPU 占比 37.12%（其中 Daft Rust 35.82%），而串联 Python UDF
   的原始文本产线只有 3.24%。
2. **制造"可观测的 Rust 计算窗口"**：`PIPELINE_A_RAY_NUMA_PERF_REPORT_2026-08-20.md` 记录
   Pipeline A 太快、Rust 占比被 Ray 固定开销淹没（2.65%）；Pipeline B 把每文档展开 8 个 band，
   产生更多分区/shuffle/聚合/join，实测 B 的 Rust 占比 18.95%（约 A 的 7 倍），**更容易被 perf
   采样到**。这是"用业务负载特征放大被测对象"的基准设计手法。
3. **Ray + NUMA 集群 harness 的负载之一**：单机多 NUMA 模拟多节点 Ray，验证调度、shuffle、
   对象传输与控制面路径（3-NUMA：head 0 CPU，两个 worker 40+38 CPU，共注册 78 CPU）。
4. **跨架构（x86 ↔ 鲲鹏 aarch64）基线**：Pipeline B 与 A/C 一起构成"原生关系算子"这组负载，
   用于 Rust/Kunpeng 亲和性研究。

### 6. 与参考实现、Data-Juicer 原算子的语义差异

**对照 Daft 官方参考实现**：Daft 文档
[Web Text Deduplication](https://docs.getdaft.io/en/stable/examples/minhash-dedupe/) 给出的是
完整版 MinHash 去重（该实现基于
[Connected Components in MapReduce and Beyond](https://dl.acm.org/doi/abs/10.1145/2670979.2670997)
的 star-contraction）。Pipeline B 复用了它的前半段，截断了后半段：

| 环节 | Daft 官方实现 | Pipeline B | 差异影响 |
|---|---|---|---|
| 文本规范化 | `normalize(remove_punct=True, lowercase=True, nfd_unicode=True, white_space=True)` | 仅 lower + strip + 折叠空白 | 不去标点、不做 NFD Unicode 归一 → 相似度估计偏高、跨系统不可比 |
| MinHash | `minhash(num_hashes=K, ngram_size=5, seed=42, hash_function='xxhash')` | 同 API，`num_hashes=64, ngram_size=5, seed=1, hash_function="xxhash3_64"` | 参数与哈希变体不同，签名不可跨实现比较 |
| band 切分 | `list.chunk(R)` + `explode` | Python 循环 8 次 `slice` 后 `union_all`（优化分支改为 `explode`） | 语义等价；原版本计划长度随 band 数增长 |
| 桶键 | `groupby(band_idx, bands)` 收集节点列表 | `groupby(band_idx, band_hash)` 取 min | 等价且更省内存（不物化 nodes 列表） |
| **分量求解** | 显式边表 + Large-star/Small-star 交替迭代至收敛 + 全局最小标签传播（上限 100 轮） | **单轮 min 传播**，无迭代、无收敛判据 | **只合并直接相连的文档**；A–B、B–C 相似但 A–C 不同桶时漏合并 |
| 校验 | 与 igraph 连通分量结果比对 | 只做物理计划零 UDF 断言 + 行数回读 | 无算法正确性校验 |
| 产物 | 去重视图 + 重复样本表 | 只有去重视图 | 无法抽样检查被删样本质量 |

**对照 Data-Juicer 原算子** `document_minhash_deduplicator`：本仓库自己的实现
`ops/text_ops.py:minhash_lsh_key()` 是**纯 Python 单 band** 版（`num_perm=16, ngram=5,
rows_per_band=4`，md5 手算 minhash，只取第一个 band 作单一 groupby 键，注释写明"单 band 是为
groupby 单键的简化，多 band 需 explode"）。它与 Pipeline B 的关系：

| 维度 | 仓库内 DJ 风格实现 | Pipeline B |
|---|---|---|
| 执行位置 | Python UDF（逐行）+ 一次 groupby | Daft 原生表达式 + 两次 groupby + 一次 join |
| 相撞概率 | 单 band：`s^rows_per_band`（近似重复召回明显更低） | 8 band：`1-(1-s^8)^8` |
| 分片语义 | "词切分 + 空格 join"的 shingle | Daft `minhash` 内部 token n-gram（实现不在本仓库） |
| 定位 | 功能可用、DJ 语义桥接 | 性能基准与原生迁移目标 |

`configs/pipelines/formal_pipelines.json` 因此把 `text_corpus_minhash_dedup` 标为
`implementation_divergent`，并注明"Daft 两机通过，DJ 路径 partial/divergent，不得用作
Daft-vs-DJ 公平横比"。**跨系统比较 MinHash 去重时，normalization / ngram / 哈希数 / seed /
band 数 / 代表选择规则必须逐项对齐**，否则比的是参数不是实现。

### 7. 参数说明与调参影响

| 参数 | 默认 | 作用 | 调大的后果 |
|---|---:|---|---|
| `num_hashes` | 64 | 签名长度 = 相似度估计精度 | 估计更准，计算与签名内存线性增长 |
| `ngram_size` | 5 | shingle 粒度 | 越大越"字面严格"（对改写不敏感），越小越易误判相似 |
| `bands` | 8 | LSH 桶数（需整除 `num_hashes`） | band 越多召回越高、误报越多、膨胀倍数越大 |
| `seed` | 1 | MinHash 排列种子 | 影响可复现性；跨系统对比必须一致 |
| `VOLC_INPUT_ROW_CAP` | 未设 | 输入行数封顶 | 控制样本档/规模档；未设时 `input_rows` 取 `resolved_input["rows"]` |
| `into_partitions`（profile） | 未设（=0 不改） | 重分区数 | 见 §9：对 minhash 结构的净收益为负 |
| `VOLC_PIPELINE_B_MATERIALIZE_MINHASH` | 未设 | 优化开关，见 §8 | — |
| `VOLC_PIPELINE_B_ONE_PASS_BANDS` | 未设 | 优化开关，见 §8 | — |

`num_hashes=64, bands=8` ⇒ `rows_per_band=8`。相撞概率 `P(s) = 1-(1-s⁸)⁸`：`s=0.9 → ≈0.95`，
`s=0.8 → ≈0.61`，`s=0.5 → ≈0.03`——**这条曲线就是本 pipeline 实际使用的"相似度阈值"**，比
Daft 官方示例里用 `optimal_param(0.7, 64)` 反解 (B,R) 的做法更粗略。参数校验只有一条：
`num_hashes` 必须为正且能被 `bands` 整除，否则 `ValueError`。

### 8. 两个优化分支（HEAD 之后，均未合并）

两个分支都在 `stone/*` 远端，同日提交（2026-08-27），**都用环境变量做 opt-in 开关**，
保留 baseline 计划可复现——这与仓库"性能建议是 opt-in、不是不变式"的既有约定一致。

**`stone/pipeline-b-opt-one-pass-bands`（4700081，+26/−9）**：把 8 次 `select+union_all`
换成一次原生 `explode`：

```python
if os.environ.get("VOLC_PIPELINE_B_ONE_PASS_BANDS", "").strip() == "1":
    band_start = col("band_idx") * lit(rows_per_band)
    expanded = (
        docs.select("doc_id", "minhash_signature")
        .with_column("band_idx", lit(list(range(bands))))
        .explode("band_idx")
        .with_column("band_hash", _hash64(col("minhash_signature").slice(band_start, band_start + lit(rows_per_band))))
        .select("doc_id", "band_idx", "band_hash")
    )
```

动机：原版本每个 band 一条分支，8 个分支各自完整消费 `docs` 计划（规范化 + MinHash 可能被
重复计算），且 `union_all` 会把 8 个分支的计划串成更长的物理计划。改成 `lit(list) + explode`
后，MinHash 只出现一次，计划更短——这也正是 Daft 官方示例的写法。

**`stone/pipeline-b-opt-materialize-minhash`（1e77183，+6）**：在 band 展开前插入一次物化：

```python
if os.environ.get("VOLC_PIPELINE_B_MATERIALIZE_MINHASH", "").strip() == "1":
    docs = docs.collect()
```

注释写明："band 分支都消费同一份规范化 + MinHash 投影；物化一次可避免为每个 union 分支
重算该投影。保持 opt-in 以便 baseline 物理计划仍可获取。" 代价是把流水线切断（全量签名进
driver 内存），只适合小样本档验证，不适合规模档。两个分支是**互补**的：one-pass 从计划结构上
消除重复计算，materialize 从执行上强制复用结果。

### 9. 并发与 `into_partitions` 的结构性限制

`docs/ONBOARDING_CONCEPTS.md` 记录了 sweep 实验的结构性结论：**minhash 去重 α ≈ 1.0**
（可并行份额为零），**N\* = 1**，切分区净亏损；`configs/parallelism_policy.json` 把
`pipeline_text_fineweb_full_min` 与 `text_corpus_minhash_dedup` 列为该规则的证据 pipeline，
依据是"α≈0.99/1.05、N\*=1、分区净负收益（全局 shuffle/barrier 结构边界）"，并注明
"性能建议非不变式：sweep 可显式 override 超过以测反扩展曲线"。同一文档还给出一个关键教训：
sweep 最初扫的是 `ray_num_cpus`，曲线全平——**因为输入只有一个分区，撑再大的集群也只有一个
task 在跑；真正的数据并行旋钮是 `into_partitions`**。

这与 Pipeline B 在 NUMA 实验里的配置形成直接张力：集群 profile 设 `into_partitions=16`
（报告说明目的是"确保计划产生分区和集群工作节点任务，而不是退化为 driver 单分区快速执行"），
而本 task 的 `engine_overrides` 只设了 `ray_num_cpus: 4`、**没设 `into_partitions`**，
即默认单分区执行。两种设定服务两个不同目的：**要测集群调度路径就重分区，要测算法结构效率
就 N=1**。报告里 B 的端到端 87.139 秒（100 行、带 perf 采样、3-NUMA）不能读作算法性能，
它主要包含 Ray 固定控制开销。

### 10. 工程质量约束与风险

**明确的强约束**：零 Python UDF（物理计划断言，失败即 `AssertionError`）；`doc_id`/`text`
缺列 loud fail；`num_hashes % bands == 0`；`VOLC_INPUT_ROW_CAP` 必须为正整数。

**风险与缺口**：

1. **代表性正确性**：单轮 min 传播不是连通分量。它只保证"直接共享桶"的文档合并；
   生产版本需按官方示例补 star-contraction + 全局最小标签传播直至收敛。
2. **无法判断 dedup 是否生效**：去重比例需另行计算（`output_rows` 与输入行数之差），
   而 Pipeline B 连"空结果"都会表现为 `output_rows > 0`。对比 `golden_acceptance.json` 给
   `text_corpus_minhash_dedup` 定的合同是 `expected_filter_effect: has_kept_and_dropped`
   （必须既有保留又有剔除，全留即判去重没生效）——**本 pipeline 自身没有这个保护，注意别把
   "跑通"当成"去重有效"**。
3. **哈希函数降级链**：`_hash64()`（从 `pipeline_a` 复用）依次尝试
   `hash(seed=0, hash_function="xxhash3_64")` → `hash(hash_function=...)` → `hash(seed=0)` →
   `hash(0)` → `hash()`，任何一步 `AttributeError/TypeError` 就降级。**任务声明 `impl` 与
   `expected` 承诺的是 xxhash3_64，但降级时实际实现可能不同**，跨环境/跨 Daft 版本复现需核对。
4. **输入侧零校验**：不校验 `doc_id` 唯一性，不校验文本长度分布，不校验源指纹
   （`input_fingerprint` 依赖 `resolved_input` 侧提供）。
5. **无删除清单**：调试"为什么这篇被删了"只能重算，无法从产物回答。
6. **环境依赖**：`runner/input_loader.py`、`paths.py` 不在仓库（分别在 `f76bf19` 被删/未跟踪），
   换机器需自备；Daft 版本 pin 在 `deploy/benchmark_env_pins.yml` 为 `0.7.2`。

### 11. 运行方式速览

```bash
# 1) 造 fixture（真实语料 + 人造近似重复；默认 1000 行）
python scripts/data/build_pipeline_b_fixture.py \
    --source fixtures/text_fineweb_edu_s0.lance \
    --output-dir fixtures/pipeline_b --rows 1000

# 2) 单次执行（需已 export VOLC_DE_BENCH_ROOT，路径解析依赖部署环境）
python runner/run_perf_suite.py \
    --task tasks/pipeline_text_minhash_lsh_dedup.json \
    --engine daft_ray --cluster_profile '{"ray_num_cpus":4}' --rounds 1

# 3) 走正式入口（task 需在 configs/pipelines/formal_pipelines.json 中登记才不告警）
TASK=pipeline_text_minhash_lsh_dedup PROFILE_NAME=smoke bash scripts/pipelines/run_pipeline.sh
```

结果 JSON 落在 `$VOLC_DE_BENCH_ROOT/bench-results/`；数据落在
`outputs/pipeline_b_data/run-<time_ns>/deduped/`。注意该 task **未登记**在
`configs/pipelines/formal_pipelines.json`，`run_pipeline.sh` 会告警并退化为只跑 `daft_ray`。

## 延伸

### 与其他候选 pipeline 的关系

| Pipeline | 业务 | 主要算子压力 | Rust 占比（100 行 NUMA 实验） |
|---|---|---|---|
| A | 训练语料整理 + **精确**去重 | 一次 groupby + join | 2.65% |
| **B** | **近似**去重（MinHash-LSH） | 8× 膨胀 + 两次 groupby + join | **18.95%** |
| C | 模型预测/标签分析 | 多表 join + coverage 检查 | 1.84% |
| 原始文本产线 | Data-Juicer 风格清洗链 | 大量 Python UDF | 0.91% |

三条候选 pipeline 的共同主张：**把可关系化的部分下推给 Daft，Rust 侧才有可测量的计算窗口**。
Pipeline B 因为自带 8 倍数据膨胀，是三者中把 Rust 窗口放得最大的一条。

### 与 mindvault 其他笔记的关系

- `daft-kunpeng-udf-inventory.md`：登记待下沉的算子（本 pipeline 是"已下沉"的样本）。
- `daft-kunpeng-perf-design-spec.md` / `daft-kunpeng-overall-design.md`：UDF→Rust kernel 的
  总体设计；`overall-design.md` 已引用 `pipeline_builder.py` 的 mapper 链融合。
- `new_pipeline_job.md`：本 commit URL 的原始记录（该文件是笔记草稿，未进 `SUMMARY.md`）。

## Q&A

**Q：为什么 Pipeline B 的输出不会丢文档？**
A：`labels` 对每个文档至少有一个条目（它至少属于一个 band），且 `component_rep ≤ doc_id`，
所以 `where(doc_id == component_rep)` 在每个分量的最小值那行必然命中一次。孤立文档
`band_rep = 自己`，因此原样保留——**输出行数 = 分量数**，这既是它的正确性来源，也是它无法
自证去重生效的原因。

**Q：`band_hash` 不带 `band_idx` 也能正确分组吗？**
A：带。`groupby("band_idx","band_hash")` 的桶键包含 band 位置，因此"签名第 3 段的哈希值"
与"第 5 段的哈希值"即使数值相同也不会被合并——这正是 LSH 的语义要求。哈希碰撞导致的误合并
是概率性的、可接受的代价。

**Q：`doc_id` 的 min 选择意味着保留哪一篇？**
A：字典序最小的 `doc_id`（fixture 里是 `doc-000000000000` 这类顺序 ID，所以约等于"最早/最靠前
的那篇"），且这是全链路唯一的确定性策略——`band_rep`、`component_rep`、"是否保留"三处都用它。
换成随机或按质量分保留，只需替换这三处的聚合表达式。

**Q：为什么先 `write_parquet` 再回读计数，而不是先 `count_rows()`？**
A：代码注释写明了原因——MinHash + 8-band union + shuffle 的 lazy 计划每次 action 都会重跑，
先落盘再从小结果回读，避免验收动作重复触发整条昂贵计划。这是"昂贵计划只算一次"的工程模式。

## 参考资料

- [PR #2 commit 02b65df：feat: add pipeline B minhash lsh dedup](https://gitcode.com/XuanYuL5/volc_operator_sim/commit/02b65dfd9bdb0db26359fc903893cc84b30a4d59?ref=new_pipeline_script&prId=2)（访问日期 2026-09-14）
- 本地仓库 `/home/suine/projects/volc_operator_sim`（commit `daab395`）：`runner/pipeline_b.py`、`runner/pipeline_a.py`、`runner/pipeline_c.py`、`runner/run_perf_suite.py`、`ops/text_ops.py`、`tasks/pipeline_text_minhash_lsh_dedup.json`、`configs/pipelines/formal_pipelines.json`、`configs/parallelism_policy.json`、`configs/golden_acceptance.json`
- 本地报告 `docs/reports/PIPELINE_A_RAY_NUMA_PERF_REPORT_2026-08-20.md`（§2.2 对 Pipeline B 的描述与实测读数）
- 本地报告 `docs/reports/PIPELINE_LANGUAGE_PERF_METHOD_CORRECTION_2026-08-24.md`（Rust/Python 语言占比口径）
- 本地说明 `docs/ONBOARDING_CONCEPTS.md`（USL、α≈1.0、N\*=1、`into_partitions`）
- [Daft 文档：Web Text Deduplication（MinHash + LSH + Connected Components）](https://docs.getdaft.io/en/stable/examples/minhash-dedupe/)（访问日期 2026-09-14）
- [Daft API：daft.functions.minhash](https://docs.getdaft.io/en/stable/api/functions/minhash/)（访问日期 2026-09-14）
- [Kiveris et al., Connected Components in MapReduce and Beyond](https://dl.acm.org/doi/abs/10.1145/2670979.2670997)（star-contraction 原始论文）
- [Nelson Elhage, Finding Near Duplicates with Jaccard Similarity and MinHash](https://blog.nelhage.com/post/fuzzy-dedup/)（MinHash/LSH 直觉解释）
