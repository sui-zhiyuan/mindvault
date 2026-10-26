---
name: sve-opt-land
description: 落地 PCA256_SVE优化点清单.md 中的一个优化点（如 Q3、B5、N2）。按固定流程实现、构建、做内核微基准与数值等价检查、跑三轮 GIST1M 基准、写记录文档到 sve-opt-records/、回填清单"真实收益"列。有收益的条目合入并默认开启；等价检查通过但本机无收益的 SVE 内核合入但默认关闭（运行时开关），供宽 SVE 硬件复测；等价检查失败或实现不完整的条目只提交记录、patch 附件、清单与结果。每次落地后重新 perf 剖析、按新占比刷新清单并推荐下一条。落地中发现的新优化点必须追加进清单。用法：/sve-opt-land Q3
---

# 落地一个 SVE 优化点

参数 `$ARGUMENTS` 是清单编号（S0 到 S4、Q1 到 Q10、B1 到 B10、N1 到 N13，或后来追加的编号）。
没有给编号时先用 `AskUserQuestion` 询问。所有路径相对仓库根目录 `/home/wheel/lance-pca-10.0`。

## 固定资产

| 用途 | 路径 |
|---|---|
| 优化点清单 | `PCA256_SVE优化点清单.md` |
| 记录文档 | `sve-opt-records/<编号>-<英文短slug>.md`，模板在本文件末尾 |
| 记录附件 | `sve-opt-records/<编号>-attachments/`：未合入代码的 patch、独立微基准源码、README |
| 已有记录（先读） | `sve-opt-records/P0-perf-profile.md`（**查询 p50 的剖析分解，选条目前必读**）、`S0-S2-sve-infrastructure.md`、`Q1-sve-fastscan.md`、`Q2-sve-l2-f32.md`、`Q3-sve-prune.md` |
| 剖析工具 | `benchmarks/pca_ivf_rq/sve_opt_profile.py`（封装 `perf record -g` + `perf report`，输出固定阶段表、内核自身占比、DSO 分布，`--previous` 给出逐阶段差分，并追加一行到 `sve-opt-records/profile-history.md`）；工作负载脚本 `benchmarks/pca_ivf_rq/query_loop.py` |
| 剖析历史 | `sve-opt-records/profile-history.md`：每次落地一行，看 N14 / N16 / N17 / Q8 / 内核合计的占比走势 |
| 最新剖析 | `sve-opt-records/<上一个落地编号>-attachments/profile.json`，作为本次 `--previous` |
| 基准协议 | `GIST1M_PCA_IVF_RQ端到端测试指南.md` 第 6 节；结果放 `benchmarks/pca_ivf_rq/results/` |
| 清单脚本 | `benchmarks/pca_ivf_rq/sve_opt_record.py`：`show` / `fill` / `set` / `add` / `compare` |
| 数值等价脚本 | `benchmarks/pca_ivf_rq/sve_opt_equivalence.py`：`dump` / `diff` / `index-hash` |
| SVE 编译配置 | `.cargo/sve.toml`；`cd python && make build-sve`（内部已用 `uv run --no-sync`，勿去掉，否则 uv 会先按默认配置重建一次） |
| 运行时档位 | `python -c "import lance; print(lance.simd_info())"`，本机 tier `sve`、`sve_vector_length_bytes` 32 |
| 公共基线索引 | `benchmarks/pca_ivf_rq/data/gist1m/work-S0-base/{baseline,pca_256_plain}.lance`，默认配置 wheel 所建；对应 dump `results/S0-base-{baseline,pca256}-ids.npz` |
| 微基准范例 | `sve-opt-records/Q1-attachments/{fastscan_variants.c,issue_rate.c}` |

所有 Python 命令用 `uv run --no-sync --project python python ...`（或在 `python/` 下 `uv run --no-sync python ...`）。

## 本机硬件事实（决定预期收益是否成立）

