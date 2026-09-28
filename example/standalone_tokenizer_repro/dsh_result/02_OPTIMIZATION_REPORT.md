# 长文本 tokenizer（Qwen2.5-0.5B）优化报告

> 更新：2026-09-28
> 对象：`example/standalone_tokenizer_repro` 复现的长文本 tokenizer 负载（batch=1、58,834 字符 / 78,960 UTF-8 字节 / 25,600 tokens）
> 一句话结论：**能优化，而且不改变任何输出**。两条各自独立、可叠加的杠杆：①`BytesToCharOffsetConverter` 的 `HashMap<usize,usize>` → `Vec<u32>`（单文件 46 行）：aarch64 **−23.7%~−24.1%**、x86 **−18%~−21%**；②`lto="fat"` + `codegen-units=1`：aarch64 **−10.3%**。两者叠加后 aarch64 **−29.5%~−30.1%**（37.37 → 26.08 ms），`ids/offsets/tokens/type_ids/special_tokens_mask` 全程逐字节不变。

---

## 1. 摘要

| 项目 | 结果 |
|---|---|
| 原始现象 | 同一 fast tokenizer、同一输入，aarch64 比 x86_64 慢约 39%~40%（包内历史数据：27.66 ms vs 38.60 ms p50） |
| 本机复现 | `192.168.41.50`（HiSilicon 0xd02，SVE VL=256bit）stock 基线 **37.17~37.28 ms**，与历史 aarch64 一致 |
| **最优方案** | **方案 1：byte→char 偏移映射 `HashMap` → `Vec<u32>`**（上游 tokenizers 单文件 46 行） |
| **最优配置（实测）** | 方案 1 + `lto="fat"`/`codegen-units=1` + `target-cpu=native`：aarch64 **−29.5%~−30.1%**（37.37 → **26.08 ms**），输出逐字节不变；已优于包内 x86 stock 基线（27.66 ms） |
| 方案 1 收益 | aarch64 **−23.65% / −24.14% / −23.68%**（default/rayon1/rayon8，10 block 配对，CI 宽 ±0.75pp）<br>x86 **−20.8%**（原案例路径）/ **−18.0%**（raw `encode_batch`） |
| 输出影响 | **零**（两机各自逐字节比对：19 个用例含空串、组合字符、emoji、尾部 CJK、换行、pair 用例） |
| 已证伪的方向 | SVE 宽组（NEON8→NEON16/SVE16/SVE32）：两机端到端**都变慢** +2.2%~+5.1% |
| 无效的方向 | 线程/Rayon 调参、改 batch 形态、绕过 transformers 包壳（差异在 ±1% 噪声内） |
| 已实测的编译参数 | LTO+CGU=1 **−10.3%**；`target-cpu=native` **−0.6~−1.1%**；按 cpuinfo 补齐 feature 无额外收益；`-aarch64-sve-vector-bits-min=256` 改变代码但无收益且有可移植性风险 |
| 仍然待实测 | 换分配器（jemalloc/mimalloc，占 36~47%） |

---

## 2. 背景与基线

### 2.1 原始案例

- tokenizer：公开 Qwen2.5-0.5B（固定 revision `060db649…`），`transformers==4.48.3` + `tokenizers==0.21.4`
- 输入：batch=1，58,834 字符 / 78,960 UTF-8 字节 / 25,600 tokens（固定种子合成的中英数字 JSON 混合文本）
- 调用：`AutoTokenizer(...)([text], add_special_tokens=False, padding=False, return_attention_mask=False, return_token_type_ids=False, truncation=False)`，8-worker `ThreadPoolExecutor` 串行提交 + `asyncio.run_in_executor`
- 历史结果（包内）：x86_64 p50 **27.66 ms**（Python 3.11.2）/ aarch64 p50 **38.60 ms**（Python 3.11.9），差 **+39.6%**

### 2.2 本报告用到的机器

