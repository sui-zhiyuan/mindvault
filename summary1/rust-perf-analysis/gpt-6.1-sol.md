# CubeSandbox Rust 编排与 I/O 性能审计：create / pause / resume

分析者：gpt-6.1-sol。代码位置以 **v0.7.1 @ d095826a** 为准；分析日期 2026-11-05。只读检查，未构建、测试、部署或运行基准。唯一写入为本报告。

## 0. TL;DR

**当前没有一个新的 Rust 优化得到 A/B 验证，不能承诺累计 5%。** 最先验证两个小而明确的冗余操作：恢复容器时的空 mount helper，以及没有消费者的 monitor 健康连接。前者潜在收益较大，后者验证/实现成本最低。pause 的三个大方向已经在 HEAD 实现，不应重复计功。

A＝本方案已有有效实测 A/B；B＝代码与历史分段/perf 支持的估算；C＝待验证假设。**下表 C1–C4 为优化候选；G1–G4 是取证门禁，不是已经量化的优化。** 宁可四个候选，也不凑八个有虚假收益的承诺。

| 排序/ID | 阶段 | 精确目标 | 预计可省时间及分母 | 置信度 |
|---|---|---|---|---|
| 1 / C1 | create、resume | 没有 propagation mount/umount 时不执行 guest mount helper | 命中请求 **1.5–2.0 ms**；投影 warm shim total 26.7 ms 的 **5.6–7.5%**；投影 warm E2E 45–50 ms 为 **3.0–4.4%** | B，未 A/B |
| 2 / C2 | create、resume | `monitor_vm(false)` 不再新建不会使用的 health 连接 | **0.10–0.20 ms** 假设；投影 warm shim total 26.7 ms 的 **0.37–0.75%**；历史第二次 connect p50 0.14 ms，不是首次连接 19.87 ms | C |
| 3 / C3 | resume、部分 create-from-snapshot | RESTORE 且无 virtio-fs 工作时省掉 CreateSandbox 空 RPC | 命中请求 **0.2–0.5 ms**；历史纯 RPC 0.510 ms 的 **39–98%**，投影 warm shim total 26.7 ms 的 **0.75–1.87%** | B，版本契约前置 |
| 4 / C4 | create、resume | 两个 reset RPC 合并为一个有版本协商的 RestoreReset RPC | **0–1.5 ms** 探索区间；旧 reset 3.701 ms 的 **0–40.5%**，投影 warm shim total 26.7 ms 的 **0–5.6%**；下界为零 | C |
| 5 / G1 | create、resume 的失败重连尾部 | 200 ms 退避/握手分帧取证 | 全体 p50 **未证明可省，暂计 0 ms / 0%**；一次早就绪的重试理论可少等至多约 180 ms，不能当全体收益 | C / 门禁 |
| 6 / G2 | S3 create、pause、resume | JSON-RPC 逐字节读改有状态缓冲 | 无 S3 分段基线；当前收益记 **未量化**，公式 `Σ(bytes−buffered_reads)×read自有成本` | C / 门禁 |
| 7 / G3 | S3 pause、create、resume | 全索引重写与持久化临界区取证，再决定增量日志 | 无 S3 分段基线；当前收益记 **未量化**，上限是实测索引序列化/锁等待/持久化时间，不含后端 RPC | C / 门禁 |
| 8 / G4 | CubeAPI resume/connect | update 成功后额外 GET 的协议折叠 | 无当前 resume p50；当前收益记 **未量化**，等于可移除 GET 实测往返，需 CubeMaster 响应扩展 | C / 门禁 |

**不能相加各行历史 p50 来预测组合 p50。** 表中比例是局部投影，不是实验结果；C1/C3 的命中率、guest 模板版本都会稀释总体收益。没有当前 Kunpeng pause/resume 分段 p50，故不伪造其百分比。

## 1. 方法与数据来源

### 1.1 只读方法与版本范围

- 核实 `pwd`、`git status --short`、`git rev-parse --short HEAD`：HEAD 为 d095826a，未发现已跟踪文件修改。工作区还有并行任务的未跟踪目录，不接触。
- 用 `git log --all --oneline --regexp-ignore-case --grep='pause\|resume'`，以及 perf/latency、reset、monitor、buffer、persist 等检索去重；用 `git show <commit>` 阅读已合入 pause 优化。
- 当前文件用 read/grep；其他分支全部用 `git show <branch>:<path>`。未 checkout/switch/stash/reset/commit；未运行 cargo、SSH、服务或压测。
- 仓库根下不存在独立 `repro/`；检索了全树 repro 文件名、docs/changelog、正式性能资产。未把不存在的目录当证据。

正式资产（以下是可直接运行的**读取命令**，不是当前工作树文件链接）：

```text
git show feat/create-latency-language-breakdown:sve-opt-records/R1-restorevm.md
git show feat/create-latency-language-breakdown:sve-opt-records/R2-restorevm-concurrency-analysis.md
git show feat/create-latency-language-breakdown:sve-opt-records/R3-createpodsandbox-analysis.md
git show feat/create-latency-language-breakdown:sve-opt-records/R3-restorevm-measurements.md
git show feat/create-latency-language-breakdown:sve-opt-records/R4-perf-c5-report.md
git show feat/create-latency-language-breakdown:sve-opt-records/R5-tokio-runtime-bench.md
git show feat/create-latency-language-breakdown:sve-opt-records/language-breakdown/REPORT.md
git show feat/create-latency-language-breakdown:sve-opt-records/language-breakdown/REPORT-90-final.md
git show perf-lazy-irqfd:perf-opt/attempts.md
git show perf-lazy-irqfd:perf-opt/reports/lazy-irqfd-crosscpu-20261027.md
git show feat/probe-perf:probe-breakdown/2026-10-22T1826-inband-timestamps/e3e2/EXPERIMENTS.md
git show feat/probe-perf:probe-breakdown/2026-10-22T1826-inband-timestamps/e3e2/REPEAT-5x100.md
```

来源缩写：**R3**＝上列 createpodsandbox analysis；**LB54/LB90**＝两个 language-breakdown 报告；**Attempts**＝lazy irqfd attempts；**CrossCPU**＝crosscpu 报告；**Probe-E**＝EXPERIMENTS 与 REPEAT-5x100。分析分支的正式归档提交为 6b969a54；不采用临时草稿中过大的收益数字。

### 1.2 三个已合入 pause 改动先读后排除

