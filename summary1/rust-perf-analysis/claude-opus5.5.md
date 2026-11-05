# CubeSandbox Rust/VMM 性能优化点：create / pause / resume（claude-opus5.5）

- 基线代码：`v0.7.1` @ `d095826a`（下文 file:line 均以此 HEAD 为准）
- 范围：`hypervisor/`（vmm、net_util、pci、virtio-devices、hypervisor crate）为主，CubeShim 生命周期为辅
- 方式：只读分析；未编译、未压测、未登录主机。所有“实测”数字都标注了来源，其余数字都标为估算。
- 主目标 aarch64（Kunpeng 950），x86_64 为次要目标

---

## 0. TL;DR

| ID | 阶段 | 改动（一句话） | 预期收益（p50） | 置信度 | 架构 |
|---|---|---|---|---|---|
| **R1** | create + resume（RestoreVm 内） | `net_util/src/tap.rs:143-157`：取 tap fd 时，用 `ppoll` 等待替换 “EAGAIN 后 sleep 1 ms” | RestoreVm −0.75~1.0 ms（lazy 后 9.0 ms 的 −8~11%；eager 16.0 ms 的 −5~6%）；E2E 约 −2% | **B**（sleep 是否发生已由日志实测，等级 A：380/400 次恰好 1 次 sleep） | 两种架构通用 |
| **R2** | create + resume（RestoreVm 内） | 把 lazy irqfd 从 legacy 扩展到 VMM 自己模拟的 virtio 设备的 MSI-X 向量（`vmm/src/interrupt.rs`、`pci/src/msix.rs:112-146`） | RestoreVm 约 −3 ms（−33% of 9.0 ms）；扣掉 create 窗口内首次触发时补注册的部分后，E2E 约 −1~2.5 ms | **B**（按 legacy 实测的单条 irqfd 成本外推） | 以 aarch64 为主；x86 的收益需另测 |
| **P1** | pause | 回移 master #1343（`d6d664e3`）：判定增量页时只用 pagemap bit 61，去掉“每个 present 页一次 `/proc/kpageflags` lseek+read” | 空闲沙箱约为 0；写过 256 MiB 时 −31 ms；写过 1 GiB 时 −79 ms（x86 PVM 实测）；同时修复一处丢数据竞态 | x86 为 **A**；Kunpeng 为 **B** | 两种架构通用 |
| **P2** | pause（以及 commit/snapshot） | `vmm/src/cpu.rs:648-663` `signal_thread`：每个 vCPU 固定 sleep 1 ms 才复查状态；改为“先信号所有 vCPU，再用 20→50→100 µs 退避等待” | 2 vCPU 约 −2.0 ms（c1 pause 117.9 ms 的 −1.7%） | **B** | 两种架构通用 |
| P3 | pause | `memory_manager.rs:2352-2387` `save_range_to_file`：seek+write 改为 pwrite/pwritev 批量写；匿名页集较大时可并行写 | 取决于 range 数和写入量：0.1~5 ms，大集合时更多 | C | 两种架构通用 |
| P4 | pause | `vm.rs:2847-2849,2876-2878`：两次串行 `sync_all` 改为“两份文件都写完后各做一次 `sync_data`”，或与内存写并行 | 0.5~3 ms | C | 两种架构通用 |
| R3 | create + resume | `msix.rs:140-142`：循环内的 `group.enable()` 改为每设备一次，路由每设备只提交一次（建议并入 R2） | ≤0.5 ms | C | 两种架构通用 |
| R5 | resume 尾延迟（create 也会走） | `CubeShim/shim/src/sandbox/sb.rs:56-57,281`：重连固定退避 200 ms 改为 5/10/20/50/100 ms 指数退避 | p50 预计不变；命中重试的样本可省最多 ~190 ms | C（缺重试率数据） | 两种架构通用 |

**三条最重要的结论**

1. **最确定的可省时间是 off-CPU 的固定 sleep，而不是 perf 上的热点。** R1 和 P2 都是“先 sleep 1 ms 再复查”。cycles 采样看不到 off-CPU 时间，所以 R4 那份 148K 样本的 perf 不可能发现它们。R1 的触发率已有实测：4 份 VMM 日志中 380/400 次 restore 都是 `after 1 try, recv 1`，即每次 create/resume 都在 VMM 线程上固定付出 ≥1 ms。
2. **pause 的 VMM 侧耗时没有任何内部分段。** 仓库里只有 pause req 的端到端数据：c1 117.9 ms、c50 385.6 ms，来自 HEAD `95f776b7` 的提交说明，口径包含 agent 连接处理和快照落盘。从代码推断主要成本在 `send_pagemap_anon_memory`（扫描 + 写页）。P1 对空闲沙箱几乎没有收益，只对写过大量内存的沙箱有效，所以要先补测分段，再决定 P1/P3/P4 的顺序。
3. **resume 没有独立的 VMM 路径。** 用户 resume 就是“用 pause 快照走一次 Create”，RestoreVm 与 create 共用同一段代码，因此 R1/R2/R3 对 create 和 resume 同时生效。resume 的端到端数据（c50 p50 486–489 ms，来自 PR #1771 表格）远大于 RestoreVm（约 9–16 ms），瓶颈在编排层，不在 VMM。

**最值得先验证的两条**：R1（改动约 20 行，现有日志已能直接复核）和 P2（改动约 15 行，用 strace 数 `clock_nanosleep` 即可复核）。两者都与其他候选正交，可以叠在同一组 3+3 中。

