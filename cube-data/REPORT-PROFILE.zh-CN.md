# 新 950：c100n100 最优配置下的时延瓶颈抓取与分析

日期：2026-11-02（Asia/Shanghai），采集 18:03–19:50。**状态：阶段一完成（本报告）；不含 F2 的最优栈 BEST2 的复测与同口径抓取进行中，结果补入后更新。**
逐轮完整表（profiler 格式：逐轮 `均值 / P95 [有效/总实例]`）：[PROFILE-TABLES.zh-CN.md](PROFILE-TABLES.zh-CN.md)。工具：[scripts/](scripts/) 下 `stage_prep.py`（复用 amd231 profiler 的 `startup_analyze.py`）、`critical_path.py`、`profile_report.py`，OS 探针在 `../host950-numafix-revert-20261102/scripts/os_*.bt`。

## 0. 结论先行

1. **auto-prewrite 是当前唯一明确有效的优化**：同一栈（BEST）开/关 ABBA 各 10 轮，c100 整轮中位 157.1 → 139.5 ms（−11%），c1n20 各 6 轮 838–907 → 684–756 ms。它把 guest 恢复后段砍掉近一半（ResetVm 11.6 → 4.9、探针 16.4 → 7.0、Shim 返回→Master 返回 22.0 → 12.2 ms）。
2. **开启 prewrite 后，瓶颈从 guest 侧转向“前门 + 宿主准备 + VM 恢复”**。按中间 50% 沙箱：前门到达偏移 ≈24 ms、Cubelet→快照解析 ≈9–12 ms、解析→shim ≈17–24 ms、VM 恢复→agent ≈21 ms（不含 F2 时）、探针及收尾 ≈12 ms。
3. **决定整轮时间的最慢 10% 沙箱**，与中位相比主要多在：Cubelet 前期（XFS reflink 源锁与 cgroup，最多 +17.7 ms）与 VM 恢复（F2 引入的 TAP 懒打开，+10 ms）。
4. **此前选出的 “BEST” 栈误含 F2（空闲 TAP 不留 fd）**：每次 TAP 交付要 `TUNSETIFF`+`configureTapFD`+ethtool，open_tap p50 14.0 / p90 23.4 ms（不含 F2 时直接 dup 约 1.4 ms），VMM 时间线在 `tap.rs:185 Fast request tap` 处整体后移 +15 ms，RestoreVm 13 → 26 ms；BEST 的 Cubelet 侧改善因此被抵消，与 C 打平（137.6 vs 142.4 ms）。**去掉 F2 的 BEST2 正在复测**。
5. OS 级已确认的阻塞点：XFS reflink 模板 rootfs 时对源 inode 独占加锁（c100 一轮 89 次等待、合计 632 ms、平均 7 ms）；cgroup v1 `cgroup.procs` 写入在持有 `cgroup_mutex` 时做 `synchronize_rcu`（单次 25–27 ms，其余 shim 排队 54×0.35 ms，fork 也被同一全局锁拖住）；rtnl 锁自旋（Cubelet 内核态 on-CPU 56%，主要来自 F2 懒打开与网卡统计 dump）；containerd 元数据 bbolt 写锁与 `shim start` 子进程（各 ≈6.5 ms/实例）。

## 1. 条件与身份

| 项 | 值 |
|---|---|
| 主机 | ARM64，2 路、384 逻辑 CPU、4 NUMA；本次 18:03 前已重启，boot `7110ed0b…` |
| 内核 | `6.6.0-sp3-tlbi3-1027`（SP3 `1cefaec0` → 前置 `99aa8241` → TLBi `d494bb1f`），`tlbi_ipi=Y`、`flags=31`，THP madvise，`numa_balancing=0`，`rcu_expedited=0` |
| 组件 | A：C = BoostKit `86f6d9b9` + `dd5baee8`（auto-prewrite）；B/C：BEST = `d095826a` + PR36 + F1（cgroup hierarchy 缓存）+ PR37 + F2（空闲 TAP 不留 fd）+ auto-prewrite；.140 同 builder 构建 |
| 开关 | prewrite：`cube-shim/conf/prewrite=auto` 开 / 删除即关；dirty_log 关；TAP 池 500 |
| 模板 / 负载 | `tpl-33aa75680ae14683b32f05fe`（isnotfc，2C/2G）；`cube-bench -c 100 -n 100 -w 3 -m create-only`，bash 启动 |
| 生效证据 | 每轮回读运行 shim 的 exe SHA 与 VMM 版本行；prewrite 开时每轮 103/103 VMM 打印 `prewrite: 1940 pages in 1200 ranges (0 failed)`（3.3 ms p50），关时 0 行 |
| 样本 | A：CAP 5 轮 + 选择块 SEL-C 10 轮；B：FIN 10 轮 + SEL-BEST 10 轮；C：OFF 10 轮。诊断轮（bpftrace/Go trace/perf）单独执行，不计入计时 |