| | `192.168.41.50`（本次主用） | `192.168.41.51`（归档实验所在） |
|---|---|---|
| CPU | HiSilicon `0xd02`，2×80 核 SMT2 = 320 线程，4 NUMA 节点 | TaiShan-v120 / Kunpeng 920 7280Z |
| SVE | **最大 VL=256 位**，可用 {128,256}，SVE1（无 SVE2） | VL=32 字节 |
| OS / Python | openEuler 24.03 SP3 / 3.11.6 | AlmaLinux 9.4 / 3.12.14 |
| Rust | 私有工具链 **1.98.1 (`48a229cea`) / LLVM 22.1.8** | 1.98.1（同 commit） |
| stock 基线 p50 | 37.17 / 37.28 / 37.25 ms | 38.35 / 38.23 / 38.46 ms |

> 两台 ARM 机器的 stock 基线相差 ~2.6%，本案例的 aarch64 慢是**可重复**的，不是单机偶然。

---

## 3. 测量方法

1. **剖析**：`perf record -F 999 -g --call-graph dwarf`，固定 `RAYON_NUM_THREADS=1`，只统计"确实在编码"的样本（剔除 `tokenizer.json` 反序列化加载段）；`.so` 未 strip，Rust 符号可解析（无 `.debug_info`，内联层不可分辨）。
2. **执行模型已确认**：任意时刻只有 1 个 encode 在跑，工作**全部由调用线程完成**——batch=1 时 Rayon 池虽被惰性创建（默认 320 线程），但 `par_iter` 只有 1 个元素；证据是进程 CPU/wall ≈ **0.998**（若调用线程在自旋等 worker 会接近 2.0），且 `executor_thread_cpu_ms ≈ tokenizer_ms`、`queue_ms ≤ 0.06 ms`。
3. **微测**：沿用归档微基准，其 `offsets()` 函数**逐字复刻** `BytesToCharOffsetConverter::new` 的构造过程；10 轮 × 5 后端、轮转顺序、绑核 240 / NUMA3。
4. **端到端配对**：每后端独立进程、`numactl --physcpubind=240,...254 --membind=3`、每进程 3 轮 ×（10 预热 + 30 正式）= 90 次正式调用；10 block、顺序轮转并反转；统计单位是**进程 p50**，配对后取几何平均 + 10000 次 bootstrap（固定种子 `20260924`）。
5. **正确性**：每次调用校验完整 token IDs；另有 19 个用例的 `ids/offsets/tokens/type_ids/special_tokens_mask` 逐字节导出比对。

---

## 4. 成本结构（两台机器、两条独立剖析路径）

编码阶段 self 时间占比：

| 成本项 | .50（本次） | .51（归档） | 可优化性 |
|---|---:|---:|---|
| **分配器 + memcpy**（`_int_free`/`malloc`/`_int_malloc`/`realloc`/`malloc_consolidate`/`unlink_chunk`/`cfree`/`__memcpy_generic`） | **46.8%** | **40.7%** | 部分可（方案 1/3/6） |
| **hashbrown 簇**（`insert` + `reserve_rehash` + `hash_one`） | **14.2%** | **16.0%** | **可（方案 1）** |
| `BytesToCharOffsetConverter::convert` | 5.3% | 5.6% | **可（方案 1）** |
| Oniguruma 正则 `match_at` | 4.5%（inclusive 10.3%） | 5.8% | 不可（改了就换 tokenizer） |
| Unicode NFC / `unicode_normalization_alignments` | 4.5% | ~2.4% | 有限（方案 5） |
| `NormalizedString::slice`（93% 是分配器） | 8.0% | — | 方案 6 |
| `drop_in_place<Encoding>`（98% 是分配器） | 4.4% | — | 方案 6 |
| BPE 合并算法（inclusive） | 9.8% | 6.7% | 空间小 |

关键 inclusive 结构（.50）：`encode_single_sequence` 79.0% → `PreTokenizedString::into_encoding` **35.0%**（其中构建 `HashMap` **15.1%**、字节↔字符换算 6.2%）／`PreTokenizedString::split` 16.0%（其中 Oniguruma 10.3%、NFC 4.5%）／`PreTokenizedString::tokenize` 12.1%（其中 BPE 9.8%）。