---

## 1. 方法与数据来源

### 1.1 数据来源（只读）

| 来源 | 内容 | 可信度 |
|---|---|---|
| `perf-lazy-irqfd:perf-opt/attempts.md` | lazy irqfd 严格 3+3：restore 段 19.10/17.94/19.63→14.71/14.94/15.16 ms；确认组 300/300 | A（严格组） |
| `perf-lazy-irqfd:perf-opt/reports/lazy-irqfd-crosscpu-20261027.md` §4.5 | 单独拆出的 `Callee=Ch RestoreVm` stat：Kunpeng bench-2g eager 16.0 / lazy 9.0 ms；AMD 7.0 ms | 只作方向参考（成功率 472/500 vs 466/500） |
| `perf-lazy-irqfd:e2e-breakdown/report.md` | Kunpeng c1 1c2g：RestoreVm 9.02、ResetVm 4.41、CreateContainer 8.03、LaunchVmm 1.12 ms（eager，v0.7.1） | A−（整数 ms，每组 100 样本） |
| `feat/create-latency-language-breakdown:sve-opt-records/R2,R3,R4,R5` | c100 分段、B/C/A 实验均失败、kprobe `kvm_irqfd_assign` 59 次/VM（p50 55 µs / mean 96 µs）、tokio runtime 0.13 ms | A（实验结论）/ B（perf） |
| `tmp/timing-run-54-20261021T0946-phase/{vmm,shim}-phase-c100.csv`（未跟踪的草稿） | VMM 内部分段（c100，eager）；min 值可近似看作无竞争 | B（草稿，未入库） |
| `tmp/timing-run-54-*/capture/evidence/vmm-delta.log`（4 份） | `tap.rs:185` 日志行 “got 1 fd after N try, recv M” | A（原始日志） |
| HEAD 提交说明 `95f776b7` | pause req p50：c1 160.2/148.8→117.9 ms；c50 417.4/426.5→385.6 ms；resume “unchanged” | A（端到端口径，未说明架构） |
| `github/master` 提交 `4b9c5809`（#1731）、`3a200fe4`（#1771）、`d6d664e3`（#1343）、`89a9c812`（#1865）、`2dec4b59` | pause/resume/scan/vGIC 的实测表格 | A（作者实测，多为 x86/PVM） |

另外两点：

- R1 记录里的 “`shim.RestoreVm` PerfTrace p50 93.735 ms” 来自本地构建；HEAD 和分析分支的 `CubeShim/shim/src/log/mod.rs` 都没有 PerfTrace 插桩，因此**不可复现，本文不使用**。
- lazy irqfd **不在 HEAD**（`git merge-base --is-ancestor perf-lazy-irqfd HEAD` 为假）。R2 以它为前提。

### 1.2 关键命令

```bash
git show <branch>:<path>
git log --all --oneline -i --grep='pause\|resume\|snapshot\|dirty\|irqfd\|pagemap'
git merge-base --is-ancestor <c> HEAD
for b in github/master master perf-lazy-irqfd; do git show $b:hypervisor/net_util/src/tap.rs | grep -c 'from_millis(1)'; done   # 去重
grep -o 'got [0-9]* fd after [0-9]* try, send [0-9]* recv [0-9]*' tmp/timing-run-54-*/capture/evidence/vmm-delta.log | sort | uniq -c
python3 计算 tmp/timing-run-54-20261021T0946-phase/*.csv 各 phase 的中位数与最小值
```

### 1.3 如何避免“把热点当成可省时间”

R4 已经警告过：perf 的 inclusive 占比不能当作可移除时间。例如 `thread_start` inclusive 13.1%，但去掉 runtime 后 RestoreVm 反而从 87 ms 变成 120 ms。本文的处理规则如下：

1. **分母只用 wall-clock 分段**：shim stat 的 RestoreVm、attempts 的 restore 段、pause req。不用 perf 百分比当分母。
2. **收益 = 可移除的单位数 × 已实测的单位成本**，不按 inclusive 帧计算：
   - R1：sleep 次数（日志实测）× sleep 时长（1 ms 加 timer slack）− Cubelet 回包时间（上界 1 ms）；
   - R2：可推迟的 irqfd 数 × legacy 实测得到的每条 irqfd wall 成本（0.144 ms，见 §3.1）；
   - P1：直接引用 #1343 的 before/after。
3. **优先找 perf 看不见的 off-CPU 等待**（sleep、回包等待、fsync）。这类时间在 wall 分段里能看到，在 cycles 采样里看不到，而且只要改掉就能省下。
4. 所有 B/C 项都附带“上实现前的计数门禁”（§3），门禁不过就不做 3+3。

---

## 2. 三阶段时间账本

### 2.1 create（模板快照恢复），Kunpeng，warm tap pool

| 段 | p50 | 来源 | 性质 |
|---|---:|---|---|
| E2E（cube-bench） | 47.3–50.8（eager）→ 43.3–45.9（lazy） | attempts.md 严格组 | 实测 A |
| shim 内合计 | 27.65–29.83 → 26.7 | attempts.md | 实测 A |
| `start vm start → agent is ready` | 15.35–19.63 → 14.34–15.16 | attempts.md | 实测 A |
| **RestoreVm（Callee=Ch stat）** | **eager 16.0 / lazy 9.0**（bench-2g 2c2g，c5）；eager 9.02（1c2g，c1） | crosscpu §4.5（方向参考）；e2e-breakdown | 实测 |
| restore 段扣掉 RestoreVm 后（launch_vmm + wait_notify + agent 握手） | ~7–8 | crosscpu §4.5 | 实测差值 |
| ResetVm（connect_agent + reset_guest，`sb.rs:254-266`） | 4.41（c1）/ 6.20（c5） | e2e-breakdown | 实测 |
| CreateSandbox / CreateContainer | 2.09 / 8.03（c1） | e2e-breakdown | 实测 |

