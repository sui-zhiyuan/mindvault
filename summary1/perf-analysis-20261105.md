# CubeSandbox：950 与 9654 perf 优化候选分析

## 结论

优先验证两类 ARM SIMD 候选：**快照 JSON 解码**、**pagemap/soft-dirty 位图批量扫描**。前者有采样与源码证据，后者主要是源码候选，尚未证实占据本次测试的主要开销。当前数据不能证明存在已经确认的、可通过 SIMD 消除的 ARM 主瓶颈，也不能给出可靠的加速比例。

950 的主要性能问题在 resume，create 次之；pause 反而比 9654 快。因此，pause 中 ARM 占比较高的函数不能直接解释 resume 的回退。优化应先降低数据解析、分配和系统调用量，再验证向量化的增益。

## 数据范围与解释边界

原始数据均位于 192.168.45.63：

- 9654：`/home/sl/cubesandbox-perf-9654/20261105-142041-855367`
- 950：`/home/sl/cubesandbox-perf-950/20261105-142506-200355`
- 分析中间文件：`/home/sl/cubesandbox-perf-analysis-20261105`

测试为 create c100n100；pause/resume c10n5。采集使用 system-wide `instructions` 事件、99Hz、DWARF 调用栈；各组未报告丢失采样。下文百分比是 **instructions 采样按 period 加权的全机占比，不是函数耗时占比，也不是 CubeSandbox 进程内部占比**。没有 cycles 调用栈、cache/stall 和 off-CPU 数据，不能据此判断谁消耗了更多等待时间。

定量比较以原始 `report.txt`、`report-cubesandbox-userspace.txt` 为准。显示阈值会截断小项；标记“至少”的合计只包括已显示项。少数热点来自 1～3 个高权重样本，稳定性很弱。跨架构重新生成的报表受 ARM 二进制缺失影响，`comparison.csv`、`flat-symbolized.csv` 等中间文件仅供排查，不作为完整排行榜。

### 实验延迟

来自 `timings.csv`，单位 ms；是本次采集期间记录的操作延迟，不是无 perf 干扰的基线。

| 操作 | 9654 | 950 | 解读 |
|---|---:|---:|---|
| create，1 次 | 164.32 | 214.15 | 950 约为 1.30 倍；无重复实验置信区间 |
| pause，5 次均值 | 204.16 | 86.52 | 950 更快 |
| resume，5 次均值 | 192.77 | 423.41 | 950 约为 2.20 倍 |
| resume，双方均去掉第 1 次 | 196.54 | 305.28 | 950 仍约为 1.55 倍；只用于观察首轮影响 |

950 resume 第一次为 895.94ms，其余为 264.56～335.61ms；需单独解释首轮异常。冷热缓存、缺页和调度只是待验证原因，不能直接认定。

## 一、优先验证的 SIMD 优化点