> **要点**：这个负载的时间主要花在"**内存与簿记**"（分配 47% + 哈希 15% + 偏移换算 6%），而不是 BPE 算法本身（9.8%）。这些都是**标量、指针追逐**型代码，对单核 IPC 敏感——这正是服务器 ARM 容易吃亏的形态，也解释了为什么"用更宽的向量"帮不上忙。

---

## 5. 优化方案与结论

| # | 方案 | 依据 | 收益 | 改输出? | 结论 |
|---|---|---|---|---|---|
| **1** | **byte→char 映射 `HashMap`→`Vec<u32>`**（改 tokenizers 单文件 46 行） | 双机实测（§6）；组件级构造 **20.5×**、查询 **22.4×** | aarch64 **−23.7~24.1%**；x86 **−18~21%** | **无**（已证） | ✅ **采纳，建议上游化** |
| **2** | 应用侧改走无 offsets 路径 `encode_batch_fast`（内核 `OffsetType::None`） | 本机交错配对 22.063→17.897 ms | **−18.9%** | **offsets 全变 (0,0)** | ⚠️ 仅在调用方只需 token IDs 时可用；无需改库 |
| 3 | 换分配器（jemalloc / mimalloc / tcmalloc） | 分配器占 36~47% | 预估 −5~15% | 无 | 🔬 待实测；与方案 1 部分重叠 |
| **4a** | **`lto="fat"` + `codegen-units=1`**（Cargo profile，非 RUSTFLAGS） | ARM 实测 **−10.50/−10.23/−10.32%**；`.so` 61.9→43.7 MB、符号 20,253→8,857 | **−10.3%** | **无**（已证） | ✅ **采纳**（本轮最大意外收获） |
| **4b** | `-Ctarget-cpu=native` | LLVM 不认部件号 0xd02，只解析出 10 个 feature；实测 **−0.59/−1.11/−0.75%**（主要来自 LSE 原子指令 21→3160） | **~−1%** | 无 | ✅ 采纳（免费的小收益） |
| 4c | 按 cpuinfo 补齐 feature（i8mm/bf16/dotprod/rdm/rcpc/fcma/fhm/flagm/flagm2/sb/ssbs/dit/ecv） | 指令普查：**一条都没生成**；实测与 4b 无差异（噪声内） | ≈0 | 无 | ⚠️ 无害但无用（本负载无对应模式） |
| 4d | `-Cllvm-args=-aarch64-sve-vector-bits-min=256` | 确实改变代码形态（SVE 2207→3746、`ptrue` 161→412），但无可测收益，且假设 VL≥256 | ≈0 | 无（但改变可移植性） | ❌ **不建议**（VL=128 机器会出错） |
| 5 | 恒等 NFC 快路径 | NFC 相关 4.5% | ≤4.5% | **有风险**（仅当文本已 NFC 时等价） | ⚠️ 通用性差，暂缓 |
| 6 | 减少临时 `String`/`Vec`、复用缓冲 | `slice` 8.0%、`drop<Encoding>` 4.4%，其中 93~98% 是分配器 | 预估 −3~8% | 需谨慎 | ⚠️ 并入 1/3，不单独立项 |
| 7 | 替换 Oniguruma 正则 | inclusive 10.3% | 可能 −5~10% | **会改 token 边界** | ❌ 属于"换 tokenizer"，非本案例优化 |
| 8 | SVE 宽组（NEON8→NEON16/SVE16/SVE32） | **两机 E2E 实测**：.51 +2.4~5.1%、.50 +2.2~3.3%（9/9 格显著） | **负收益** | 无 | ❌ **已证伪** |
| 9 | 线程 / `RAYON_NUM_THREADS` 调参 | 1/8/默认 差异在噪声内；`queue_ms`≤0.06 ms；执行本就单线程 | ≈0 | 无 | ❌ 不做（固定 `RAYON=1` 只为降方差） |
| 10 | 改 batch 形态 / 绕过 transformers 包壳 | `__call__` 22.063 / raw `encode` 22.102 / raw `encode_batch` 21.888 ms | ±1%（噪声） | 无 | ❌ 不是瓶颈 |
| 11 | 多文档并行（batch>1 + rayon，或进程池） | batch=1 时并行池不参与（已证） | 多文档吞吐近核数倍 | 无 | ✅ 按场景采用（ARM 的正确用法） |
| 12 | 硬件层（频率/绑核/NUMA/关 SMT 竞争者） | governor=performance、绑核已做 | 均值≈0，方差↓ | 无 | ❌ 只影响方差 |