1. **366dd0a5 / b6d52800**：pause 回复前仍 shutdown、join vCPU、drop VM/设备；只将内存 unmap 和 KVM VM fd 最终释放用持有 memory manager 的后台线程后移。不是整个 teardown 异步化。接口实现见 [VMM pause2snapshot:689–736](</home/suine/projects/CubeSandbox/hypervisor/vmm/src/lib.rs#L689-L736>)。
2. **32004501 / 95f776b7 / 后续连接修正**：quiesce 不关闭 main、log、monitor 连接；跨 freeze 后让 restore RST 收敛。已删除旧的 pause 前 50 ms settle。成功后才 detached 停 log、清 client。见 [sb:333–377](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L333-L377>)、[sb:1485–1505](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1485-L1505>)。历史 5 ms settle 曾在 c50 约 1/50 restore wedge，因此不能为性能恢复提前断连。
3. **86f6d9b9 / 02887907 / e95f8b62**：host opt-in dirty logging 已从 Cubelet 注解传到 restore；已有 KVM dirty bitmap pause 写出路径。见 [sb:1078–1081](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1078-L1081>)、[config:48–51](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/config.rs#L48-L51>)。默认 false；启用配置不是新的 Rust 优化。

### 1.3 计量边界

- Rust **区段 wall time** ≠ Rust CPU 时间。agent RPC 包含 guest 调度、vsock、页错误；VMM `send_request` 包含内核 ioctl、VMM 控制循环和设备等待；FICLONE 是内核文件系统工作。
- LB54/LB90 是历史阶段账本，不能拿各阶段 p50 的和当联合分布的真实 p50，也不能把所有未归因 gap 归给 CubeAPI。
- R4 profile 时 E2E p50 343.352 ms，未 profile 约 60 ms；只用来认方向。Tokio `thread_start` 13.10% 是 inclusive；Self 0.02%，不是可省 13.10%。Rust CPU 3–5% 的已知结论也不意味着 Rust 编排改变只能省 3–5%：消除一次无效 RPC/进程可省的是调度和等待。
- R1 历史报告提到的 `PerfTrace` / 93.735 ms 来自当时临时构建。**当前 HEAD、分析分支 tip 的 [log 模块](</home/suine/projects/CubeSandbox/CubeShim/shim/src/log/mod.rs>) 均没有这个类型；`git log --all -S PerfTrace -- CubeShim/shim/src/log/mod.rs` 无结果。** 分支仅可核实 `language` 字段增补，不能声称直接设置环境变量就可重现该插桩。

## 2. 三阶段时间账本

### 2.1 create：真实调用链与历史分段

链路：

```text
CubeAPI create → CubeMaster/Cubelet/containerd → 新 shim Task::create
  load spec → sandbox mutex → init → join_host_cgroup
  create_sandbox → start_vm → launch_vmm → restore_vm
    SnapshotInfo load/版本验证/设备配置 → VMM VmRestore 同步往返
    app_snapshot_restore 成功直接返回；其他情况等待 VsockServerReady
  connect_agent_with_retry
    UNIX connect → CONNECT 1024 → 等 OK → 新 ttrpc client
    reset_guest: SetGuestDateTime → 读宿主随机数 → ReseedRandomDev
  CreateSandbox RPC → watch_oom → monitor_vm(false) 再建一条 vsock
  CreateContainer RPC → task Create reply
随后 Task::start → start_log_forward 再建独立 vsock → wait_process
→ Cubelet probe → API 回复
```

证据：[Task create:323–423](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/task_srv.rs#L323-L423>)、[create_sandbox:567–650](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L567-L650>)、[start_vm:965–1011](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L965-L1011>)、[connect/reset:251–308](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L251-L308>)、[reset:535–564](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L535-L564>)、[Task start:425–480](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/task_srv.rs#L425-L480>)、[container start/log:701–765](</home/suine/projects/CubeSandbox/CubeShim/shim/src/container/mod.rs#L701-L765>)。

**三层计时必须分开**（CrossCPU，Kunpeng / bench-2g，多轮中位数）：

| 层 | eager | lazy | 来源/属性 |
|---|---:|---:|---|
| VMM 单次恢复，`Callee=Ch, CalleeAction=RestoreVm` | 16.0 ms | 9.0 ms | CrossCPU §4.5，stat 毫秒向下取整；配对中位差 −5.5 ms |
| 整段 restore，`start vm start → agent is ready` | 23.41 ms | 17.63 ms | CrossCPU §4.1；**含 launch/connect/reset**，不是纯 RestoreVm |
| 客户端 create E2E | 53.25 ms | 48.60 ms | CrossCPU §4.1 |

第二层减第一层约 7–8 ms 归 launch、事件/agent 握手及 reset 等；E2E 减第二层还有 agent sandbox/container、Task Start、Go/containerd/probe/网络等。**差中位数不等于中位数之差的独立阶段估计**，只能说明量级。

CrossCPU 的 aarch64 成功率分别 472/500、466/500，不满足本任务要求的 100/100；不能用它代替严格验收。Attempts 的独立确认组两臂各 300/300，整段 restore 18.44/18.58/17.82 → 14.34/14.41/14.63 ms，E2E 48.6/49.6/49.0 → 43.3/43.4/45.2 ms。已是落地 lazy irqfd 的收益，不重复提出。

**warm c5 整体校准**：Attempts 的不同轮次 shim total 26.7–30.7 ms、E2E 约 45–50 ms；pre-lazy 的 prep 0.31–0.40 ms、restore+agent 15.35/16.85/15.95 ms、agent-ready 后至 Create reply 10.59–12.81 ms。不同运行不可拼成一条虚构 timeline。

**R3 直接 timer，c5、100/100（历史临时插桩，不是 HEAD 可复用类型）**：

| Rust 区段 | p50 | 包含/归属 |
|---|---:|---|
| create_sandbox_total | 37.963 ms | init 后到 sandbox 建好，**不含后续 CreateContainer** |
| start_vm | 12.808 ms | 内含 RestoreVm 约 11.4 ms |
| 首次 connect_agent | 19.871 ms | 单独握手 timer；不能外推到 guest ready 后第二连接 |
| reset_guest | 3.701 ms | 两个串行 guest RPC |
| 纯 CreateSandbox RPC | 0.510 ms | 不含监控第二连接；不是 stat 的 CreateSandbox 整个范围 |
| watch_oom + monitor | 1.945 ms | 内含第二连接；第二 connect 单项 p50 0.14 ms、mean 约 2.0 ms |
| restore 容器 mount helper | 2.35 ms | R3 候选分析记录；不是 CreateContainer 全部时间 |

注意 create_sandbox_total **也不等于 Task CreatePodSandbox total**，尤其 C1 helper 在该 timer 结束后的 CreateContainer 才执行，不能除以 37.963 ms 冒充该段收益。下文统一将 warm shim total 26.7 ms 用作 Task CreatePod 范围的跨 run 投影分母，并将 helper/RPC 本身的实测成本另列。历史 timer 与后续 warm 模板不同，所有投影须在新基线上重测。

**LB90 / LB54 账本**（两份历史运行；单位 ms；子段都是 wall-time）：

| 区段 | .90，100 并发 warm | .54，冷 tap |
|---|---:|---:|
| create E2E | 244.827 | 691.432 |
| Rust 区段预算合计 | 68.360（27.92% E2E） | 169.240（24.48%） |
| Shim/CreatePodSandbox 全包 | 约 68.3 | 约 169.2 |
| shim.CreatePodSandbox.local 残余 | 12.780 | 44.230 |
| agent.CreateContainer | 27.610 | 6.770 |
| agent.ResetVm | 12.460 | 6.560 |
| vmm.RestoreVm | 10.300 | 102.850 |
| agent.CreateSandbox（stat 范围） | 3.850 | 7.750 |
| vmm.LaunchVmm | 1.320 | 1.040 |
| cubecow 预算行 | 0.040，coverage 52% | 0.040，coverage 52% |
| 未归因 client/API/network gap | 71.827 | 63.322 |

cubecow 0.040 ms 是报告中的有限覆盖/约 10–40 µs 用户态预算，不是完整含 FICLONE/fsync 的稳定 p50。LB54 的 Go prepare 418.835 ms 主要冷网络；不转嫁给 Rust。R2 还证实 14/100 restore.devices 有 tap 请求 128–134 次 1 ms 退避，约 136–143 ms，背后是 Cubelet Go tap pool：Rust 栈在等，不等于 Rust CPU 可省。

### 2.2 pause：真实用户路径与已有数据

**不是 ttrpc Task::Pause**。[旧 pause_vm:1448–1451](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1448-L1451>) 直接报 legacy removed。真实链路：

```text
CubeAPI pause → CubeMaster update(pause) → Cubelet prepare memory artifact
→ task.Update 注解 PauseToSnapshot
→ update_route → do_pause_to_snapshot → SandBox::pause_vm_to_snapshot
  state/exec 禁止检查 → quiesce_agent_for_pause（不关闭连接）
  建/清 snapshot 目录 → VmPauseToSnapshot 同步等待
    VMM pause/freeze → snapshot 写出 → shutdown/drop VM
    仅 RAM unmap / KVM fd 最终 close 后台释放
  wait_notify(VmShutdown) → 三个版本文件读取/metadata.json 写出
  detached 停 log → 清 main/monitor/container client → Update reply
→ Cubelet CommitRootfs（必须在 freeze 后）
→ 记录 sandbox_spec/catalog，S3 时 seal/upload
→ keep_tombstone Destroy / binary delete / wait runtime gone → pause API reply
```

接口证据：[Update:692–739](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/task_srv.rs#L692-L739>)、[route:185–242](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/update_ext.rs#L185-L242>)、[sb pause:1467–1571](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1467-L1571>)、[VMM 请求:299–317](</home/suine/projects/CubeSandbox/CubeShim/shim/src/hypervisor/cube_hypervisor.rs#L299-L317>)、[Cubelet 内存冻结后 rootfs:416–436](</home/suine/projects/CubeSandbox/Cubelet/services/cubebox/pause_cow.go#L416-L436>)、[pause 回复前 Destroy:510–537](</home/suine/projects/CubeSandbox/Cubelet/services/cubebox/pause_cow.go#L510-L537>)。

| pause 区段 | 当前 HEAD p50 | 实测来源/判断 |
|---|---|---|
| API → Cubelet 前置资源/CoW 准备 | 无实测基线 | Rust cubecow + Go 编排；S3 不能套 XFS 数据 |
| Rust state/quiesce/watchers | 无实测基线 | 无固定 settle sleep；应很短但不编 ms |
| VMM freeze + snapshot + 同步 teardown | 无分段实测基线 | dirty 字节数、后端写、设备 stop 是主要分母；不能当 Rust CPU |
| event wait + metadata + detached log/client 清理 | 无实测基线 | 三版本读取为同步小 I/O；无收益证据 |
| Cubelet rootfs/catalog/S3 packaging/runtime gone | 无实测基线 | 已在用户 Pause 的回复前，不能只优化 Update 而不测 E2E |
| 同线历史 c1 Pause req p50 | **117.9 ms** | 95f776b7，旧 baseline A/B/A 160.2/148.8 → 117.9 ms，2C2000M |
| 同线历史 c50 Pause req p50 | **385.6 ms** | 同提交，旧 baseline 417.4/426.5 → 385.6 ms |

该 commit 数据没有完整当前 HEAD/host 架构/dirty-log 配置的重跑信息；可称“历史同线 pause 基线”，不能称“当前 Kunpeng 950 HEAD p50”。dirty-log merge 未提供新的数值。**当前 HEAD 的分段 pause 仍是无实测基线。**

补充确实存在的更旧数据（非当前 p50）：[BMI5 报告:492–506](</home/suine/projects/CubeSandbox/docs/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md#L492-L506>) 是 Intel x86_64、2C2GiB、full-memory-copy，c1 Pause **wall avg 558.4 ms**；不是 p50。PVM [旧表:455–467](</home/suine/projects/CubeSandbox/docs/blog/posts/2026-06-03-cubesandbox-perf-benchmark-pvm.md#L455-L467>) 的 Pause c1 wall avg 370.8 ms，同样不可校准当前 ARM dirty-log pause。

### 2.3 resume：重新创建，而不是旧 shim 原地 Resume

[旧 resume_vm:1574–1577](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1574-L1577>) 明确用户 resume 为 Master Create-from-pause-snapshot。[resume_vm_with_config:1645–1690](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1645-L1690>) 仅被 rollback 调用；不能把它的流程直接写成公共 resume。

```text
CubeAPI update(resume) → Master/Cubelet 读取 pause catalog/重新派生 rootfs/memory
→ 新 shim Task::create → 同 create 的 launch/restore/connect/reset
→ CreateSandbox(start_mode=RESTORE)
   pause_resume()/from_snapshot() 保留 guest 挂载，不重放 virtiofs
→ CreateContainer 复用快照内的 container_id，reset log readers、处理 propagation
→ Task Start 重连日志；probe/路由/状态收敛
→ CubeAPI 额外 fetch_sandbox_detail GET → resume reply
```

挂载证据：[guest_mount_restore:148–162](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L148-L162>)、[get_storages:397–423](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L397-L423>)、[agent restore container:172–215](</home/suine/projects/CubeSandbox/agent/src/rpc.rs#L172-L215>)、[API resume:332–362](</home/suine/projects/CubeSandbox/CubeAPI/src/services/sandboxes.rs#L332-L362>)。

| resume 区段 | 当前 p50 | 是否可借 create 数字 |
|---|---|---|
| pause catalog/内存与 rootfs 派生、S3 import | 无实测基线 | 否，后端/祖先链/缓存不同 |
| 新 shim launch + VmRestore | 无 resume 独立基线 | 可借 create 9 ms 纯恢复、14–20 ms 整段作量级参照，不能填成 resume 实测 |
| carried vsock RST 后 connect/reset/重试 | 无 resume 独立基线 | 共用代码，但 pause 快照携带活动连接状态，重试分布需单列 |
| RESTORE sandbox RPC / 恢复 container / log | 无 resume 独立基线 | 与模板 fresh create 的 mount/进程语义不同 |
| CubeAPI post-resume GET | 无实测基线 | create 无此额外 GET |
| 同线 pause 优化提交的 Resume | 仅“unchanged” | 95f776b7 未给独立数值 |

旧 BMI5 Resume c1 **wall avg 41.8 ms**，c10 **批次 wall avg 35.7 ms、均摊 3.6 ms**；旧 PVM c1 wall avg 18.9 ms、c10 均摊 2.7 ms。均不是 per-request p50，更不是当前 Kunpeng 950 数据。

### 2.4 低成本补账本办法（本次不执行）

1. 优先复用已有 [StatDefer](</home/suine/projects/CubeSandbox/CubeShim/shim/src/log/stat_defer.rs#L74-L115>) 与请求日志，按 InstanceId + 操作 ID join。HEAD 的 `CostTime` 用 `as_millis()` 向下取整；C2/C3 是百 µs 量级，必须加 µs/ns 字段，不能用整数 ms 验证。
2. create/resume 临时或后续正式插桩：Task entry/锁等待、start_vm、launch、metadata validate、VmRestore、UNIX connect、CONNECT ACK、每个 reset RPC、retry count/backoff、纯 CreateSandbox、第二 monitor connect、CreateContainer、Task Start/log connect。resume 带 `pause_snapshot_id` 分类，避免混入模板 create。
3. pause 插入六个边界：Update entry/锁、quiesce、VmPauseToSnapshot send/return、event receive、metadata、Update reply；VMM 仅接口周围补 freeze/snapshot/send-memory/shutdown/drop/release。Cubelet 串接 artifact prep、rootfs、seal/upload、Destroy/runtime-gone，才能得到真实用户 E2E。
4. 采集每段 monotonic wall 与 `CLOCK_THREAD_CPUTIME_ID`，以及 guest mount helper child elapsed。初筛只要每实例十几条结构化日志，无需 system-wide cycles perf；另采写出字节数、dirty 页数、RPC/重试数。
5. `cube-shim-req.log` 现有起止日志能读 create 三大段，`cube-shim-stat.log` 能读 RestoreVm/agent stats；**pause 当前无完整 StatDefer 范围，不能仅用现有 create analyzer 宣称已分解 pause。**

## 3. 候选优化点与门禁

### 3.0 所有条目统一的 3+3 A/B 协议

- 每次只改一个点；A/B/A/B/A/B，共各三轮，每轮 **n=100，成功 100/100**。默认先 c5，再 c100；pause/resume 是完整 100 对生命周期，分别做 c1/c50 校准；容量不能满足 c100 时先换小规格或扩容量，不收“仅成功样本更快”的结果。
- 同机 Kunpeng 950、同模板/agent/kernel、同资源、同 dirty-log、同 lazy irqfd 配置、同 warm tap、同冷热缓存规则；记录二进制 hash、组件与模板版本。S3 和 XFS 不混合，首轮冷/稳态分开。x86_64 单独做相同语义回归，不直接横比 ARM 的绝对 ms。
- **四项主指标均保留**：Shim/CreatePodSandbox p50、纯 Ch/RestoreVm p50、用户 Pause p50、用户 Resume p50；另记 create E2E p50、p95/p99、100/100 成功与错误类型。不得只比较整段 restore 却声称纯 RestoreVm 改善。
- 先比较每轮 p50 与配对差，不把 300 个样本合并后掩盖轮间漂移；记录置信区间/噪声。对百 µs 项，至少要求目标子段稳定下降并用负对照验证；E2E 不显著则只认子段，不宣传 E2E。
- C1/C2/C3/C4 主要省 agent/shim 编排，**纯 RestoreVm 预计不变**；若它大幅改变，先查环境噪声。任何错误率增加、mount/日志/时间/熵退化直接拒绝。暂停/恢复至少再做多次循环及载荷回归，不以一轮快通过。

### 3.1 create / resume 候选

#### C1：空 propagation 工作不 fork/exec guest mount helper

- **当前实现**：[agent RPC:172–215](</home/suine/projects/CubeSandbox/agent/src/rpc.rs#L172-L215>) 命中 restored container 后，无条件 `start_exec_process`；[helper:1687–1713](</home/suine/projects/CubeSandbox/agent/rustjail/src/container.rs#L1687-L1713>) 获取 `WAIT_PID_LOCKER`、同步 spawn + wait_with_output，阻塞 Tokio worker。子程序 [Exec 分支:304–306](</home/suine/projects/CubeSandbox/agent/src/main.rs#L304-L306>) 执行 [do_exec_mount:82–134](</home/suine/projects/CubeSandbox/agent/cube/src/rootfs.rs#L82-L134>)：即使两个向量为空，仍 open namespace + setns，再空循环退出。
- **具体改法**：第一版只在两个注解均缺失（并确认无继承同名环境变量）时跳过 helper；保留 restored process 查找、log reader reset、passfd 重连。如果覆盖字符串 `[]`，先用共享解析器完整解析两个 Vec，保留坏 JSON 的错误；仅当两 Vec 都为空才 skip。不能把空字符串/解析失败静默当无工作。
- **语义风险**：原 helper 还隐含检查 `/proc/<pid>/ns/mnt` 存在及 setns 权限。应保留针对 process 存活/namespace 可访问的轻量检查，或明确错误契约；不在多线程 agent 主线程直接 setns。非空 mount 仍走原 helper，不用“预 fork 池”增加复杂性。
- **关联/收益**：R3 记录 helper p50 2.35 ms，估命中请求省 **1.5–2.0 ms（64–85% helper）**。投影 warm Task CreatePod 范围的 shim total 26.7 ms，约为 **5.6–7.5%**；它发生在 CreateContainer，不能除以此前的 create_sandbox_total。按 LB54 CreateContainer 6.77 ms 算局部占 22.2–29.5%，只是跨 run 投影。投影 warm create E2E 45–50 ms 为 3.0–4.4%。resume 共享这条 restored-container 分支，但无其独立基线，收益只能写命中请求 ms，不填假 p50 百分比。
- **置信度/架构**：B；不是 A/B 实测收益。aarch64、x86_64 均适用，进程启动成本各机重测。模板快照内 agent 的代码必须更新并**重建模板/快照**；仅换 host shim 不会改变已快照的 guest agent。
- **3+3 增补**：记录 `helper_invoked`、两注解命中率、child elapsed、CreateContainer 纯 RPC；分无注解/空 Vec/有 mount/有 umount/无效 JSON/已退出容器六类。功能集必须覆盖 host-dir/plugin volume、日志连续性、passfd、signal/reap。无注解臂预期每 create 少一个 child，纯 RestoreVm 不变。
- **去重**：R3 已列但未实测；HEAD 没有空向量 fast path；全分支日志未发现已验证的同方案。不是已经落地项。

#### C2：删除或惰性建立没有消费者的 monitor health 连接

- **当前实现**：[create 调用:638–647](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L638-L647>) 和 [rollback 调用:1681](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L1681>) 都传 `false`；全仓没有 `monitor_vm(true)`。然而 [monitor:654–669](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L654-L669>) 无条件 `connect_agent`，只有 [check_agent 条件:690–698](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L690-L698>) 才会用 HealthClient。该连接不做检查，却消耗一次 vsock/ttrpc 建连并加入 carried ports 集合。
- **具体改法**：`check_agent == false` 时不新建 health 通道，仅启动 VMM 事件监控；保守变体是第一次真正启用健康检查时才建立独立通道。无需共享长流日志连接；也不关闭已有连接来换快。监控 shutdown/通知容器逻辑不变。
- **关联/收益**：R3 第二 connect p50 **0.14 ms**、mean 约 2 ms，后者有长尾，不能拿 mean 当 p50。假设省 **0.10–0.20 ms**，为 1.945 ms watchers 总段的 5.1–10.3%，投影 warm shim total 26.7 ms 的 0.37–0.75%；warm E2E 45–50 ms 的 0.20–0.44%。**不是首次 connect 的 19.871 ms。**
- **置信度/架构**：C（成本数字来自旧 timer，不是删除连接后的 before/after）；架构无关。port reservation 曾是正确性修复，[monitor_conn 注释:90–93](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L90-L93>) 明示 checks disabled、目的为保留端口：少建一条连接会改变 snapshot carried ports，必须验证 muxer/RST 正确性，而不是机械删除字段。
- **3+3 增补**：单测/计数确认 connect 从 main+health+log 三条降到 main+log；对每轮 create 后做 pause/resume、多周期回环、rollback、日志负载。特别统计 `Recv len invalid:0`、reset timeout、复用端口碰撞，要求全部 100/100，无历史约 1/50 wedge。微秒 timer 比较第二连接，整段 CreatePod/Resume 一并报。
- **去重**：跨 freeze 保留连接已落地，但“从一开始不建立未启用功能的连接”未做；二者不同，不重复声称保存 pause 前 50 ms。

#### C3：已恢复 guest 且无 virtio-fs 工作时省掉空 CreateSandbox RPC

- **当前实现**：[shim 请求:593–635](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L593-L635>) 总会发；[agent RESTORE 分支:1358–1371](</home/suine/projects/CubeSandbox/agent/src/rpc.rs#L1358-L1371>) 只调用 [add_virtiofs_storages:888–897](</home/suine/projects/CubeSandbox/agent/src/mount.rs#L888-L897>)，该函数跳过全部非 `virtio-fs` driver。
- **正确守卫**：`start_mode == RESTORE && storages.iter().all(|s| s.driver != "virtio-fs")`，并且已确认该 agent 版本的 RESTORE 语义。**不能写 `storages.is_empty()`**：[get_storages:441–456](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L441-L456>) 无条件放入 ephemeral shm，另外还有 pmem/disk/propagation，空集合守卫通常永不命中。也不能仅信 pause_resume flag 而不看最终请求。
- **具体改法**：仅省 RPC 发送；保留 reset_guest 屏障、watch_oom、monitor、stat 结束。优先限 pause_resume/from_snapshot；模板 from_tpl 的 `guest_mount_restore` 为 false，仍可能有新 virtiofs mount，不能跳。fresh boot 非 RESTORE 绝不可跳。
- **版本前置**：利用现有 [SnapshotInfo agent_version:116–118](</home/suine/projects/CubeSandbox/CubeShim/shim/src/hypervisor/snapshot.rs#L116-L118>) 和 [版本比较:191–199](</home/suine/projects/CubeSandbox/CubeShim/shim/src/hypervisor/snapshot.rs#L191-L199>) 识别已审计版本；缺失 agent_version 或无法证明能力时退回原 RPC。版本相等只保证一致，不自动证明具备该语义，需明确可识别的构建集合。不要新造无实现的能力位。
- **关联/收益**：R3 纯 RPC p50 0.510 ms；估命中请求省 **0.2–0.5 ms（39–98% RPC）**；投影 warm shim total 26.7 ms 的 0.75–1.87%，warm E2E 45–50 ms 的 0.40–1.11%。**不能省 stat 的 CreateSandbox 3.85/7.75 ms 全段**，其中有第二连接等工作。pause-resume 与 runtime snapshot restore 命中率分别统计；模板 create 只在确实无 virtiofs 时命中。
- **置信度/架构**：B；无 A/B，架构无关；主要风险是后续 agent 给 RESTORE 增加初始化工作造成 host fast path 漏执行。当前数据不足以给 resume 总 p50 百分比。
- **3+3 增补**：每轮分类型记录 `RESTORE/no_virtiofs/agent_version/skip`；验证 restored mount、共享目录、插件卷、host-dir 和 DNS/网络状态维持。对有 virtiofs 与旧 agent 做反向控制，确认 RPC 未跳过。四项主指标一起采集，纯 RestoreVm 应不变。

#### C4：专用单 RPC 合并时间/熵重置，不重复并发失败实验

- **当前实现**：[reset_guest:535–564](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L535-L564>) 串行两次 ttrpc 往返；agent [set time:1663–1674](</home/suine/projects/CubeSandbox/agent/src/rpc.rs#L1663-L1674>) 与 [reseed:1601–1612](</home/suine/projects/CubeSandbox/agent/src/rpc.rs#L1601-L1612>) 都是小同步工作。
- **具体改法**：增加兼容的 `RestoreReset{sec,usec,entropy}` unary RPC，guest 内顺序执行两工作并成功后回复；旧版本 fallback 两 RPC。不将非幂等 CreateSandbox/CreateContainer 包在 retry loop 中。继续保持失败前不进入后续 agent RPC 的门槛，不缩减熵、不删重置时间。
- **与已失败实验差异**：R3 §8 对比两个 RPC `tokio::join!`，CreatePod 子段 p50 三轮均值 33.044 → 32.860 ms，只有 **−0.184 ms / −0.56%**；reset 10.540 → 10.436 ms，拒绝。并发仍有两次消息、两次 dispatch；专用合并减少一次消息边界，但**并不因此证明有收益**。R2 的旧单轮更差也不能忽视。
- **收益/置信度**：C，探索区间 **0–1.5 ms**；占历史 reset 3.701 ms 的 0–40.5%、投影 warm shim total 26.7 ms 的 0–5.6%；warm E2E 投影 0–3.3%。数字是原型预算，不是按两 RPC 时间对半切的预测。若单次 RTT/dispatch 可省部分不足 0.5 ms，停止追逐高收益。resume 无独立基线，不报它的百分比。
- **架构/风险**：架构无关，需 guest agent 与模板重建、协议兼容。两操作中途失败后的重复执行要与现有 retry 一致；时间应取发送时而不是提前冻结在请求构造时。
- **3+3 增补**：计数 reset wire message 从 2→1；分别记录 guest 内两个 syscall 时间和 host RTT；覆盖旧 agent fallback、reseed 失败、第一 RPC reset 错误、迁移恢复时间正确、多个 clone 不共享可预测随机状态。必须验证四项主指标和 100/100；若只 E2E 变好而 reset/CreatePod 没变，如旧实验一样判噪声。

### 3.2 pause / storage 门禁：先获得可优化分母

#### G2：S3 JSON-RPC 逐字节读取（小实现，先量再做）

- **代码/当前实现**：[s3 read_line:1425–1444](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L1425-L1444>) 每字节调用一次 `UnixStream.read([u8;1])`；[call_raw:1268–1320](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L1268-L1320>) 在整个 RPC 内持有连接 Mutex。每个响应 B 字节至少 B 次读 syscall，而这些自有成本还延长同连接后续请求排队。
- **路径关联**：create/resume 的 S3 clone+activate 使用 [S3 clone:785–866](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L785-L866>)；pause 打包会 seal memory/meta，[Finalize:101–120](</home/suine/projects/CubeSandbox/Cubelet/storage/s3_finalize.go#L101-L120>)、[seal→FFI:203](</home/suine/projects/CubeSandbox/Cubelet/storage/s3_finalize.go#L203>) 进入 Rust snapshot RPC。**XFS 完全不命中；不能拿 LB 的 0.040 ms 或 warm XFS E2E 当 S3 分母。**
- **具体原型**：连接槽存 `Connection{stream, read_buffer}`，用 4–8 KiB 分块 read 到换行，保留换行后剩余字节；重连同时清 buffer。保留 EINTR、EOF、response ID 校验，不用每次新建即丢弃的 BufReader，防止读过头丢下一帧。
- **收益/置信度**：C，**无实测基线，ms/百分比不具备可报告的值**。预测公式 `ΔT_cpu≈Σ(B_i−read_calls_buffered_i)×c_read`，锁等待改善需另测；后端处理/磁盘等待不在可省项。示例仅作门禁算式：若一次操作 10 个 256-byte 响应、每 syscall 自有时间实测 0.5 µs，则预算 1.275 ms；若实测 S3 phase=20 ms，则 6.4%；这些数**不是本仓库实测或预期承诺**。
- **3+3 门禁/风险**：先统计每操作响应长度、read 次数、自有 CPU 与后端 RTT；投影达到 0.5 ms 才做原型，按 §3.0 验四指标。加入分帧、多帧、EINTR/EOF、frame ID mismatch 回归。架构无关，Kunpeng syscall 常数须实测，不能套 x86。
- **去重**：HEAD 已长连接缓存，不再提“复用 RPC 连接”；f00e74b1 已修 EINTR/帧错位，缓冲不能撤销其正确性。

#### G3：S3 全索引重写与锁等待（pause 的明确取证点，不先许诺 WAL 收益）

- **代码/当前实现**：[persist_index:1543–1576](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L1543-L1576>) 每次 clone/snapshot/activate/deactivate 变更，clone 全部条目、排序、pretty JSON、write、fsync、rename、dir fsync。snapshot 在 [767–772](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L767-L772>) 的全局 name_index 写锁下持久化；clone [792–866](</home/suine/projects/CubeSandbox/cubecow/src/engine/s3.rs#L792-L866>) 甚至持有写锁跨 clone/activate RPC 与 persist。
- **路径关联**：S3 pause seal/deactivate/delete 会产生多次状态变更，[Finalize:132–158](</home/suine/projects/CubeSandbox/Cubelet/storage/s3_finalize.go#L132-L158>) 在 pause 回复前。create/resume 派生 clone 同样命中。不是不同 shim 之间的 Rust 锁；这是 Cubelet 中共享 cubecow engine 的索引/连接，作用域不同。
- **具体改法（门禁通过后）**：第一阶段分解索引锁等待、RPC、排序编码、fsync；若 full-index CPU/索引读者等待占主导，采用带序号/checksum 的增量元数据日志，返回前 durable append，后台 compact，再缩短 idx 写锁；同名创建用 Pending 预留、失败回滚，源对象生命周期用引用/对象锁。保留持久化完成契约，不能简单把 `persist` 扔后台后提前成功，也不能无锁并发覆写固定临时文件。
- **瓶颈边界**：仅缩短 name_index 写锁但保留单连接 Mutex，并不能让所有后端 RPC 并行。不能拿 clone API 整个 RTT 当可省。若只有后端 SPDK/fsync 慢、索引 CPU 很小，就不做 WAL；写放大必须先证明。
- **收益/置信度**：C，**无 S3 基线，未量化**。上限为 `ΔT ≤ T_sort+encode+redundant_checkpoint + 可消除的idx锁排队`，不包括仍需的 durable append/后端工作。例：若测得一次 pause 6 次持久化、每次 full-index 专属开销 1 ms、新日志每次 0.2 ms，则投影 4.8 ms；若该机 Pause=120 ms 则4.0%。这是将来测量的代入公式，不能拿历史 XFS 117.9 ms 直接套 S3。
- **3+3 门禁/风险**：固定 index 大小 100/1000/10000 条，分无并发/并发 create、pause。先只采分段，不改数据结构；超过主路径 1.5% 才实现。验四指标，并在操作回复后 kill/restart 恢复、断电模拟、同名竞争、activate/delete race 上验证 crash 一致性。d0c13d81 已修 activate/delete index poisoning，不可退回。两架构均适用，风险高、优先级低于 C1–C3。

### 3.3 resume / RPC 门禁

#### G1：200 ms retry backoff 与 CONNECT 分帧（不宣称正常 p50 收益）

- **当前实现**：[constants:53–63](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L53-L63>) 为5次、200 ms、总10 s；[retry:251–299](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L251-L299>) 重试 connect+reset，所有错误类别；[connect:529–552](</home/suine/projects/CubeSandbox/CubeShim/shim/src/common/utils.rs#L529-L552>) 仅一次 read_buf，短于2字节即失败。协议字节合法拆包成单字节也可能触发无必要重试，必须用故障注入证明发生率。
- **具体改法**：先把 handshake 改为有总长上限/逐次超时的完整行解析，避免合法分帧被视为断连；对真实 restore settle 再测试短阶梯退避（如20/50/100 ms后回200 ms），保持10 s总体预算并合理增加尝试数，防止5次短退避过早失败。不通过缩短总deadline伪造“更快失败”。
- **关联/预算**：create/resume 共用；pause 不做新连接。不重试正常请求的 p50 预算为 **0 ms/0%**。某次客体在20 ms已可用而原策略200 ms才试，理论可少180 ms，但仅影响该失败子集；若重试比例不足一半，不能声称全体 p50 改善。至多4次固定sleep共800 ms；10 s deadline用于挂起尝试，不是所有成功请求必付10 s。
- **3+3 门禁**：先从 warn 日志统计 attempt>=2比例、错误分布、成功等待；记录 `read_len`/ACK完整性与guest可用时刻。分别跑正常模板与高并发pause循环；短退避原型需100/100，统计请求/连接风暴、重连端口碰撞以及最后一个错误保留。架构无关；C，正常 p50 无证据，不优先动可靠性修复。

#### G4：CubeAPI resume/update 后的详情 GET（协议依赖，不是 Rust JSON 微调）

- **当前实现**：[resume:337–362](</home/suine/projects/CubeSandbox/CubeAPI/src/services/sandboxes.rs#L337-L362>) update成功后串行 `fetch_sandbox_detail`；[connect:370–386](</home/suine/projects/CubeSandbox/CubeAPI/src/services/sandboxes.rs#L370-L386>) pause 情况是 GET→update→GET。该函数是 [Master GET:556–576](</home/suine/projects/CubeSandbox/CubeAPI/src/services/sandboxes.rs#L556-L576>)。[SandboxUpdateResponse:1474–1477](</home/suine/projects/CubeSandbox/CubeAPI/src/cubemaster/mod.rs#L1474-L1477>) 当前仅 ret。
- **具体改法**：扩展 Master update 响应可选的已恢复 sandbox summary（template_id/实际host_id/envd_version），Rust 有字段则直接构造 response，缺失 fallback GET。**这不是纯 Rust 独立补丁，Master 协议扩展是前置依赖。** 不能缓存/复用 resume 前 host_id 代替，因为跨节点恢复可能换host；不能无条件删除GET造成错误路由。
- **预算/置信度**：C，当前无 resume p50、无 GET 分段，ms与百分比不做承诺。严格公式 `ΔResume≈T_post_GET−T_added_response_encoding`，相对为 `ΔResume/Resume_baseline`；例如实际测到 GET1 ms、resume50 ms，才可谈2%。LB 的63–72ms gap不能全算作该GET。
- **3+3 门禁**：API span分解update/postGET/handler CPU，达到0.5–1ms才推动协议；按§3.0采四指标，预计仅Resume E2E变，纯RestoreVm/CreatePod/Pause不变。覆盖同节点/跨节点S3恢复、旧Master、already-running409、404、timeout、envd版本与host字段一致性。架构无关。

## 4. 已排除 / 不建议

| 方向 | 代码/数据证据与理由 |
|---|---|
| SVE/NEON 数据循环 | R1无≥1KiB规则循环；restore是mmap/ioctl/协议等待。不会把页错误/IO向量化；不重复提出。 |
| 去掉 restore 的5-worker Tokio runtime | R3 实测87→120 ms、顺序127→268 ms，拒绝。R5 build+drop仅0.13–0.15ms；create约33ms的0.4%。不把Thread::new inclusive当创建成本。本报告也不提runtime池；如以后做，必须保留设备并行、区分长期复用与失败的scope/串行。 |
| 批GSI 32→1；裁vGIC寄存器 | R3纯恢复+2.6%、+5.1%，均失败，不提。 |
| lazy legacy irqfd | perf-lazy-irqfd已实现、默认开；不是HEAD新候选。若跨CPU出包，还须注意CrossCPU记录原分支x86 cfg缺陷；不能把x86二进制等价臂的−1.2%噪声当ARM通用收益。 |
| tap pool预热/N3 | 已配置落地、非Rust代码；R2含Go tap等待不当Rust可省。 |
| pause前关闭agent/减少settle | HEAD已去旧50ms；连接必须跨freeze，历史5ms方案有约1/50 wedge，不能退化可靠性。 |
| 再异步化整个pause teardown | HEAD只延后unmap/KVMfd，保留回复前设备/磁盘fd释放语义；再提前回复会使Cubelet rootfs快照/删除与旧VM写入竞态。没有新测量不提。 |
| 把Task Delete的50ms算Pause收益 | [Delete:591–597](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/task_srv.rs#L591-L597>) 是spawn后的sleep，不阻塞Delete回复；而用户pause [destroy:130–134](</home/suine/projects/CubeSandbox/Cubelet/services/cubebox/destroy.go#L130-L134>) 使用WithSkipRuntimeAPI→binary delete，[delete_shim:56–71](</home/suine/projects/CubeSandbox/CubeShim/shim/src/service/srv.rs#L56-L71>) 直接kill/清资源。不能宣称省固定50ms。 |
| seccomp每线程BPF JIT合并 | R4只有inclusive seccomp_set_mode_filter0.38%/Self0.08%，没有每shim实际安装次数和JIT耗时。本次不重开为候选；先用线程名+seccomp安装/JIT计数与持续时间取证，不能从多个静态apply_filter调用点推实际次数。 |
| 提前启动VMM“省1.5ms” | launch有1.04–1.8ms历史成本，但把它移到Task entry前不是省E2E；warm prep只有0.31–0.40ms可重叠且还需处理init/cgroup。现有 [先join再launch:567–575](</home/suine/projects/CubeSandbox/CubeShim/shim/src/sandbox/sb.rs#L567-L575>) 保证NUMA首触。无独立可重叠工作证据，不列候选。 |
| 把同步VMM API改async就省其等待 | [send_request:273–278](</home/suine/projects/CubeSandbox/hypervisor/src/lib.rs#L273-L278>) / [service:85–88](</home/suine/projects/CubeSandbox/hypervisor/vmm/src/api/service.rs#L85-L88>) 阻塞recv；shim只有 [2个worker:44–54](</home/suine/projects/CubeSandbox/CubeShim/shim/src/main.rs#L44-L54>)。异步bridge可改善反应性，但不会自动省VMM工作；Probe-E里相关host调度最大约2ms、不足解释7ms握手。没有worker饥饿证据不预估毫秒收益。 |
| shim日志逐条flush“必省IO” | [writer:157–166](</home/suine/projects/CubeSandbox/CubeShim/shim/src/log/mod.rs#L157-L166>) 确有逐条Tokio flush，但生产者 [try_send:354–357](</home/suine/projects/CubeSandbox/CubeShim/shim/src/log/mod.rs#L354-L357>) 不等落盘；flush不是fsync。可batch减少后台IO，却无关键路径p50收益证据；不能把所有日志IO计入create/pause等待。 |
| CubeAPI请求每次新建HTTP client | 已用 [共享client:42–49](</home/suine/projects/CubeSandbox/CubeAPI/src/state.rs#L42-L49>)、[池包装:40–54](</home/suine/projects/CubeSandbox/CubeAPI/src/cubemaster/mod.rs#L40-L54>)；不重复提连接复用。默认 [FileLogger:129–135](</home/suine/projects/CubeSandbox/CubeAPI/src/logging/file.rs#L129-L135>) 也是channel发送，不是每请求同步磁盘落盘。 |
| cubecow CommandExecutor 50ms轮询 | [executor:56–101](</home/suine/projects/CubeSandbox/cubecow/src/pkg/executor/mod.rs#L56-L101>) 确有50mssleep，但全库Rust检索无调用者；当前引擎是reflink/S3，不把死代码列为三路径收益。 |
| 删除FICLONE/fsync或提前并行rootfs snapshot | [reflink clone:715–724](</home/suine/projects/CubeSandbox/cubecow/src/engine/reflink.rs#L715-L724>) / [directory fsync:1143–1149](</home/suine/projects/CubeSandbox/cubecow/src/engine/reflink.rs#L1143-L1149>) 是持久化/内核工作。pause rootfs必须在freeze后；盲删durability或提前快照改变一致性，不建议。 |
| 全量prefault/hugepage直接宣传p50 | Probe-E：warm/cold只有p95 1.46→8.23ms，50ms尾部5×100干净样本0次，不能证明p50收益；prefault只是把page-in成本前移，还需E2E验证。内部策略交由VMM专项，不在本报告重复列。 |

## 5. 下一步排序与补测清单

1. **前置：给当前 HEAD 的 pause/resume 补最小分段账本。** 当前只有历史Pause req p50，无freeze/snapshot/metadata/Rust teardown各段；先区分S3/XFS与dirty-log配置，获得四项主指标。这是测量基础，不是优化收益。
2. **先验证 C1 与 C2（各自3+3，不能合包）。** C1潜在1.5–2ms、客体版本更新成本较大；C2宿主改动小、无需新协议，潜在只有0.1–0.2ms但容易归因。它们最值得先做，不代表C2能过E2E显著性门槛。
3. **随后 C3**：先确认agent版本与命中率，再跳空RPC。检查无virtiofs，而非空storages，避免永不命中的fast path。保留reset/reconnect屏障。
4. **C4后置**：先测单次RPC真正消息/dispatch可省部分；已有两RPC并发失败。只有投影≥0.5ms才为新协议/模板重建付成本。
5. **S3专项先G2、后G3**：G2改动可控，先read计数；G3涉及durability/索引并发，只有稳定可见写放大才实施。XFS warm create不承担这两项预期收益。
6. **G1只追尾部**：获取retry命中率、ACK完整性后再动退避；不把健康请求正常p50当200ms可省。G4先测postGET，不用LB gap猜API成本。

### 必须补测的观测值

- 当前Kunpeng HEAD：create/CreatePod、纯RestoreVm、Pause、Resume每轮p50/p95/p99；所有轮100/100。
- C1：propagation缺失/空/非空比例、helper自身elapsed与child个数、agent模板/快照版本。
- C2：main/monitor/log连接各自耗时、carried port集合变化、恢复失败率。
- C3：RESTORE且无virtiofs的真实比例、暂停恢复和runtime-snapshot分别统计、版本可识别率。
- C4/G1：各reset syscall、每RPC RTT/dispatch、attempt分布、200ms backoff花费与guest实际就绪时刻。
- pause：dirty页数/写字节数、VMM freeze/snapshot/stop/drop、metadata、rootfs、S3 seal/upload、runtime-gone时长。
- G2/G3：S3响应字节/read syscall数、连接锁/索引锁等待、后端RPC、full-index sort/encode/fsync时间随索引规模变化。
- G4：update与postGET分别的wall及handler CPU，跨节点resume返回host/envd字段一致性。

**最终判断**：新的可靠空间首先在“少做一次没有工作内容的进程/RPC/连接”，而不是优化Rust算术。pause大头必须先量化现有接口等待；当前证据不足以对新pause优化承诺毫秒收益，也不足以把create的好数字直接迁移成resume基线。