| 编号 | 候选与证据 | 优化方向 | 优先级与限制 |
|---|---|---|---|
| S1 | 快照 JSON 整数解析。create 的 `serde_json::de::Deserializer::parse_integer`，950 已显示项合计至少 0.96%，9654 至少 0.57%。950 其中 0.73% 来自仅 1 个样本。 | 对整个输入块做 NEON 结构字符、字符串转义、UTF-8 扫描；对连续数值数组批量解析。进一步考虑兼容版本的二进制快照格式。 | SIMD 第一候选，但绝非已确认主瓶颈。`SeqAccess::next_element` 已显示项反而为 950 至少 0.46%、9654 至少 0.82%，不能说整个 JSON 路径在 ARM 都更差。 |
| S2 | PCI 配置快照包含 `registers`、`writable_bits` 两个各 1024 项的 u32 数组；恢复经过 JSON 解码。 | 批量处理数组；避免逐个短整数解析。二进制保存两块数组可能比单值 SIMD 十进制解析更有效。 | 有具体数据结构依据；缺少该字段独立耗时。需要确认数组长度、端序和旧快照兼容性。 |
| S3 | `get_anon_pages` 对 pagemap 每个 64 位条目提取 present/swapped/PFN，再查询 kpageflags。 | NEON 每批处理两个 64 位条目，批量掩码、比较、生成有效项；SVE 仅在实际支持且运行时检测后考虑。 | 源码候选；本次栈没有直接证实该函数为热点。逐页 kpageflags seek/read 可能先于算术成为瓶颈。 |
| S4 | `get_soft_dirty_pages` 提取 bit55/63/62；anon 与 soft-dirty 路径重复读 pagemap，并构造多个布尔数组。 | 一次读取后融合位提取、逻辑与、范围生成；复用缓冲区，再对连续输入使用 NEON。 | 先消除重复读取/分配，再量化 SIMD。首次快照可能走 anon 路径，需确认 soft-dirty 是否已 armed，不能假定本实验一定覆盖增量路径。 |
| S5 | `coalesce_pages_to_ranges` 标量扫描布尔状态并合并连续页。 | 使用块掩码快速跳过全零/全一块；位扫描确定边界，与上游过滤融合，避免中间 Vec。 | 低置信源码候选；分支和范围输出可能限制向量化收益。不能硬编码 4KB 页大小。 |
| S6 | ARM pause 的 `__arch_copy_from_user` 已显示项至少 2.26%；create 内核 `memset` 0.74%，后者仅 1 个样本。 | 优先减少复制量、次数和中间缓冲区；确认具体 libc/内核实现后才评估自定义向量化。 | 弱候选。内核复制往往已有架构优化，不能直接把它改成用户态 NEON；x86 与 ARM 符号不同，不能直接计算同函数差距。 |

不建议直接套用 SIMD 的地方：短小的单个整数、带锁树操作、seccomp 安装、文件系统元数据更新、缺页与调度。它们主要需要减少工作量或等待。

## 二、950 较突出及其他可优化路径