**RestoreVm 内部（VMM 线程）**。来自 `tmp/timing-run-54-20261021T0946-phase/vmm-phase-c100.csv`，eager。min 值近似无竞争；c10 值来自 R2。

| 子段 | 代码 | min / c10 | c100 p50 | 说明 |
|---|---|---:|---:|---|
| API + `recv_vm_config/state`（JSON）+ `Vm::new_from_snapshot` | `lib.rs:762-835` | ≈1.0–1.2（= shim.ch_restore 8.246 − vmm.restore 7.008，min）；其中 new_from_snapshot 0.36 | 0.48（new_from_snapshot） | JSON 用 `from_slice`，≤0.7 ms |
| `DeviceManager::restore` → `create_devices` | `device_manager.rs:4922-4933` | **2.27** | 2.53（14/100 因 tap 池停顿达 136–143 ms） | **含 ≥1.06 ms 的 tap sleep（R1）** |
| CpuManager restore | `cpu.rs:2170` | 0.24 | 0.25 | ~670 次 SET_ONE_REG |
| `restore_vgic_and_enable_interrupt` | `vm.rs:2288-2360` | 1.08（c10） | 65.0 | 32 条 legacy irqfd，lazy 的目标 |
| `restore_devices`（tokio 5 worker） | `device_manager.rs:4467-4553` | 2.19（c10）/ 2.73（min） | 55.8 | 含约 24 条 MSI-X irqfd 注册，R2 的目标 |
| start_restored_vcpus / setup | `vm.rs:2794-2806` | 0.07–0.1 / 0.03 | 0.1 / 0.04 | |
| `Vm::resume` | `vm.rs:2624-2652` | 0.11–0.17 | 0.17 | |

c100 时 vgic/devices 增长约 50 倍，而纯 CPU 段基本不变，与 R2 的结论一致：瓶颈在 host 全局的 `irq_bypass_register_consumer`（全局 mutex 加链表遍历）。

### 2.2 pause（冻结 VM + 生成快照）

**实测锚点**（只有端到端口径，包含 agent 连接处理、快照落盘和 teardown，**不是** VMM 内部 `Vm::pause` 的单独耗时）：

| 指标 | 数值 | 来源 |
|---|---|---|
| c1 pause req p50 | 160.2 / 148.8 → **117.9 ms** | HEAD `95f776b7` 提交说明（2C2000M，未注明架构） |
| c50 pause req p50 | 417.4 / 426.5 → **385.6 ms** | 同上 |
| teardown 后移（unmap + KVM fd close ≈ 60 ms） | c1 149.8 → 88.4 ms | `4b9c5809`（#1731，v0.7.2，2 GiB）；HEAD 中对应提交为 `b6d52800`/`366dd0a5` |
| 增量扫描（kpageflags → bit 61） | 写入 256 MiB：209→178 ms；1 GiB：384→305 ms；64 MiB：155→154 ms | `d6d664e3`（#1343，PVM 2 GiB，**HEAD 未合入**） |

**VMM 内部：无实测分段。** 以下阶段按 HEAD 代码推导，入口为 `update_ext.rs:231` → `sb.rs:1509` `pause_vm_to_snapshot_inner` → `lib.rs:689` `vm_pause_to_snapshot`。

| # | 阶段 | 代码 | 主要成本 | 估算 | 依据 |
|---|---|---|---|---:|---|
| a | shim 端 quiesce（不关连接） | `sb.rs:341-347` | 停 monitor/oom 任务 | <0.1 | 代码 |
| b1 | 激活 pending virtio 设备 | `vm.rs:2611` | 通常无事 | ~0 | 代码 |
| b2 | **CpuManager::pause** | `cpu.rs:2106-2132` → `648-663` | 每个 vCPU：pthread_kill → 首次检查几乎必然为 false → **sleep 1 ms** | **≈1.06 × nvCPU** | 代码（P2） |
| b3 | DeviceManager::pause（串行）+ GIC pause（ITS/pending 表写回 guest RAM） | `device_manager.rs:4825-4844` | 线程握手、KVM_DEV_ARM_* ioctl | <0.5–1 | 估算 |
| c | `Vm::snapshot`：vCPU 寄存器（~670 次 GET_ONE_REG）、vGIC（~660 次 GET_DEVICE_ATTR）、设备快照（新建 5-worker runtime，约 0.13 ms，见 R5）、JSON | `vm.rs:2669-2740`、`device_manager.rs:4862-4920` | ioctl 与序列化 | ~1–2 | ioctl 计数取自 `89a9c812`；单次成本按其 0.44 µs 推算 |
| d1 | config.json 写 + `sync_all`；state.json（先 `snapshot.clone()`）写 + `sync_all` | `vm.rs:2821-2878` | 2 次串行 fsync | 0.5–5 | 估算（P4） |
| d2 | **增量内存**：`/proc/self/pagemap` 2 GiB（512k 项，4 MiB）+ **每个 present 页 lseek+read kpageflags** | `memory_manager.rs:2405-2473`、`pagemap_anon.rs:188-286` | 每页 2 次 syscall（VMM 线程，带 seccomp） | 约 0.3 µs × present 页（x86 推算）：空闲时为个位数 ms，写过 1 GiB 时约 80 ms | #1343（P1） |
| d3 | 写匿名页：每个 range 一次 seek + write_volatile_to（buffered，写 CubeCow 卷） | `memory_manager.rs:2352-2387,2461-2465` | memcpy 进 page cache | 与写入量成正比；#1343 称“写盘占主导” | P3 |
| e | `vm.shutdown()` + drop（unmap/KVM close 已后移） | `lib.rs:701-731`、`vm.rs:1296-1339` | 唤醒并 join 设备线程 | 几 ms（估算） | `4b9c5809` |
| f | shim `wait_notify` + `store_pause_snapshot_metadata` | `sb.rs:1539-1547` | 事件与小文件 | <1 | 估算 |