整轮中位（ms）：A 139.1（CAP）/ 142.4（SEL-C）；B 139.5（FIN）/ 137.6（SEL-BEST）；C 157.1（OFF）。同日上午重启前同配置 C 为 126.6 ms，重启后偏慢（见 §5 前门部分），不同时段数值不拼接。

## 2. 关键路径：最后完成者在等什么

去掉每轮 3 个串行预热（按 cube-bench 固定顺序推断，无客户端 SID 映射），以突发首个到达 Master 为 0；“完成”= Master 返回时刻。均值 ms，“中间”= 第 25–75 百分位，“最慢”= 最后 10%。

| 区间 | A 中间 / 最慢 | B 中间 / 最慢 | C 中间 / 最慢 |
|---|---|---|---|
| 到达 Master 的偏移（前门） | 23.7 / 26.6 | 24.7 / 26.8 | 23.7 / 24.6 |
| Master → Cubelet | 8.4 / 13.7 | 7.1 / 8.9 | 5.1 / 6.3 |
| Cubelet → 快照路径解析 | **11.6 / 29.3** | 8.7 / 12.3 | 10.2 / 15.1 |
| 解析 → Shim 收到 Create | **24.4 / 21.8** | 16.8 / 17.8 | 19.0 / 21.3 |
| Shim → start_vm | 0.7 / 1.3 | 0.8 / 0.8 | 0.8 / 1.1 |
| start_vm → agent ready | 21.0 / 21.5 | **34.0 / 44.0** | **34.7 / 47.3** |
| agent ready → Shim 返回 | 4.0 / 4.6 | 3.9 / 4.5 | 7.7 / 10.8 |
| Shim 返回 → Master 返回 | 12.2 / 14.9 | 12.2 / 16.8 | **22.0 / 21.8** |
| **完成时刻** | 106.1 / 133.7 | 108.4 / 131.9 | 123.2 / 148.4 |
| 组件：RestoreVm | 13.2 / 12.7 | 26.1 / 34.6 | 20.2 / 28.8 |
| 组件：ResetVm | 5.0 / 5.9 | 4.9 / 6.5 | 11.6 / 15.5 |
| 组件：sandbox-probe | 7.2 / 7.3 | 7.0 / 7.9 | 16.4 / 17.1 |
| 组件：cgroup（>5 ms 才记，条数） | 9.7 / 16.1（66/39） | 6.7 / 6.9（13/4） | 7.2 / 7.7（21/5） |

- A（不含 F2）的尾部来自 Cubelet 前期：最慢 10% 比中间多 +17.7 ms，cgroup +6.5 ms、XFS reflink 等待（§4.1）。
- B/C 含 F2：VM 恢复多出的 ≈14 ms 全部来自 TAP 懒打开（VMM 时间线 `tap.rs:185` 中位 7 → 22 ms，其后所有步骤同幅后移）；BEST 的 F1/PR36/PR37 使 Cubelet 前期尾部明显收敛（29.3 → 12.3 ms、慢 cgroup 66 → 13 条）。

## 3. 关闭 prewrite 后瓶颈分布的变化（同一 BEST 栈，B vs C）

| 部分（中间 50%，ms） | 开（B） | 关（C） | 变化 |
|---|---|---|---|
| 前门到达偏移 | 24.7 | 23.7 | 不变 |
| 宿主准备（Master→Cubelet + Cubelet→解析 + 解析→Shim） | 32.6 | 34.3 | 基本不变 |
| VM 恢复（start_vm→agent ready） | 34.0 | 34.7 | RestoreVm +5.9（预写 3.3 ms 等），ResetVm −6.7，相互抵消 |
| guest 后段（agent→Shim 返回 + Shim 返回→Master 返回） | 16.1 | 29.7 | **−13.6：探针 16.4→7.0、CreateContainer 等** |
| 完成时刻 | 108.4 | 123.2 | −14.8 |