| 编号 | 候选与证据 | 可实施的优化方向 | 证据强弱与验证要求 |
|---|---|---|---|
| O1 | **嵌套 JSON 的重复编码/解码**：`SnapshotDataSection.snapshot` 是 String；内部 `to_string`，外层再序列化，恢复时外层解析后再 `from_str`。 | 避免嵌套转义与二次解析；缓存模板中不可变的已解析元数据；或引入有版本的二进制 payload。 | 结构确定、JSON 栈存在；属于 S1/S2 的更高层替代方案，不应重复计算收益。每个 VM 的可变状态必须隔离。 |
| O2 | **分配与临时对象**：950 create 的 malloc 已显示项至少 2.35%；JSON/Vec/String 路径存在多次分配与复制。 | 预估容量、复用解码缓冲区、减少 clone/to_vec、减少临时快照对象；对高频固定结构复用存储。 | 中等候选；不同 libc 和符号别名阻止严格比较。先统计分配次数与字节数，不宜直接更换分配器。 |
| O3 | **vmalloc 池回收**：950 create `decay_va_pool_node` 1.75%，仅 3 个样本；9654 对应解析结果未见采样，不等于真实为零。 | 查清触发它的 vmap/vmalloc、线程栈或其他内核映射生命周期；减少高频映射销毁，复用符合隔离要求的资源。 | ARM 相对突出但很不稳定。该函数处理池回收、合并与锁，不是 SIMD 目标；需调用者和映射次数佐证。 |
| O4 | **pagemap 扫描与小系统调用**：950 pause `pagemap_pte_hole` 至少 1.92%；9654 跨 comm 合计约 1.30%、19 个样本，不能说 x86 不存在。源码有逐 PFN seek/read。 | 合并 anon/dirty 的 pagemap 读取；先过滤不需要保存的页，再查询 PFN；对连续 PFN 有界批读/缓存 kpageflags；流式合并范围。 | 源码机会较具体，采样较弱；950 pause 总体更快，不能作为其 resume 慢的结论。保留 swapped、PFN 隐藏/权限不足等语义。 |
| O5 | **快照碎片写入与 XFS 元数据**：950 pause `xfs_rmapbt_init_high_key_from_rec` 至少 0.74%；9654 聚合约 0.29%、11 个样本。ARM 为少量样本。 | 合并相邻输出范围，减少 seek/小 write；评估批量写入、reflink/CoW 元数据更新次数和存储布局。 | 疑似。不同内核、文件系统与存储后端可能是原因；需 syscall 计数、写入大小分布和块 I/O 延迟。不是 SIMD 候选。 |
| O6 | **resume 缺页、page cache、NUMA 与首触**：ARM 调用栈出现页分配、filemap、fault 等路径，且首轮 resume 明显异常。 | 对比缺页数量、文件缓存冷热、NUMA 本地性和映射建立开销；对需要的范围评估 prefault，检查 THP/映射配置。 | 与较慢操作相关，但尚无高置信的单函数归因。不要盲目预触整个客体内存或开启大页，可能增加内存和破坏 CoW 收益。 |
| O7 | **线程创建与 seccomp 重复工作**：两架构均有 seccomp 热点；950 create `seccomp_set_mode_filter` 1.27%，9654 `seccomp_cache_prepare_bitmap...` 2.14%。 | 缓存同架构/同线程角色的用户态 BPF 构建结果；减少不必要的线程创建；可安全复用时使用有界线程池。 | 通用候选，不是已证明的 ARM 独有瓶颈；两个函数处于同一路径不同层级，不能直接相减。缓存用户态构建不会消除内核逐线程校验，不应关闭 seccomp。 |
| O8 | **日志格式化和锁**：快照/启动路径有格式化、Unicode 和日志调用栈；启动日志输出完整 VM config。 | 用简短关键字段替代完整 Debug；在格式化前判断级别；减少重复字符串构造，批量写日志。 | 可优化，但不是 ARM 更慢的证据。`Logger.enabled=true` 不代表全局过滤器允许所有日志；x86 Unicode 热点还更高。 |
| O9 | **Go 调度/GC 与过多并行度**：9654 已还原 `runtime.stealWork`，master pause 4.50%、create 1.69%、resume 2.22%；950 对应符号尚未还原。 | 补齐 ARM 符号后，对照实际 GOMAXPROCS、CPU quota、线程数及 GC 分配量，评估并行度和对象复用。 | 目前更像两端共同排查项，不能称作 ARM 瓶颈；不能凭机器 384 逻辑 CPU 就断言服务实际 GOMAXPROCS=384。 |

### 函数差异应如何排序

目前较明确“950 指令采样占比更突出”的具名候选为 `parse_integer`、`decay_va_pool_node`、`pagemap_pte_hole`、`xfs_rmapbt_init_high_key_from_rec`；全部受到显示阈值、低样本数或调用归属限制。`__arch_copy_from_user` 是 ARM 路径疑点，但跨架构符号不同，比较更弱。

**没有发现能够可靠解释 950 resume 延迟差距的、已完成符号解析的单一高占比函数。** resume 应优先补齐符号并采集 cycles/off-CPU/缺页证据；不能将 create/pause 的热点移植为 resume 的结论。

## 三、源码定位与版本风险

源码仓库：`/home/sl/CubeSandbox_v0.7.1`，当前源码 HEAD 为 `d095826ad6f3d9c56c6a2f04e5d104eb83b802f7`。本次采样实际 Shim 路径为 `/data/cubelet/root/component_versions/cube-shim/v0.7.1/bin/containerd-shim-cube-rs`，不能用当前源码 HEAD 代替运行版本。