快照类型说明：Cubelet `snapshot_base_memory.go:271-306` 在可 reflink 克隆 base 时下发 `incremental`，VMM 因此走 `send_pagemap_anon_memory`。若主机开启 `host.memory.dirty_log`（默认关闭），则走 `memory_manager.rs:3092-3168` 的 KVM 脏页位图路径，不扫 pagemap。

### 2.3 resume（从 pause 快照恢复）

- **路径事实**：`sb.rs:1448` `pause_vm` 与 `sb.rs:1574` `resume_vm` 是死路径；用户 resume 等于 Master 拿 pause 快照走 **Create**（Task::create → start_vm → restore_vm → connect_agent_with_retry），与 create 共用 `lib.rs:762` `vm_restore`；`sb.rs:1645` 只被 rollback 使用。
- **实测**：HEAD 上没有 resume 基线。可引用的只有：PR #1771（github，非 HEAD）c50 resume req p50 489.1 / 489.4 / 486.3 ms；`95f776b7` 写的是 “resume unchanged”，没有给数值；2026-06 博客中 x86 单次 resume ~42 ms（旧版本，不可比）。**结论：HEAD 上 resume 无实测基线。**
- **与模板恢复相比，Rust 侧的差异**：

| 维度 | 模板快照（create） | pause 快照（resume） | 对 Rust 优化的含义 |
|---|---|---|---|
| 内存镜像 | 一个 base 被多次 create 复用，page cache 热 | 每次 pause 一个 CubeCow 卷；同节点刚 pause 后，buffered 写留在 page cache；跨节点 S3 时为冷 | 冷读体现在 RestoreVm **之后**的 guest page fault（connect/reset 段），不在 RestoreVm 内 |
| 快照体积/内容 | config/state 小；memory 为全量镜像 | 相同格式（增量只是写法不同，结果仍是全尺寸镜像） | RestoreVm 的 mmap 为 O(1)，与体积无关 |
| dirty-log | `RestoreConfig.dirty_log`（`sb.rs:1080`）按主机策略；开启后 `restored_from_image=true` | 同左 | aarch64 上 file-backed MAP_PRIVATE 本来就是 4K stage-2 映射，开启日志的额外 fault 有限（推断，未测） |
| vsock | 模板快照中的连接状态 | 携带 ttrpc/fwd/monitor 连接，restore 时 `queue_rst_for_connections`（`virtio-devices/src/vsock/device.rs:390`） | guest 侧处理 RST；若首次重连失败，固定退避 200 ms（R5） |
| agent 重连 | `connect_agent_with_retry(restore=true)`，最多 5 次 | 同左，而且 guest 刚解冻，更容易遇到瞬时失败 | R5 |

- **结论**：resume 的 VMM 可优化点就是 create 的 RestoreVm 段（R1/R2/R3）。resume 的端到端（~487 ms @c50）远大于 RestoreVm，瓶颈在编排层。

---

## 3. 候选优化点

### 3.1 create + resume（RestoreVm 段，两者共用）

#### R1 — tap fd 取回：去掉 1 ms 轮询 sleep

- **位置**：`hypervisor/net_util/src/tap.rs:198-211`（`request_from_pool`：socket `set_nonblocking(true)`，见 206 行），`:142-157`（`recv_with_fds` 返回 EAGAIN 就 `std::thread::sleep(1 ms)` 后重试）。调用链：`open_tap.rs:84` ← `virtio_devices::Net::new` ← `device_manager.rs:2443` ← `create_devices`（`device_manager.rs:4929`）← `Vm::restore`（`vm.rs:2757-2761`）。全部同步执行在 **VMM 线程**上，处于 RestoreVm 关键路径。
- **现状证据（A）**：4 份 `.54` VMM 日志共 400 次 restore：

  | run | try=1, recv=1 ms | try=2/3 | try≈130（Cubelet 停顿） |
  |---|---:|---:|---:|
  | 0920-warm | 100 | 0 | 0 |
  | 0522 | 99 | 1 | 0 |
  | 0536 | 95 | 5 | 0 |
  | 0946-phase(c100) | 86 | 0 | 14 |

  Go 服务端不可能在 `write_all` 之后、紧接着的第一次 `recvmsg` 之前就回包，所以首次必然 EAGAIN，必然 sleep 1 ms（hrtimer 加默认 50 µs slack ≈ 1.05–1.1 ms）；第二次即成功，说明真实回包时间 < 1 ms。