- CPU MIDR 0x480fd020（HiSilicon implementer 0x48，part 0xd02），SVE VL = 256 bit，无 SVE2。
- **所有 SVE 指令发射率为 0.56 cycles/instr，NEON 为 0.28 到 0.36**（`issue_rate.c` 实测，含 `tbl`、`add`、`lsr`、`ext`、`ld1b`、`fmla z.s`）。256 位 SVE 指令每周期处理的字节数与 128 位 NEON 持平。
- 推论：**等宽改写（把 NEON 内核逐条换成 SVE）在本机必然为负收益**（Q1 实测慢 1.25 到 1.55×）。SVE 只有在**减少指令数**时才可能获益：谓词消除尾部与分支、`COMPACT` 压缩幸存者、gather/scatter 替代标量循环、更大寄存器堆减少溢出、FMMLA 类块指令（需单独验证）。
- 全局 `+sve` 自动向量化（S0）对查询延迟无可观测收益，热点均在手写 NEON intrinsics 中。
- **跨 lane 归约（`ORV` / `ADDV` / `FADDV`）代价高且不可重叠**：Q3 实测 4 次独立 `ORV` 比合并为 1 次快 20%，无归约的谓词域路径再快 20%。谓词域操作（`CNTP`、`PTEST`、`BIC`、`NOR`、`WHILELT`）便宜。可复用模式："常见情况留在谓词域，只在必要时提取掩码或归约"（Q3 的无幸存者早退出）。
- 已验证的正收益来源：Q2（FMLA 合并乘加 + 4 行分块共享加载 + 每行一次归约）内核 1.4 到 1.6×；Q3（谓词早退出）内核 1.36×。两者端到端均在噪声内，因为对应阶段占查询 p50 不到 1%。**选条目时优先看该阶段在 p50 中的占比**，而不只看内核加速比。
- **P0 剖析（2026-09-23）给出的 p50 分解**：查询路径全部 SIMD 内核合计不到 15%（PCA 变换 6 到 9%、FastScan 3 到 4%、ex_dot 约 2%、剪枝约 1%、L2 不到 1%）；每查询重建 IVFIndex 与 318 个 scratch 预分配约 30%（N14，`LANCE_CPU_THREADS=2` 实测 -21% 到 -28% p50）；DataFusion 计划与流构建约 10%（N16）；refine 的 take 路径 10 到 15%（N17）；OpenBLAS 自旋线程 11% 样本（N15）。在 N14 / N16 / N17 落地前，任何内核类条目都不可能超过噪声线。
- 因此在第 1 步就要判断：条目方案若只是"lane 翻倍"，直接按"预期无收益"处理，可跳到只做最小验证（内核微基准）并记录，不必跑完整流程；确有指令数减少或算法改动的才走完整流程。

## 流程

### 第 1 步：读取条目，确定验收类别与 A/B 方式

1. `sve_opt_record.py show --id <编号>`，读出位置、方案、数值影响、路线、优先级。读已有记录文档，避免重复 Q1 已证伪的路径。
2. 若"真实收益"已有内容，停下来用 `AskUserQuestion` 确认是重做还是跳过。
3. 验收类别（按"数值影响"列）：
   - **位精确**：查询侧 `diff --require-bit-exact` 通过；构建侧 `index-hash` 一致（N13 落地前不可用，改以 recall 验收）。
   - **ulp 级**：记录 top-10 集合变化条数与最大距离偏差，三轮 recall 均值差不超过 0.5 pt。
   - **模型变更**：仅以三轮 recall 均值差不超过 0.5 pt 验收，记录中注明已缓存索引需重建。
4. A/B 方式，按优先级选择：
   - **同一 wheel + 运行时开关**（首选）：新内核在 `rust/lance-linalg/src/simd/sve.rs` 的 `SVE_KERNEL_NAMES` 中登记一个名字，并新增 `PREFER_SVE_<NAME>` 静态开关，由 `LANCE_SVE_KERNELS=<name>` 单独启用。**只开本条目的内核**，不要用 `all` 或旧的 `LANCE_PREFER_SVE_KERNELS=1`，否则已合入的其他 SVE 内核（如 Q1 负收益的 fastscan）会混入结果。对照与候选用同一二进制，按 base/cand **交替**各跑三轮。
   - **两个 wheel**：无法加开关时，对照为同一编译配置下不含本条目改动的构建。依赖 S0 的条目两边都用 `make build-sve`。
   - `S*` / `N*` 条目：对照为本条目之前的 HEAD 构建。
5. 检查当前安装的 wheel 与 HEAD 是否一致（上一个条目若有未合入的 patch 代码仍留在 wheel 里，先按 HEAD 重建）。已合入但默认关闭的内核不影响对照，无需重建。

### 第 2 步：建立对照基线