- 关闭时，guest 恢复后的首批写 CoW 拖慢 agent 重连（ResetVm 11.6 ms）、探针（16.4 ms）与 CreateContainer，guest 相关段约占中位完成时间的 52%；开启后降到约 46%，**宿主准备与前门的占比上升，成为下一步主要对象**。
- 最慢 10% 在两种状态下都主要多在 VM 恢复（含 F2 惩罚）与 Cubelet 前期；prewrite 不改变这两个尾部来源。

## 4. 细化后的原因（OS / 运行时证据）

### 4.1 XFS reflink 模板 rootfs：源 inode 独占锁串行化

- 探针确认 reflink 源文件为模板 rootfs（`tpl-tpl-33aa…-rootfs`，每轮 103 次）。6.6 内核 `xfs_iolock_two_inodes_and_break_layout` 对源 inode 取独占 `inode_lock`，同一源的并发 FICLONE 只能串行。
- Cubelet D 态 `xfs_reflink_remap_prep → … → down_write` 在每个诊断轮都出现，量级随轮次波动：

  | 条件（单个 c100 诊断轮） | 等待次数 | 合计 | 平均 |
  |---|---:|---:|---:|
  | A：C + prewrite 开 | 89 | 632 ms | ≈7 ms |
  | B：BEST + prewrite 开 | 90 | 280 ms | 2.5–3.6 ms |
  | C：BEST + prewrite 关 | 94 | 429 ms | 4.4–4.9 ms |
  | 对照：G2（v0.7.1，无 prewrite，当日 12:30 重启前） | 88 | 215 ms | 2.4–2.9 ms |

  另一个探针轮（C 条件）测得 `xfs_reflink_remap_prep` 本身平均 0.83 ms、最大 3.7 ms，说明排队深度取决于同一时刻到达克隆步骤的创建数。A 中 Cubelet `ioctl`（FICLONE）109 次、平均 6.1 ms。
- Go trace：`cubecow_create_snapshot_from_volume` 系统调用阻塞约 6.5 ms/实例（B），5.1 ms/实例（C）。
- 方向：每次创建不克隆同一源（预克隆的私有卷池、多份源副本轮转），或评估回合上游 v6.7 `14a537983b22`（源侧锁调整）。

### 4.2 cgroup v1 迁移：`synchronize_rcu` + `cgroup_mutex` 车队

- shim 早加入 pool cgroup 写 `cgroup.procs` 时，`percpu_down_write(cgroup_threadgroup_rwsem) → rcu_sync_enter → synchronize_rcu`，单次 25–27 ms，**期间持有 `cgroup_mutex`**；其余 shim 在 `cgroup_kn_lock_live` 排队（B：54 次、平均 345 µs），shim 内创建线程的 `cgroup_can_fork → percpu_down_read` 也等待（305 次、平均 45 µs）。
- Cubelet 读 cgroup 统计（`cgroup1.(*cgroup).Stat`）与容器启动的 open/close 在 Go trace 中单次可达 ~45 ms（A）；cgroup rstat flush 本身不大（C：Cubelet 10 次合计 14 ms），F1 后慢 cgroup 步骤显著减少（§2）。
- 方向：同一沙箱只迁移一次（shim 早加入与 Cubelet 晚 AddProc 二选一）；挂载带 `favordynmods` 的 v1 hierarchy 去掉写者的 RCU 等待（运行时可验证，待测）。

### 4.3 rtnl 锁竞争

- C 的 Cubelet 内核态 on-CPU 采样（999 Hz）中 **56% 是 `osq_lock < mutex_lock < rtnl_lock` 自旋**，另有 `rtnl_dump_ifinfo → snmp6_fill_stats`（全量网卡 dump，逐网卡汇总 IPv6 per-CPU 统计）与 `close_fd`/`alloc_fd` 的 fd 表自旋锁。
- Go trace：F2 懒打开路径 `openTapFdByName/configureTapFD` 的 ioctl 阻塞合计 B 2.08 s（≈20 ms/实例）、C 1.05 s；`enableTXTCPMangleIDSegmentation → setFeature`（ethtool）0.19–0.26 s。这些 ioctl 都要拿 rtnl。
- 方向：去掉 F2（BEST2）；网卡 dump 不放在突发窗口内（排查周期性 LinkList 的来源与频率）；ethtool 特性只在建 TAP 时设置一次。