---

## 6. 方案 1 的完整证据链

### 6.1 改动内容

只改 `tokenizers/src/tokenizer/pre_tokenizer.rs`（46 行）：`BytesToCharOffsetConverter` 的 `map` 由 `HashMap<usize, usize>` 改为 `Vec<u32>`，`convert()` 的三个分支（两端命中 / 右端是文末 / 起点越界返回 `None`）与上游**逐分支等价**。补丁见 `patch/pre_tokenizer_vec.patch`（完整改动后文件见 `patch/pre_tokenizer.patched.rs`）。

为什么可行：这个映射**单调且稠密**（字节下标 0..len-1 每个位置恰好对应一个字符下标），哈希表在这里只买到常数因子更差的访存、一次扩容重哈希和 ~1.8 MB 的临时内存。

### 6.2 组件级量化

| 测量 | HashMap（stock） | Vec<u32>（补丁） | 加速 |
|---|---:|---:|---:|
| 构造（本机 x86，78,960 字节） | 1.735 ms（21.97 ns/byte） | 0.085 ms（1.07 ns/byte） | **20.5×** |
| 查询（25,600 tokens × 2 次） | 0.434 ms | 0.019 ms | **22.4×** |
| 合计 | 2.169 ms | 0.104 ms | **20.9×** |
| 内存 | `capacity=114,688` 槽 × 16 B ≈ **1.8 MB** | 78,960 × 4 B = **316 KB** | ~6× 更小 |
| 构造（.50 ARM，归档微测口径） | 55.55~61.75 ns/byte → **4.39~4.88 ms/call** | — | — |

### 6.3 x86 端到端（三方配对，排除编译器污染）

同一台机器、交错顺序、5 轮配对：

| 路径 | PyPI stock | 自编 stock | 自编**补丁** | 补丁 vs 自编 stock |
|---|---:|---:|---:|---:|
| 原案例（`transformers.__call__`） | 23.273 ms | 23.331 ms（+0.25%） | **18.473 ms** | **−20.8%** |
| raw `encode_batch` | 23.025 ms | 23.067 ms（+0.18%） | **18.909 ms** | **−18.0%** |

（"自编 stock vs PyPI stock = +0.2%" 证明 −20.8% 不是工具链差异造成的。）

### 6.4 aarch64 端到端（.50，10 block 配对）

两份 wheel 用**相同工具链（rustc 1.98.1 / LLVM 22.1.8）与相同编译选项**（`--sysroot=neon8`、`-Ctarget-cpu=generic -Ctarget-feature=+sve`），只差源码。

| 配置 | stock | 补丁版 | 变化 | 95% CI | 逐 block |
|---|---:|---:|---:|---|---|
| default | 37.166 ms | **28.403 ms** | **−23.65%** | [−24.38, −22.93] | 10/10 更快 |
| RAYON=1 | 37.279 ms | **28.236 ms** | **−24.14%** | [−24.81, −23.51] | 10/10 更快 |
| RAYON=8 | 37.245 ms | **28.184 ms** | **−23.68%** | [−24.59, −22.73] | 10/10 更快 |