9654 实际 Shim 已确认版本提交为 `31d911e430fdf8a8879bd062b8e78066c1a8e89d`，构建时间 2026-09-11；源码分析同时检查了该提交。950 对应二进制未随数据上传，且本次无法 SSH 连接 192.168.25.90，**950 Shim 的实际提交尚未核实**。

| 路径（仓库内） | 相关位置 |
|---|---|
| `hypervisor/vm-migration/src/lib.rs` | `SnapshotDataSection`、`to_state`、`new_from_state`：嵌套 JSON |
| `hypervisor/pci/src/configuration.rs` | `PciConfigurationState`、`state`、`set_state`、`restore`：两个 1024 项数组 |
| `hypervisor/vmm/src/migration.rs` | `recv_vm_config`、`recv_vm_state`：读取并解析快照 |
| `hypervisor/vmm/src/pagemap_anon.rs` | `get_anon_pages`、`coalesce_pages_to_ranges` |
| `hypervisor/vmm/src/soft_dirty.rs` | `get_soft_dirty_pages`、`filter_memory_ranges_by_anon_and_soft_dirty` |
| `hypervisor/vmm/src/memory_manager.rs` | `send_pagemap_anon_memory`、`save_range_to_file`、`restore_memory_regions_and_zones`、`create_ram_region` |
| `hypervisor/src/common.rs`；`hypervisor/vmm/src/vm.rs` | Logger 与完整 config 日志 |
| `hypervisor/virtio-devices/src/thread_helper.rs`、`seccomp_filters.rs` 等 | 线程角色对应的 filter 构建与安装 |

内核 `decay_va_pool_node` 的池回收/合并语义参考 [Linux vmalloc.c](https://linux.googlesource.com/linux/kernel/git/torvalds/linux/+/f1e50b1bcc2377f16574f891c8650def928647ee/mm/vmalloc.c)；远端内核版本有差异，具体实现仍应匹配各自内核源码。

## 四、不能作为优化结论的样本

- 9654 pause 中 `pmlogger::do_work` 14.39%、`pmaSortInDom` 1.57% 是全机背景负载，不是 CubeSandbox 函数；它也会改变全局百分比分母。
- ARM swapper 的 `finish_task_switch`、idle 占比不等于 CubeSandbox 的 CPU 计算开销，不宜直接改内核调度。
- ARM cubelet 的 `_start` 及 master 的裸十六进制地址是符号解析盲区，不能把 `_start` 解释为反复进程启动。
- x86 libc 的 `__strncasecmp_l_avx2_rtm` 出现在 Vec 扩展、字符串解析栈，可能是 stripped/IFUNC 符号别名归属问题；不能据此认定 ARM 缺少字符串 SIMD。
- `hypervisor/Cargo.toml` 的局部 release `opt-level="s"` 不足以证明实际 Shim 使用尺寸优化；需以 CubeShim workspace 生效配置为准。

## 五、下一轮验证顺序

1. 补齐 950 的采样原版 Shim、Go 服务、libc 和内核符号，核对 build-id；确认两端服务版本、编译选项和实际配置。
2. 将 resume 首轮与稳定轮分开，重复相同操作；保留 instructions，并加 cycles 调用栈、缺页/上下文切换计数及 off-CPU 证据。统计服务自身而非全部背景进程，明确指标分母。
3. 首先对真实快照输入测 JSON 解码时间、分配次数、数组尺寸，验证 S1/S2/O1/O2；同时统计 pagemap/kpageflags 的读取次数、字节量与范围碎片数，验证 S3～S5/O4/O5。
4. 在上述证据支持后，分别 A/B 测试“减少重复解析/读取”和“NEON 批处理”，报告端到端延迟、CPU cycles/操作、instructions/操作及内存；不要仅比较热点百分比下降。
5. resume 的 page cache/NUMA/prefault 与线程/seccomp 分开实验，避免把多种改动合并后无法归因。

本次仅进行了离线分析，未实施优化、未重跑基准，也未修改服务配置或删除远程文件。