### 4.4 解析 → Shim 收到 Create（≈17–24 ms）

- `containerd-shim-cube-rs start` 子进程：containerd `binary.Start` 等待其退出，≈6.5 ms/实例（B）。该进程只为再 `spawn` 一个 shim 守护进程并写地址文件，两次 exec 15 MB 二进制，且受 Go ForkLock 串行化。
- containerd 元数据 bbolt：`metadata.(*DB).Update` 同步等待 ≈6.7 ms/实例（B）、10.3 ms/实例（C），即单写者锁 + fsync 的排队；Cubelet 目录 fsync 另有 ~0.3 ms/次。
- 方向：预拉起 shim 守护进程池或由 Cubelet 直接拉起守护进程（省掉 start 子进程）；减少每次创建的 bbolt 事务数或合并提交。

### 4.5 前门：100 个请求到达 Master 的时间分散 29–44 ms

- 到达偏移中位约 24 ms，是最慢沙箱完成时间里最大的单段之一；重启前上午为 15–35 ms（中间 14.3 ms），与当日 126.6 → 139 ms 的差异一致。
- 本次未部署 Client→API→Master 的入口观测（profiler-frontdoor 补丁），无法再细分；CubeAPI 软 NOFILE 1024 只在 c1000 失败，与 c100 无关。

### 4.6 其他

- VM 恢复（不含 F2，A）：RestoreVm 13 ms（prewrite 3.3 ms），ResetVm 5 ms；KVM `kvm_set_irq_routing` 的 `synchronize_srcu_expedited` 约 1,400 次 × 16 µs/轮（每 VM ≈0.2 ms）。PVM 三补丁只覆盖 ioeventfd 注册部分，实测无端到端收益。
- **shim start 偶发 panic**：tokio `OS can't spawn worker thread: Resource temporarily unavailable (os error 11)`，今日 2 次（11:31 c400 被 Master 重试吸收；19:11 重度跟踪轮 3 个请求失败）。任务数、进程数、内存提交、fd 限制均已排除，根因未定。

## 5. 下一步优化方向（按预期收益排序）

| 优先级 | 方向 | 依据 | 预期 |
|---|---|---|---|
| 1 | 去掉 F2，确认 BEST2（PR36 + F1 + PR37 + prewrite）为新基线 | §2/§4.3：F2 使每实例 +14–20 ms；BEST 的 Cubelet 侧改善已可见 | 中位完成时间预计回到 A 的 VM 恢复水平，同时保留 Cubelet 尾部收敛（待 BEST2 复测） |
| 2 | 前门：部署 Client→API→Master 入口观测，查清 29–44 ms 到达分散 | §4.5 | 最慢沙箱 ≈24 ms 的上界空间 |
| 3 | 模板 rootfs 不在创建路径上并发克隆同一源（预克隆池 / 多源副本 / XFS 回合） | §4.1 | 尾部 5–15 ms，并随并发放大 |
| 4 | shim 拉起：省掉 `shim start` 子进程或预拉起；减少 bbolt 事务 | §4.4 | 每实例 ≈6–13 ms |
| 5 | cgroup 只迁移一次 + favordynmods | §4.2 | 去掉 25 ms 级单点停顿与车队 |
| 6 | 探针改为 agent 就绪信号；rtnl：突发期间不 dump 网卡、ethtool 只设一次 | §2、§4.3 | 每实例 ≈5–7 ms |

## 6. 限制与待补

- BEST2（不含 F2）的选择块与同口径抓取（含 probe2/probe3：rstat、reflink、on-CPU、`shim start` 生命周期、fsync）正在执行，结果补入后更新 §2–§5。
- prewrite 开的二级探针（probe2）首轮因 bpftrace 映射键类型错误未加载，开/关对照的 probe2 只有关闭侧；B 的 OS 阻塞与 Go trace 完整。
- 关键路径以“去掉最早 3 个到达”近似排除预热，属基于 cube-bench 固定顺序的推断。诊断轮受跟踪开销影响，只用于定位，不作计时样本。
- 本报告数值均为本主机、本内核、TAP 500、2C/2G 模板条件；不同时段（重启前后）不拼接计算收益。