ARM 收益（−24%）**大于** x86（−18~21%）：被消掉的不只是那张表，还有 78,960 次插入及其**扩容重分配**——ARM 上分配器代价更高，所以省得更多。

### 6.5 输出等价性（两机各自验证）

19 个用例（空串、单字符、"记录0：请观察圆形积木"、`he\u0301llo wo\u0308rld` 组合字符、`🙂🚀a`、`abc记录`、5000 长 ASCII、`a\n\nb\r\nc`，各 × `add_special_tokens` True/False，另加一个 pair 用例），导出 `ids / offsets / tokens / type_ids / special_tokens_mask`：

- x86：`cmp` 逐字节相同
- aarch64：`cmp` 逐字节相同（1,882,759 字节）

### 6.6 上游化前要注意的点

1. `Vec<u32>` 的字符下标上限 ~4.29e9（约 4 GiB 字符的文本）；上游若接受，建议用 `usize` 或加检查/断言（本案例 78,960 字节远低于上限）。
2. 每次调用仍会分配 316 KB（相对原来的 ~1.8 MB + 哈希已大幅下降）；进一步可做线程本地缓冲复用，但会引入状态，建议单独评估。
3. 需要上游的等价性测试与更广的 tokenizer 矩阵（不同 normalizer / pre-tokenizer / 空输入 / 极端 Unicode）。

### 6.7 编译参数实验（ARM 实测；源码固定为 stock，与补丁分离）

| 变体 | 编译参数 | SVE 指令 | LSE 原子指令 | 端到端 default / rayon1 / rayon8 |
|---|---|---:|---:|---|
| 基线 neon8 | `-Ctarget-cpu=generic -Ctarget-feature=+sve` | 2207 | 21 | — |
| v1-native | `-Ctarget-cpu=native` | 2207 | **3160** | −0.59% / −1.11% / −0.75% |
| v2-feat | v1 + 按 cpuinfo 补齐 feature | 2207 | 3160 | −1.24% / −1.10% / −0.83% |
| v3-featvl | v2 + `-Cllvm-args=-aarch64-sve-vector-bits-min=256` | **3746** | 3161 | −1.25% / −1.05% / −0.39% |
| v4-lto | v2 + `lto="fat"` + `codegen-units=1` | 1826 | 3100 | **−10.50% / −10.23% / −10.32%** |
| opt-feat | 方案 1 + v2 参数 | 2205 | 3160 | −24.40% / −23.09% / −24.07% |
| **opt-lto** | **方案 1 + v2 参数 + LTO/CGU=1** | 1824 | 3100 | **−30.11% / −29.53% / −29.68%** |

`rustc --print cfg -C target-cpu=native` 显示，`native` 在这台机器上只解析出 **10 个 feature**：`aes crc fp16 lse rand sha2 sha3 sm4 sve neon`（`generic` 只有 `neon`）——**LLVM 不认识部件号 0xd02**，所以 native 只拿到一个子集；而按 `lscpu`/`cpuinfo` 补齐的那些（`i8mm bf16 dotprod rdm rcpc fcma fhm flagm flagm2 sb ssbs dit ecv`，rustc 都认识）**没有生成任何指令**，因为它们对应的模式（矩阵/点积/crc/aes）在本负载里不存在。真正改变机器码的只有两件事：`native` 带来的 **LSE 原子指令**（21→3160）和 **LTO/CGU=1**（函数符号 20,253→8,857）。

四条结论：

1. `target-cpu=native` **不是"不生效"，而是只生效了一部分**：约 −1%，来自 LSE 原子指令。
2. 按 cpuinfo 补齐 feature：代码与收益都**没有变化**（噪声内）。要补齐的是"人以为需要"的东西，不是这台机器上跑这段代码需要的东西。
3. `-aarch64-sve-vector-bits-min=256` **确实改变代码形态但无可测收益**，还引入"必须 VL≥256"的隐含约束 → **不建议**。
4. **`lto="fat"` + `codegen-units=1` 是真正的大头：−10.3%**。归档实验里所有 wheel 都是默认 release（CGU=16、无 LTO），这 10% 一直没被拿走。叠加方案 1 后达到 **−30%**。