- **改法**：保持非阻塞 socket；EAGAIN 时调用 `libc::ppoll(&[pollfd{fd, POLLIN}], deadline 剩余时间)` 后再 `recv_with_fds`，并加 10 s 总超时（现在是无限循环）。`ppoll` 已在 vmm 线程的 seccomp 白名单中（`seccomp_filters.rs:567`）。顺手把日志改成 µs（现在 `as_millis` 会把 0.1 ms 显示成 0/1）。改动约 20 行，不涉及协议。
- **收益**：节省 = sleep(≈1.06 ms) − Cubelet 实际回包时间（未测，上界 1 ms；Go goroutine 唤醒 + JSON + sendmsg 一般在 0.05–0.3 ms）。取 **−0.75~1.0 ms**：
  - RestoreVm lazy 9.0 ms → −8~11%；eager 16.0 ms → −5~6%；c1 1c2g 9.02 ms → −8~11%；AMD 7.0 ms → −11~14%；
  - shim total 26.7 ms → −3%；E2E ~45 ms → −2%；
  - c100 时，Cubelet 停顿期间的轮询粒度误差（约 6%）也一并消除，即 136–143 ms 的样本可回落到真实等待时间。
- **置信度**：B（触发率 A；节省幅度取决于未测的回包时间，但有明确上下界）。
- **风险**：低。行为语义不变，只是等待方式不同。注意 c100 时 Cubelet 停顿造成的长等待依旧存在（属于 Go 侧问题，N3）。
- **去重**：HEAD、master、github/master、perf-lazy-irqfd 都仍是 sleep(1 ms)；R1/R2/R4 及 `tmp/fragments/hypervisor.md` 都没有识别到这一点（R2 只把 128–134 次重试记成 “Go/tap 等待”）。
- **架构**：通用。
- **验证**：
  1. 零代码门禁：在 `.54` 上 `strace -f -T -e trace=recvmsg,clock_nanosleep -p <shim>` 抓 10 次 create，确认每次 1 次 `clock_nanosleep(1ms)`，并得到首个成功 `recvmsg` 前的真实等待时间。
  2. 3+3：同一二进制用开关文件切换（如 `/etc/cube/tap_poll_off`），按 ABABAB 跑 n=100 c=5，warm 128 taps，每轮先清空并 `drop_caches`。指标：`Callee=Ch RestoreVm` p50、`start vm start→agent ready` p50、CreatePodSandbox p50、E2E p50，要求 100/100。验收标准：3 对差值同号，且 RestoreVm 中位差 ≥0.5 ms。

#### R2 — MSI-X irqfd 稀疏化/延迟注册（建立在 lazy legacy 之上）

- **位置与现状**：
  - `pci/src/msix.rs:112-146` `MsixConfig::set_state`：每个 unmasked 向量调用 `update()`（`interrupt.rs:168-193`：route.enable 加整表 `set_gsi_routes`），随后在**循环内**调用 `group.enable()`（`interrupt.rs:133-139`），把该组**全部** route（包括 masked 和从不触发的 config 向量）都执行 `KVM_IRQFD`。
  - 现状计数：kprobe 实测 59 次/VM（32 legacy + 27 MSI-X，R4 §10）；lazy legacy 后 56→24/VM（attempts.md），即剩约 24 条 MSI-X irqfd 全部在 restore 时注册。
- **改法**（与 `perf-lazy-irqfd` 的 legacy 实现同构）：
  1. `MsiInterruptGroup` 增加 `lazy_msix` 标志，并给每个 route 增加 `eager: AtomicBool`；
  2. `update()` 在 lazy 模式下只写入 `gsi_msi_routes` 并提交路由（保留现有路由语义）。masked 向量若已注册，照旧 `disable`；未注册的向量不再 `enable`；
  3. `trigger()` 首次遇到未注册 route 时执行 `route.enable()`（KVM 在 `kvm_irqfd_assign` 时会注入已累积的 eventfd 计数，不丢中断，R4 §9 已核对），然后写 eventfd；
  4. `notifier()` 被调用（vhost-user 场景下把 call fd 交给后端：`virtio-devices/src/transport/pci_device.rs:833` ← `vhost_user/vu_common_ctrl.rs:256`；vdpa/vfio 同理）时，把 route 标为 eager 并立即注册，因为这类向量绕过 VMM，不能延迟；
  5. `MsixConfig::set_state` 在 lazy 模式下不调用 `group.enable()`，并把每个向量一次的整表提交合并为每设备一次（即 R3）。
- **收益推导**（B，按已实测的 legacy 单条成本外推，不使用 perf 百分比）：
  - legacy 严格组：28 条 irqfd → restore 段 −4.16 / −4.03 ms，约 **0.144 ms/条**（含 c5 下 host 全局 irqbypass 竞争）；
  - 约 24 条 MSI-X 中，vhost-user fs 的队列向量（约 2 条）必须 eager，可推迟约 22 条 → **RestoreVm −3.2 ms**（9.0 ms 的 −35%）；
  - create 窗口内会被首次触发的向量（vsock rx/tx、console tx、blk 等，估计 5–8 条）在设备线程上补注册，其中 vsock 的部分落在 connect_agent 上。扣掉后 **E2E −1.0~2.5 ms**（45 ms 的 −2~5%）。