对照与候选必须同机、同 toolchain、同编译配置。已有且满足条件的三轮结果可复用，记录中写明文件名。

```bash
cd python && uv run --no-sync maturin develop --uv --release     # 或 make build-sve
cd ../benchmarks/pca_ivf_rq/results
PHYSICAL_CORES=$(lscpu -p=CORE,SOCKET | grep -v '^#' | sort -u | wc -l)
for i in 1 2 3; do
  OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 \
  uv run --no-sync --project ../../../python python ../run.py \
    --mode same-recall --data ../data/gist1m/benchmark \
    --dimensions 256 --variants plain --num-partitions 256 --num-bits 4 \
    --target-recall 0.90 --nprobe-grid 16,24 --refine-factors 2,3 \
    --max-queries 1000 --warmup-queries 20 \
    --output <编号>-base-run$i.json --csv-output <编号>-base-run$i.csv > <编号>-base-run$i.log 2>&1
done
```

BLAS 线程数在查询阶段必须为 1（N15：160 个 OpenBLAS 自旋线程占 11% 样本；`PHYSICAL_CORES` 只用于 `ground_truth.py`）。**不要设置 `LANCE_CPU_THREADS`**：它会同时改变 scratch pool 大小（N14）和构建并行度，除非条目本身就是 N14 的验证。已有的 S0 到 Q11 结果均在 `OPENBLAS_NUM_THREADS=160` 下产生，新条目的对照组必须与候选组同设置，不可与旧结果直接混比。

同一 wheel 方式下，对照与候选合并为一个循环：`for i in 1 2 3; do for side in base cand; ...`，候选一侧设置开关环境变量。

查询侧条目的等价检查直接复用公共基线索引与 `S0-base-*-ids.npz`，无需 `--keep-work-datasets`。构建侧条目在第 1 轮加 `--keep-work-datasets --work-dir ../data/gist1m/work-<编号>-base`。

### 第 3 步：实现

1. 只改条目"位置"列指向的代码及必要的分派 / 构建入口。SVE 代码必须：
   - 通过 `SimdSupport::Sve` 运行时档位分派，保留 NEON 或标量回退；
   - C 内核放 `rust/lance-linalg/src/simd/sve.c`，符号加 `_sve` 后缀，Rust 绑定放 `simd/sve.rs`，`#[cfg(all(kernel_support = "sve", target_arch = "aarch64"))]` 门控；固定 VL 假设的内核用 `sve::FIXED_WIDTH_KERNELS_AVAILABLE` 门控；
   - nightly intrinsics 路线需 `#[cfg(feature = "nightly")]` 门控；
   - 浮点内核不得引入 FMA 融合或改变归约顺序，除非条目已标为 ulp 级或模型变更（C 内核已统一 `-ffp-contract=off`）；
   - **加运行时开关**：在 `sve::SVE_KERNEL_NAMES` 登记内核名，新增 `pub static PREFER_SVE_<NAME>: LazyLock<bool> = LazyLock::new(|| kernel_enabled("<name>"))`，分派处只检查该开关。`lance.simd_info()["sve_kernels_enabled"]` 可观测。
2. 单元测试：与现有 NEON 或标量实现逐位比对，覆盖非 8 / 16 / 32 倍数的尾部长度和会触发整数回绕的输入。注意 `sum_4bit_dist_table_scalar` 用饱和加而 NEON 用回绕加，SIMD 内核以 NEON 为准。
3. `cargo fmt`；`cargo clippy -p <crate> --lib --tests --benches -- -A clippy::chunks_exact_to_as_chunks`（nightly 新 lint，仓库既有代码未适配）；`cargo test -p <crate>`。默认与 `--config .cargo/sve.toml` 两种配置各跑一次。测试中不要用 `eprintln!`，clippy 禁止。
4. **内核微基准（内核类条目必做，先于端到端）**：在 `rust/lance-linalg/benches/` 对应 bench 中增加钉死 NEON 与 SVE 内核的分组，`cargo --config .cargo/sve.toml bench -p lance-linalg --bench <name>`。若内核级已慢于现有实现，端到端仍要跑，但可以只跑最小规模（`--max-queries 200`）确认方向后按无收益处理。
5. 构建 wheel：`cd python && uv run --no-sync maturin develop --uv --release` 或 `make build-sve`，约 10 分钟。确认 `lance.simd_info()["tier"] == "sve"`，`nm` 能找到新符号，`objdump -d` 目标函数含 SVE 指令（`z[0-9]+\.`、`whilelt`、`tbl z`）。