两个工程坑（复现用）：

- `-Clto` **不能**放进 `RUSTFLAGS`：它会作用到所有编译单元，与依赖的 `-C embed-bitcode=no` 冲突（实测报错 `options -C embed-bitcode=no and -C lto are incompatible`）。正确做法：写进被构建包的 `Cargo.toml` 的 `[profile.release]`。
- tokenizers 仓库**根目录没有 `Cargo.toml`**，profile 必须加在 `bindings/python/Cargo.toml`（maturin 构建的那个包）；在根目录新建 manifest 会让 `cargo metadata` 失败。

---

## 7. 收益叠加与"能否追平 x86"

- 方案 1 之后 aarch64 本案例：**37.2 ms → ~28.2 ms**；再叠加 LTO/CGU=1（实测 −10.3%）后 **→ ~26.1 ms（合计 −30%）**。
- 用 repro 包的报告口径（x86 stock 基线 27.66 ms）比较：**优化后的 aarch64 已经比它更快（26.1 vs 27.66 ms，−5.7%）**——报告里"+39~40%"的观测差距在本案例上不但消除、而且反超。
- 但 x86 打同样补丁也会变快（本机实测 −18~21%），两侧都优化后仍残留约 **+25~30%** 的差距。这部分来自**单核标量 IPC 的架构差**（分配器、哈希、正则、Unicode），软件优化只能改绝对量、不能消除该比例。
- 若目标是吞吐而非单文档延迟：用 batch>1 让 rayon 按 item 并行（batch=1 时并行池完全不参与），或多进程处理文档——这才是多核 ARM 的正确用法。

---

## 8. 复现方法

### 8.1 aarch64（.50）

```bash
# 前提：私有工具链 + 5 套定制 sysroot + stock wheel 与 venv（见 wide/scripts/）
# 补丁源码放在 tokenizers-opt（由 tokenizers-clean 复制并打 patch/pre_tokenizer_vec.patch）
cd ~/hb/wide
./scripts/build_opt_wheel.sh                                   # 用 neon8 sysroot 编补丁 wheel（~30 s）
~/hb/shared/venv-build/bin/python -m venv venv-opt
./venv-opt/bin/python -m pip install -r ~/hb/shared/artifacts/python-requirements.lock
./venv-opt/bin/python -m pip install --no-deps --force-reinstall artifacts/wheels-opt/*.whl
# 等价性
./venv-neon8/bin/python /tmp/dump.py > /tmp/dump_stock_arm.json
./venv-opt/bin/python   /tmp/dump.py > /tmp/dump_opt_arm.json && cmp /tmp/dump_stock_arm.json /tmp/dump_opt_arm.json
# 配对端到端（10 block × stock/opt）
python3 scripts/run_e2e_opt.py --rounds 10 --output results/retest-opt-e2e
```

### 8.2 x86（本机）

```bash
cp -a <tokenizers v0.21.4 源码> /tmp/tokenizers-opt && <打同一补丁>
cd /tmp/tokenizers-opt/bindings/python && maturin build --release --out /tmp/wheels-opt
# 三方差（PyPI stock / 自编 stock / 自编补丁）交错配对：见 bench_api_shapes.py、ab_stock_vs_patched.txt
```

---

## 9. 产物索引（本目录 `dsh_result/` 内）