- **置信度**：B−。门禁：在 guest 内执行 `cat /proc/interrupts`（create 完成即刻，以及 create 后 1 s），统计各 virtio MSI-X 向量的计数；**只有“create 窗口内为 0 的向量数”≥12 才继续**。另可在 VMM 加 debug 计数器（首次 trigger 时打印 device/vector）。
- **风险**：中高。
  - masked/PBA 语义：未注册 route 在 masked 状态下 trigger，走的是 `pci_device.rs` 的 PBA 路径，需逐条核对；
  - 热插拔、rollback、resume 的 `set_state` 路径；
  - vhost-user 必须 eager；
  - 首次 trigger 与 `update()` 并发，需要像 legacy 一样处理 CAS/锁；
  - x86 上 lazy legacy 曾编译失败（crosscpu §3），本项必须用 `cfg` 正确隔离，或实现为与架构无关的代码。
- **去重**：所有分支只有 legacy lazy；R4 §7.2 提过“窄范围 MSI-X 清理”，但没有方案也没有计数；R2 的 D（MSI-X 路由一次提交）未实现。本项是“稀疏化 MSI-X irqfd 注册”，与已排除的 B（只批量提交 GSI 路由）不同。
- **架构**：aarch64 收益最大（GIC + irqbypass 竞争）。x86 的 MSI-X 也走 irqfd，但 AMD RestoreVm 只有 7.0 ms，须单独测。
- **验证**：在 lazy legacy 合入后的二进制上，用开关文件 `/etc/cube/lazy_msix_off` 做 ABABAB 3+3，c5 n=100。指标：RestoreVm p50、`kvm_irqfd_assign` 次数/VM（perf probe）、connect_agent（ResetVm）p50、E2E p50。另跑一轮 c100 观察竞争段 `restore.devices`。

#### R3 — MSI-X restore 循环清理（并入 R2；单独做收益 ≤0.5 ms）

- `msix.rs:140-142` 在每个向量的循环内都调用 `group.enable()`。第二次起只是原子扫描，没有 ioctl；真正的成本在每个向量的 `set_gsi_routes`（整表 `KVM_SET_GSI_ROUTING`，内核要分配新表、做 srcu 同步并执行 `kvm_irq_routing_update`，Self 0.18%）。
- 改法：循环内只插入路由，循环结束后提交一次。每个设备从 N 次降为 1 次（全 VM 约 24→8 次）。
- 收益：已实测的同类改动 B（legacy 32→1）在 3+3 下只是噪声（+2.6% / E2E −0.32%），因此单独做的预期 ≤0.5 ms，**置信度 C，不单独立项**；只作为 R2 的组成部分。

### 3.2 pause

#### P1 — 回移 #1343：pagemap bit 61 判定匿名页

- **位置**：`vmm/src/pagemap_anon.rs:228-283`。对每个 present 页执行 `kpageflags_file.seek()` 加 `read_exact(8B)`，即每页 2 次 syscall，运行在带 seccomp 的 VMM 线程上；`soft_dirty.rs` 也同样调用。入口在 `memory_manager.rs:2417`，pause 的 Incremental 路径为 `memory_manager.rs:3171-3173`。
- **改法**：cherry-pick `d6d664e3`（`github/master`、`master` 已有，HEAD 没有）：只读一次 pagemap，用 bit 61（PM_FILE）区分文件页和私有匿名页，删除 kpageflags 路径。改动 6 个文件，+383/−117，需处理与 HEAD `soft_dirty.rs` 的冲突。
- **收益（#1343 作者实测，PVM 2 GiB，首次 create_snapshot）**：

  | guest 写入量 | before → after | Δ |
  |---|---|---|
  | 64 MiB | 155 → 154 ms | ≈0 |
  | 256 MiB | 209 → 178 ms | −31 ms（−15%） |
  | 1 GiB | 384 → 305 ms | −79 ms（−21%） |

  仅看扫描：快 29–35 倍。按 HEAD c1 pause 117.9 ms 换算：空闲 bench 沙箱约 0–1%；写过 256 MiB 以上的真实 agent 沙箱可省 −15~21% 量级。Kunpeng 上每次 syscall 加 seccomp 的成本高于 x86，收益方向不变、幅度可能更大，但未测；若 Kunpeng 内核为 64K 页，页数减少 16 倍，收益相应缩小，需先 `getconf PAGESIZE` 确认。
- **附带收益**：消除 PFN→kpageflags 两次读取之间页迁移导致**静默漏存 guest 数据**的竞态（#1343 的论证）。
- **置信度**：x86 为 A；Kunpeng 为 B。
- **风险**：低到中，只需 cherry-pick 冲突处理。依赖内核 pagemap bit 61 语义（#1343 已覆盖到 5.4.119）。
- **验证**：
  - 零代码：VMM 日志已有 “PagemapAnon snapshot: total=… anon=…”（`memory_manager.rs:2426-2427`）和 “PagemapAnon snapshot saved”（`2467-2468`），第二列是进程相对 ms 时间戳（`hypervisor/src/common.rs:217-221`），相邻行时间差即可分出“扫描”和“写页”两段。
  - 3+3：`examples/snapshot-rollback-clone/bench_pause_resume_concurrency.py`，c1 与 c10 各跑；分两组沙箱，空闲组和预先写 256 MiB 的组，每组 n≥30 对。指标：pause req p50、VMM 扫描段 p50、resume p50（防回退）。

#### P2 — vCPU 暂停信号：去掉固定 1 ms sleep，并改为并行等待