### 第 4 步：候选三轮基准

与第 2 步相同命令，输出 `<编号>-cand-run{1,2,3}`。同一 wheel 方式下已在第 2 步交替完成。

### 第 5 步：数值等价检查

- 查询侧：候选路径在**公共基线索引**上 dump，与 `S0-base-pca256-ids.npz`（nprobe 16 / rf 3）和 `S0-base-baseline-ids.npz`（nprobe 16 / rf 2）分别 `diff`；同一 wheel 方式下再加一组对照路径 dump 与候选路径 dump 的 diff。位精确条目加 `--require-bit-exact`。
- 构建侧：`index-hash` 对比 `work-<编号>-base` 与 `work-<编号>-cand` 的 `pca_256_plain.lance`（N13 前不可用）。
- 输出保存为 `<编号>-equivalence-{pca256,baseline}.json`。

### 第 6 步：对比与判定

```bash
uv run --no-sync --project ../../../python python ../sve_opt_record.py compare \
  --baseline <编号>-base-run1.json <编号>-base-run2.json <编号>-base-run3.json \
  --candidate <编号>-cand-run1.json <编号>-cand-run2.json <编号>-cand-run3.json \
  --target-recall 0.90 --json-output <编号>-compare.json
```

- 查询侧主指标：达标点表中 `pca_256_plain` 的 Δp50 / Δp95；同时必须报告 `baseline` 配置的变化，共享内核会同时影响两者，**任一配置显著劣化即判定失败**。
- 构建侧主指标：构建指标表中对应阶段 Δ。`pack_ex_codes` 在 N4 前不可用，用 shuffle 总时长。
- |Δp50| 小于 3% 为噪声，判定"无显著变化"。
- **默认开启的门槛**：一组三轮达到 |Δp50| ≥ 3% 后，必须再跑一组独立三轮（交替顺序反转）确认，六轮合并仍 ≥ 3% 且两组方向一致才判"有效"。Q3 第一组 -4.7% 在确认组反转为 +2.5%，单组三轮不足以判定。
- 等价检查未通过判定"失败"。
- 判定只有三种：**有效 / 无显著变化 / 失败**。

### 第 7 步：写记录文档

按末尾模板写 `sve-opt-records/<编号>-<slug>.md`。必须包含：改动清单、开关与门控方式、单测覆盖、内核微基准表、等价结果、`compare` 三张表原样粘贴、结论与根因、处置、新发现、遗留。

### 第 8 步：回填清单

```bash
uv run --no-sync --project python python benchmarks/pca_ivf_rq/sve_opt_record.py fill --id <编号> --text "..."
```

文本格式：
- 有效：`p50 -x.x% / p95 -x.x% / recall +0.00 pt / 位精确 ✓ / 记录: sve-opt-records/<编号>-<slug>.md`
- 构建侧：`<阶段> x.xx→x.xx s (-x.x%) / recall +0.00 pt / index-hash 一致 / 记录: ...`
- 无显著变化：`无显著变化：pca_256 p50 +x.x% / p95 ...（噪声内）/ ... / 记录: ...`
- 失败（负收益，已合入默认关闭）：`失败（负收益）：<一句话数据> / 根因：... / 代码已合入但默认关闭，LANCE_SVE_KERNELS=<name> 开启，待宽 SVE 硬件复测 / 记录: ...`
- 失败（未合入）：`失败（未通过等价检查 | ...）：<一句话数据> / 代码未合入，实现见 sve-opt-records/<编号>-attachments/<...>.patch / 记录: ...`

### 第 9 步：重新剖析（每次落地后必做）

落地后的默认路径是后续所有条目的起点，所以每次都要重新采样并与上一次比较：

```bash
cd benchmarks/pca_ivf_rq/results
uv run --no-sync --project ../../../python python ../sve_opt_profile.py \
  --label <编号> \
  --dataset ../data/gist1m/work-S0-base/pca_256_plain.lance \
  --queries ../data/gist1m/benchmark/queries.npy \
  --seconds 30 \
  --previous ../../../sve-opt-records/<上一个落地编号>-attachments/profile.json
```