| 路径 | 内容 |
|---|---|
| `01_OPTIMIZATION_POINTS.md` | **所有优化点汇总**（16 条，逐条机制/证据/收益/输出影响/结论） |
| `03_COST_STRUCTURE.md` | 成本结构：两台机器剖析对照 + inclusive 结构 + 归因推理 |
| `04_MEASUREMENTS.md` | 全部实测数字汇总（ARM / x86 / 微测 / 等价性 / 校验链） |
| `patch/pre_tokenizer_vec.patch` | 方案 P1 补丁（单文件 46 行） |
| `patch/pre_tokenizer.patched.rs` | 打完补丁的完整文件（对照用） |
| `scripts/local/mapbench_hashmap_vs_vec.rs` | 组件级微基准（HashMap vs Vec，`rustc -O` 可直接跑） |
| `scripts/local/bench_offsets_vs_fastpath.py` | 方案 P5（无 offsets 路径）基准 |
| `scripts/local/bench_api_shapes.py` | 方案 P12（API 形态）基准 |
| `scripts/local/bench_transformers_path.py` | transformers `__call__` vs raw API 对比 |
| `scripts/local/equivalence_check.py` | 19 用例输出等价性检查 |
| `scripts/remote/` | .50 上的构建/运行/统计脚本（`build_*`、`run_e2e_*`、`paired_stats.py`、`run_with_vl.py`、`equivalence_dump.py`、`Cargo.toml.bindings-python.with-lto`） |
| `measurements/arm_e2e/retest-opt-e2e/` | 方案 P1 的 10 block 配对原始 JSON + `manifest.json`（含确切命令） |
| `measurements/arm_e2e/retest-feat-e2e/` | 编译参数变体 v1-native / v2-feat / v3-featvl 的 10 block 配对原始 JSON |
| `measurements/arm_e2e/retest-best-e2e/` | opt-feat（P1+native）与 v4-lto（native+LTO）的 10 block 配对原始 JSON |
| `measurements/arm_e2e/retest-optlto-e2e/` | **最优配置 opt-lto（−30%）** 的 10 block 配对原始 JSON |
| `measurements/arm_e2e/equivalence/` | ARM 侧 stock / 补丁输出导出（逐字节相同） |
| `measurements/arm_sve_repro/` | SVE 宽组复现：`retest-e2e`（13,500 次调用）+ `retest-aa`（1,620 次） |
| `measurements/micro/` | 微测原始数据：`retest-{legacy,extended,insert}` + 空闲重跑 `retest-insert2` |
| `measurements/local_x86/` | x86 交错配对原始输出（`ab_*`）+ 等价性导出（`dump_*.json`） |
| `measurements/perf/` | .50 火焰图（SVG + 折叠栈）+ .51 归档 perf 报告（对照用） |
| `logs/` | 全部过程日志；逐文件说明见 `logs/INDEX.md` |

> 说明：归档方自己的 SVE 宽组报告仍在 `results/other_updates/wide_sve_results.md`，本目录只复制了它的 perf 归因报告（`measurements/perf/archive-51-report-demangled.txt`）用于成本结构对照。

## 10. 附录：SVE 宽组证伪数据（方案 8）

端到端相对 NEON8 的耗时变化（正 = 变慢）：

| 后端 | .51 default / rayon1 / rayon8 | .50 default / rayon1 / rayon8 |
|---|---|---|
| NEON16 | +1.46% / +2.27% / +0.65% | +0.09% / −0.39% / −0.29% |
| SVE16-fused | +2.54% / +3.81% / +2.53% | +2.51% / +2.16% / +2.80% |
| SVE32-fused | +4.92% / +5.10% / +4.20% | +3.30% / +2.35% / +2.91% |
| SVE32-fused-private | +4.03% / +3.43% / +2.40% | +2.67% / +2.27% / +2.69% |

微测（高占用率未命中）确实大幅变快（−17.8% ~ −64.8%，两机一致），但该负载的 hashbrown 时间主要在**建表**而非 miss 探测，因此收益无法外推。

一处**未完全复现**的细节：`grow_insert` / `byte_offset_collect` 两个微测用例中，SVE32 的符号在两台机器上相反（.50 慢 +5.1~5.6%、.51 快 −1.0~−2.9%，两侧 CI 都排除 0）。在 .50 空闲时重跑仍如此，说明这是**对硬件/缓存敏感的真实差异**（宽组改变了哈希表布局与访存模式），但不影响"两机 E2E 都是 SVE 更慢"这一主结论。