- **位置**：`vmm/src/cpu.rs:648-663` `VcpuState::signal_thread` 先 `pthread_kill(SIGRTMIN)`，若 `vcpu_run_interrupted` 仍为 false，则 `thread::sleep(1 ms)` 后重发。调用方 `CpuManager::pause`（`cpu.rs:2106-2116`）按 vCPU **串行**逐个执行。vCPU 线程要经过“退出 KVM_RUN → 信号处理 → `immediate_exit` 再 run 一次 → `store(true)`”（`cpu.rs:1038-1048`），需要几十 µs，所以紧接在 kill 之后的检查几乎必然为 false，**每个 vCPU 固定 sleep ≥1 ms**。
- **改法**：
  1. 先对所有 vCPU 执行 `pthread_kill`，再统一等待；
  2. 等待采用退避 sleep 20 µs → 50 µs → 100 µs → … → 1 ms，每轮重发信号。保留上游注释所说的“避免与 vCPU 线程优先级反转”（不忙等，仍是 sleep），只缩短量子。也可以由 vCPU 线程在设置 `vcpu_run_interrupted` 后 `unpark` 等待方，但这需要 VMM 线程句柄，改动更大。
- **收益**：2 vCPU 时 ≈2.12 ms → ≈0.1 ms，**−2.0 ms**：c1 pause 117.9 ms 的 −1.7%；c50 385.6 ms 的 −0.5%；1 vCPU 模板 −1.0 ms。CommitSandbox/AppSnapshot 走同一个 `Vm::pause`，同样受益。
- **置信度**：B（代码确定；未直接测量）。门禁：pause 时 `strace -f -e trace=clock_nanosleep -T -p <shim>`，确认 vmm 线程上每个 vCPU 有 1 次 1 ms 的 nanosleep。
- **风险**：低。若 vCPU 线程与 VMM 线程同在 1 核 cpuset（pool cgroup，`4f4fa82c`），短 sleep 仍会让出 CPU，不会饿死对方。shutdown/delete 路径同样受益。
- **去重**：所有分支（包括上游）都未改。
- **架构**：通用。
- **验证**：同一二进制用开关文件 ABABAB 3+3，c1 与 c10 的 pause req p50，各轮 n=30 对，要求成功率 100%；辅以 strace 计数。

#### P3 — 匿名页写出：pwrite/pwritev 批量写，大集合时并行（C）

- **位置**：`memory_manager.rs:2352-2387` `save_range_to_file`：`seek` 加 `write_volatile_to`，每个连续匿名 range 一次。调用点在 `2464`（pagemap_anon）、`2611`（soft-dirty）、`3160`（dirty-log）、`3183`（full）。
- **改法**：
  1. 去掉 seek，改用 `pwrite`（`FileOffset`）；
  2. 按 `IOV_MAX=1024` 把 range 打包成 `pwritev`（同一 file 中 offset 连续的合并）；
  3. 写入量大于约 64 MiB 时，用 `std::thread::scope` 加 2–4 个 writer 线程按 offset 分片。memory vol 是块设备（`/dev/cubecow/*`），buffered 写不会落入 XFS 那样的 inode 写锁串行化——此点需在目标内核上确认。
- **收益**：第 1、2 步省去每个 range 1–2 次 syscall，约 0.3–0.5 µs/次，range 数未知，因此为 0.1–5 ms。第 3 步只对大写入量有意义，#1343 指出写盘占主导，但没有拆分数据。**先做 §5 的分段测量，确认“写页”段占 pause 的比例和 range 数，再决定。**
- **风险**：中（第 3 步在 c50 时与内存带宽竞争，可能反而变差）。

#### P4 — 快照 config/state 的 fsync（C）

- **位置**：`vm.rs:2846-2849`、`2874-2878`：两次 `sync_all()` 串行地夹在两份 JSON 写入之间。注释写的是“cross-machine pause-snapshot”，但同一路径下 memory 镜像**并不 fsync**（同类修复 `b5c9c5dd` 因损害 cube-bench Pause 指标，已在 master 由 `2dec4b59` 回退），所以这两次 fsync 提供的只是部分持久性。
- **改法**：先写完两份文件，再分别做 `sync_data()`（文件已经 `set_len`，元数据无需额外同步），或交给后台线程，与内存写并行执行、在回复前 join。持久性语义不变。
- **收益**：0.5–3 ms（XFS 小文件 fsync 需要 log force；c50 时竞争更大）。置信度 C。门禁：pause 期间 `strace -T -e fsync`。

### 3.3 resume 专属（CubeShim，次要）

#### R5 — 重连退避 200 ms

- `CubeShim/shim/src/sandbox/sb.rs:56-63`（`RECONNECT_ATTEMPTS=5`、`BACKOFF=200ms`、`DEADLINE=10s`）、`:281`。恢复后首次 connect/reset 若瞬时失败，下一次尝试固定在 200 ms 之后。
- 改法：退避序列改为 5/10/20/50/100/200 ms，截止时间不变。
- 收益：p50 预计不变（只要多数恢复一次成功）；命中重试的样本可省最多 ~190 ms，对 p95/p99 和 c50 resume（~487 ms）可能有可见影响。置信度 C：**没有重试率数据**。门禁：统计 `cube-shim-req.log` 中 “reconnect attempt N/5 failed” 的出现率；低于 1% 则不做。
- 归属：shim 编排层（另一位 agent 的重点），这里只给出 Rust 证据。

---

## 4. 已排除 / 不建议