- 默认剖析的是**默认路径**（开关全关）。条目合入但默认关闭时，再跑一次 `--env LANCE_SVE_KERNELS=<name> --out sve-opt-records/<编号>-attachments/profile-switched`，记录开关开启后的阶段变化，但**不**写入 history（加 `--no-history`）。
- 输出 `sve-opt-records/<编号>-attachments/profile.{md,json}`，history 自动追加一行。
- 把阶段表与"Δ vs 上一次"粘进记录文档新增的"## 落地后剖析"一节。本条目对应阶段的占比应下降；若没有下降，说明内核未进入该路径或占比估计有误，要在记录中解释。
- 判定条目是否"有效"仍以第 6 步的三轮基准为准，剖析只用于归因与选下一条。

### 第 10 步：追加与调整清单条目（必做检查）

遇到以下情况用 `add` 追加新条目，而不是只写在记录里：
- 发现清单未覆盖的热点、冗余计算、串行段、可疑计时；
- 本条目因某个前置问题无法达到预期（内存带宽、分配、锁、I/O、发射率）；
- 等价检查暴露的既有数值不一致；
- 编译配置、分派层、构建脚本层面的缺口。

```bash
uv run --no-sync --project python python benchmarks/pca_ivf_rq/sve_opt_record.py add --prefix <S|Q|B|N> \
  --cells "<节点>||<位置 file:line>||<当前形态>||<SVE 方案>||<预期收益>||<数值影响>||<路线>||<优先级>"
```

`S` / `Q` / `B` 表列顺序：节点、位置、当前形态、SVE 方案、预期收益、数值影响、路线、优先级（`S` 表无"节点"列，7 项）。`N` 表：节点、位置、问题、修复、预期收益、数值影响、优先级（7 项，优先级固定"低（非 SVE）"）。

**按新剖析刷新清单**：对第 9 步阶段表里占比变化超过 1 pt 的条目（N14 / N16 / N17 / Q8 / N3 / Q1 到 Q4 / Q11 等，脚本已在阶段名里标注编号），用 `set --column 优先级 --append` 追加一句 `（<编号> 后剖析：<阶段> x.x%）`；占比已低于 1% 的条目下调为"低"并注明。已落地条目的"真实收益"列不改。

本条目的结论若使其他条目的预期不再成立（如 Q1 证伪等宽改写），用 `set` 调整它们的优先级并加注，注中引用本编号：

```bash
uv run --no-sync --project python python benchmarks/pca_ivf_rq/sve_opt_record.py set --id Q4 --column 优先级 --append \
  --text "（<编号> 实测：<一句话原因>，见 <相关编号>）"
```

追加与调整都在记录文档"新发现"一节列出，并在最终回复中告知用户。

### 第 11 步：提交

按判定与实现质量决定代码去向：

| 情况 | 代码 | 默认状态 |
|---|---|---|
| 有效 | 合入 | 开启 |
| 无显著变化 / 失败（负收益），且等价检查通过、单测通过、分派干净 | **合入** | **关闭**：`LANCE_SVE_KERNELS=<name>` 按内核开启，默认走原路径 |
| 失败（等价检查不通过 / 实现不完整 / 需要 nightly 且无回退） | 不合入 | 保存为 patch 附件 |

合入但默认关闭的目的，是让同一份代码能在宽 SVE（全速流水线）硬件上不改代码直接 `make build-sve` 后开开关复测；记录文档"处置"一节要写明复测步骤和内核名，并说明 `lance.simd_info()["sve_kernels_enabled"]` 可观测字段。默认关闭的内核必须保证默认路径与合入前逐位一致（第 5 步同 wheel 的对照路径 dump 即为证据）。

不合入的情况，提交前：

1. `mkdir -p sve-opt-records/<编号>-attachments && git diff <对照 commit> -- rust/ python/src > sve-opt-records/<编号>-attachments/<编号>-<slug>.patch`；独立微基准源码与 README 也放进该目录；
2. `git checkout -- rust/ python/src`，确认 `git diff --stat <对照 commit> -- rust/ python/src` 为空，`git apply --check` 该 patch 通过；
3. 记录文档"改动"一节标题改为"改动（已测量、未合入，见 patch）"。

通用规则：
- 一个条目一个 commit，信息格式 `perf(sve): <编号> <一句话>`，正文含 Δp50 / Δp95 / 等价结果 / 判定，末尾 `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`。
- `results/<编号>-*`（run JSON/CSV/log、dump npz、equivalence/compare JSON、runs.status）随 commit 提交；`data/` 已 gitignore，work 索引不提交。
- 提交后 `git status` 应为空；`git diff --stat <对照 commit> -- rust/` 与上表的代码去向一致。