| 方向 | 结论与原因 |
|---|---|
| SVE/NEON 向量化 | 已排除（R1 记录）。唯一的位图循环在 pause 发送侧；P1 去掉 kpageflags 后剩下的 pagemap 读取由内核页表遍历主导，用户态循环也不值得向量化 |
| 去掉每次 restore 新建的 tokio runtime | 已实测回退（87→120 ms；顺序版 127→268 ms）。pause 侧 `DeviceManager::snapshot`（`device_manager.rs:4878`）同样新建 runtime，但只有 0.13 ms（R5），不做 |
| 批量 GSI 路由 32→1 | 已实测无效（+2.6% / −0.32%）。R3 只作为 R2 的附带部分 |
| vGIC distributor 寄存器裁剪 | 已实测无效（+5.1%）。**新证据**：`89a9c812`（master，#1865）用配对样本（每臂 4000 次）测得 restore 窗口 −0.22 ms（95% CI 0.15–0.30）、ioctl 1327→831。效应真实，但只占 RestoreVm 9 ms 的 ~2.4%，低于 3+3 的检出下限；可以顺手回移，**不计入收益** |
| tap pool 预热（N3） | 属于配置，不计入；R1 是它之外、仍然残留的 1 ms |
| lazy legacy irqfd | 已在 `perf-lazy-irqfd` 实现，**HEAD 未合入**，是 R2 的前提；合入时要修 x86 编译（crosscpu §3：E0433） |
| seccomp 每线程 BPF JIT 合并 | 重新评估后**不建议**：R4 Self 统计中 `seccomp_set_mode_filter` 只有 0.08%；`start_restored_vcpus`（含 vCPU 线程装 filter 后过 barrier）p50 仅 0.07–0.1 ms；设备 worker 线程在自己线程内装 filter，不在 VMM 线程关键路径上。可省时间上限 <0.1 ms |
| JSON 双重编码（`SnapshotDataSection.snapshot: String`） | restore 用 `from_slice`；API + JSON + new_from_snapshot 合计仅 1.0–1.2 ms（min），JSON 部分 ≤0.7 ms，改格式会破坏快照兼容性，不值得 |
| `prefault`/`MAP_POPULATE`、hugetlb（N2） | page fault 主要发生在 RestoreVm 之后；prefault 2 GiB 会让 RestoreVm 明显变长；hugetlb 需要改快照存储形态。不属于 Rust 内的可落地改动 |
| 开启 `host.memory.dirty_log` | 代码已存在（`86f6d9b9`），属于**配置**，不计为 Rust 优化；但它是 pause 侧最大的潜在杠杆（把 O(present 页) 的扫描换成位图读取），建议与 P1 同批做 A/B 作为参照 |
| 跨 VM 复用 irqfd | KVM 无此 API（R4 已排除） |

---

## 5. 建议的下一步排序与需要补测的项

**实施顺序**（按 收益确定性 ÷ 改动风险 排序）：

1. **R1**：约 20 行，先跑 strace 门禁，再做 3+3。
2. **P2**：约 15 行，先跑 strace 门禁，再做 3+3。可与 R1 叠在同一个实验二进制里，用两个独立开关、交错的 4 臂设计，分别归因。
3. **pause 分段测量**（无需改业务逻辑）。零代码方式：从 VMM 日志第二列的相对 ms 时间，按行切出“扫描 / 写页 / teardown”；shim 侧取 `update_ext.rs:225` 的 “pause to snapshot” 与 `task_srv.rs:738` 的 “update req finish”。轻量插桩方式：在 `cube_hypervisor.rs:299` 的 `pause_vm_cube_with_config` 外加 `new_stat("PauseToSnapshot")`（与 RestoreVm 同一套 stat 机制），并在 `memory_manager.rs` 的扫描与写页前后各加一行带 µs 的 info。这一步决定 P1/P3/P4 的先后。
4. **P1**：cherry-pick `d6d664e3`，需用“空闲 / 预写 256 MiB”两组沙箱分别验收。
5. **R2**：先合入 lazy legacy（修 x86 cfg），再跑 `/proc/interrupts` 门禁，然后做原型并 3+3（c5 与 c100 各一组）。
6. P3/P4 视第 3 步的结果决定；R5 视重试率决定。

**统一的 3+3 规范**：同一二进制用开关文件切换；ABABAB 交错；n=100，c=5（pause 用 c1 与 c10 对）；warm 128 taps；每轮先清空沙箱并 `drop_caches`；每轮要求 100/100 成功。验收条件：3 对差值同号，阶段中位差 ≥1.5%，且超过同二进制噪声地板（AMD 等价臂 E2E −1.8%）。p95/p99 只作参考（`.54` 存在 600 ms 级宿主停顿）。

**需要补测的数据**：

| # | 项 | 方法 | 服务的候选 |
|---|---|---|---|
| 1 | HEAD 上 pause 的 VMM 内部分段（scan/write/fsync/vCPU pause/teardown） | 第 3 步 | P1/P2/P3/P4 |
| 2 | HEAD 上 resume（由 pause 快照 create）c1/c5 的 RestoreVm、ResetVm、E2E 基线 | bench_pause_resume_concurrency + shim stat | 全部 resume 结论 |
| 3 | Cubelet tap 回包的 µs 级延迟 | strace `-T`，或把日志改为 µs | R1 幅度 |
| 4 | pause 时的 present 页数与匿名页数 | `/proc/<pid>/smaps_rollup` 的 Rss/Anonymous，加现有 PagemapAnon 日志 | P1 幅度 |
| 5 | create 窗口内各 MSI-X 向量的触发计数 | guest `/proc/interrupts` | R2 门禁 |
| 6 | Kunpeng 宿主页大小 | `getconf PAGESIZE` | P1 |
| 7 | 重连重试率 | grep “reconnect attempt” | R5 |