### 第 12 步：汇报

最终回复：编号与方案一句话、判定、达标点表（含 baseline 与 pca_256 两行）、等价结论、内核微基准要点、回填文本、新追加与调整的编号、遗留问题、**当前安装 wheel 的状态**（是否含未合入代码，下一条目是否需重建）、**落地后剖析摘要与下一条推荐**。

下一条推荐的规则：
1. 从第 9 步阶段表中取 inclusive 占比最高、且尚未落地（"真实收益"为空）的条目；同占比下优先位精确、改动小、不依赖 nightly 的；
2. 若最高占比条目有前置（如 B4 依赖 B11，Q3 默认开启依赖 S4），推荐其前置；
3. 内核类条目只有在其阶段占比 ≥ 5% 时才推荐，否则明确说"内核类条目当前都不值得做"；
4. 给出该条目的预期 p50 收益区间 = 阶段占比 × 预期内核加速比对应的节省，并与 3% 噪声线比较；
5. 回复中列出前三名及各自一句话理由，第一名即为建议的下一次 `/sve-opt-land` 参数。

## 记录文档模板

```markdown
# <编号> <标题>

- 日期：YYYY-MM-DD
- 清单条目：<编号>，优先级 <高/中/低>，数值影响 <位精确/ulp 级/模型变更>
- 分支 / commit：<branch> @ <sha>；代码 <已合入，默认开启 | 已合入，默认关闭（开关 <名>） | 未合入，见 <编号>-attachments/<...>.patch（对 <sha> 可 git apply）>
- 对照基线：<同一 wheel 的 <开关关闭> 路径 | commit ...>，构建配置 <默认 / .cargo/sve.toml>，结果文件 <...>
- 候选：<同一 wheel，<环境变量>=1 | 构建配置 ...>，结果文件 <...>；<交替 | 顺序> 各三轮
- toolchain：rustc <版本>，cc <版本>，`lance.simd_info()` = <tier, VL>；CPU MIDR <...>

## 改动（<已合入，默认开启 | 已合入，默认关闭 | 已测量、未合入，见 patch>）

- <file:line>：<做了什么>；循环体指令数 <前 → 后>
- 分派 / 门控：<SimdSupport::Sve | kernel_support="sve" | FIXED_WIDTH_KERNELS_AVAILABLE | feature="nightly"> + 开关 <环境变量名>
- 单元测试：<测试名>，覆盖 <尾部长度 / 回绕输入>
- 附件：<微基准源码等>

## 数值等价

- 类别：<位精确/ulp 级/模型变更>
- 方法：<dump/diff 公共基线索引 | index-hash>
- 结果：<bit_exact true | top-10 集合变化 n/1000，最大距离偏差 x | combined_sha256 一致/不一致>
- 文件：<编号>-equivalence-*.json

## 内核微基准

| 规模 | 现有实现 | 本条目 | 比值 |
|---|---|---|---|

（必要时附逐指令发射率或变体拆解）

## 基准对比

<粘贴 compare 输出的三张表>

## 结论

- 主指标：<pca_256_plain 达标点 Δp50 / Δp95；baseline 达标点 Δp50 / Δp95 | 阶段 Δ>
- 判定：<有效 / 无显著变化 / 失败>
- 根因：<一句话>
- 处置：<代码合入并默认开启 | 代码合入默认关闭，开关 <名>，宽 SVE 硬件复测步骤 ... | 代码未合入，patch 保存于 ...>
- 回填文本：<与清单一致>

## 落地后剖析

- 默认路径：`<编号>-attachments/profile.md`，p50 <x> ms，样本 <n>；对比上一次 `<上一编号>-attachments/profile.json`
- 本条目对应阶段：<阶段名> <前 x.x%> → <后 y.y%>
- 其他变化超过 1 pt 的阶段：<...>
- （开关开启剖析，若适用）：`profile-switched.md`，<阶段变化>
- 下一条推荐：<编号>（<阶段> x.x%，<一句话理由>）；备选 <编号>、<编号>

## 新发现

- <新编号>：<一句话>；<被调整的编号>：<调整内容>（或"无"）

## 遗留

- <未解决的问题、后续建议>
```
