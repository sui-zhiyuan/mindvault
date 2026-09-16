# 虚拟化厂商、产品与沙箱全景

> 更新：2026-09-16

## 简介

本文回答"虚拟化领域有哪些公司和产品，以及沙箱"。厂商侧的关键判断是：**传统 Type-1/Type-2 二分法已经不够用**——现代云厂商走的是"专用卡 + 极简 hypervisor"的第三条路，它既不是 Type-1 也不是 Type-2，而是**把虚拟化本身卸载到硬件**。这条路线在 2017 年前后由 AWS Nitro、阿里云神龙、华为云擎天几乎同时确立，是当前云虚拟化架构的主线。沙箱侧的关键判断是：**沙箱是目的，虚拟化只是手段之一**；按"是否共享宿主内核"分层，才能同时解释隔离强度与性能代价为什么单调变化。

前置知识：建议先读[硬件虚拟化：发展过程、多层方案与硬件能力依赖](hardware-virtualization.md)。
相关：`docs/rust-kunpeng/cubesandbox-rust-analysis.md`、`docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| Type-1 / Type-2 | — | 裸金属 hypervisor / 宿主式 hypervisor 的传统分类 |
| microVM | Micro Virtual Machine | 极简设备模型的轻量虚拟机（Firecracker 提出） |
| DPU / IPU | Data / Infrastructure Processing Unit | 卸载网络、存储与虚拟化开销的专用处理单元 |
| MOC | — | 阿里云神龙架构中的虚拟化卸载卡 |
| SR-IOV / VF | Single Root I/O Virtualization / Virtual Function | 单物理设备虚拟出多个可直通给 VM 的功能实体 |
| VBS / HVCI | Virtualization-Based Security / Hypervisor-enforced Code Integrity | Windows 基于虚拟化的安全机制 |
| SDN / MANA | — | Azure Boost 的自研高性能网卡 |
| gVisor / runsc | — | Google 的用户态应用内核沙箱及其 OCI 运行时 |
| TCB | Trusted Computing Base | 必须被信任的代码与硬件集合；隔离方案的强度上限由它决定 |
| CoW | Copy-on-Write | 写时复制，快照与克隆的通用实现机制 |

## 核心内容

### 一、类型学：五类，而不是两类

| 类型 | 结构 | 代表 | 说明 |
|---|---|---|---|
| Type-1（裸金属） | hypervisor 直接接管硬件 | ESXi、Xen、Hyper-V（微内核 + root partition） | 经典定义 |
| Type-2（宿主式） | 跑在宿主 OS 之上 | VirtualBox、VMware Workstation | 面向桌面 |
| **KVM 型** | Linux 内核内的 hypervisor 层 + 用户态 VMM | KVM + QEMU/Firecracker | **既不是 Type-1 也不是 Type-2** |
| 混合微内核型 | 微内核 + 特权分区 | Hyper-V（root/child partition）、Xen（dom0） | 设备驱动与策略在特权分区 |
| **云上卸载型** | 专用卡 + 极简 hypervisor | AWS Nitro、Azure Boost、阿里云神龙、华为云擎天 | 2017 年后的主流 |

**KVM 到底算哪一类**：KVM 官方 FAQ 的表述是 "KVM is part of Linux and uses the regular Linux scheduler and memory management"，且 "QEMU uses emulation; KVM uses processor extensions (HVM)"。按"是否接管整机"它不是 Type-1，按"是否依赖宿主用户态模拟器"它不是 Type-2。工程上应表述为"**Linux 内核内的 hypervisor 层 + QEMU 用户态 VMM**"。

**术语边界**：hypervisor = CPU/内存分区隔离（KVM 模块、Microsoft Hypervisor、Xen 本体）；VMM = 设备模型与生命周期（QEMU / Firecracker / CubeHypervisor）；容器运行时 = OCI 生命周期（runc / runsc）；microVM monitor = 极简 VMM。三处常见误称：把 QEMU 叫 hypervisor、把 gVisor 叫 VM（官方明确否认）、把 Kata/runV 叫纯容器。

### 二、传统 hypervisor 厂商与产品

| 厂商 | 产品 | 底层与要点 |
|---|---|---|
| **VMware（Broadcom）** | ESXi / vSphere / Workstation / Fusion / vSAN / NSX | 2023-11 收购完成 → 2023 年底改**按核订阅**、下架永久授权、打包 VCF/VVF → **2025-05 SPD 要求每 180 天提交合规报告**（逾期 270 天后封锁 VCF 管理面）。永久授权"封顶 5.x"；vCenter/Aria 管理组件**仅限 VCF 内使用**；云上使用仅限 MaaS（Azure VMware Solution / Google Cloud VMware Engine / Oracle Cloud VMware Solution） |
| **Microsoft** | Hyper-V、Windows Hypervisor Platform、WSL2、Windows Sandbox、VBS/HVCI、Azure | 微内核 + root partition（管理栈、硬件直访）+ child partition；**VMBus**（VSP↔VSC）+ **Enlightened I/O**（在 VMBus 上直跑 SCSI，绕过设备模拟）；**SLAT 是 Windows Server 2016+ 的硬性要求**。同一 hypervisor 复用为 WSL2 的轻量 utility VM、Windows Sandbox 与 VBS/HVCI |
| **Oracle** | VirtualBox、Oracle Linux KVM | **VirtualBox 的"KVM 后端"需要精确表述**：7.2.2（2025-09-10）changelog 为 "Use KVM APIs on kernel 6.16.0 and newer for **acquiring/releasing VT-x**"。背景是 Linux 6.16 把 `kvm.enable_virt_at_load` 改为加载即占用 VT-x，导致其他 hypervisor 拿不到 VMX root。**它只是借用 KVM 的 VT-x 占用接口，不是改用 KVM 引擎** |
| **Xen 生态** | XCP-ng（Vates）、Xen Project | **XCP-ng 8.3 是 8.x 末版**；**9.0 是全新平台，"not based on Citrix XenServer at all"**。商业订阅按**物理主机数**计价（Vates VMS）；旧版 Xen Orchestra Starter/Enterprise 订阅 2026-07-31 退役（开源版不受影响） |
| **QEMU** | QEMU（TCG 模拟 / KVM 加速） | 定位是 **VMM 而非 hypervisor**；纯 TCG 模式是模拟器 |
| **Proxmox** | Proxmox VE 9.0（2025-08-05） | Debian 13 基础、KVM + LXC 双栈、AGPLv3、订阅 EUR 115/年/CPU 起 |

### 三、云厂商自研卸载架构（2017 年后的主线）

| 厂商 | 架构 | 卸载了什么 | 备注 |
|---|---|---|---|
| **AWS** | **Nitro System** | **Nitro Cards**（Annapurna SoC；主卡 Controller 是硬件信任根与对 EC2/EBS/VPC 控制面的唯一网关；VPC/EBS/NVMe 专用卡做硬件加解密，近三代 VPC 卡可透明 AES-256-GCM 加密实例间流量）+ **Nitro Security Chip**（拦截所有固件写、控制主 CPU/BMC 复位引脚，**裸金属模式下替代 hypervisor 保护固件**）+ **Nitro Hypervisor**（无网络栈、无文件系统、无外设驱动、无 shell，支持整机在线热更新） | 官方原文称 hypervisor 现为 "an **optional discrete component**"（这是裸金属实例的技术前提）；**"Nitro Hypervisor 基于 KVM"未在官方白皮书中出现** → 广泛报道但**厂商未证实** |
| **AWS（轻量化）** | **Firecracker** | Rust 实现、基于 KVM、仅 5 个模拟设备（virtio-net/block/vsock、串口、最小键盘控制器） | Lambda / Fargate 底座 |
| **Microsoft** | **Azure Boost** | 网络与存储处理卸载到**可编程 FPGA + 自研 MANA 网卡**；网络 200 Gbps 起；远端盘 14 GB/s / 75 万 IOPS，本地盘 36 GB/s / 660 万 IOPS；**Cerberus 芯片作独立硬件信任根**（NIST 800-193）；官方称"**Rust 为所有新代码首选语言**" | |
| **阿里云** | **神龙 X-Dragon** | MOC 卡 + X-Dragon Hypervisor + 服务器硬件架构；2017-10 首款产品、2018-05 公开架构；弹性裸金属 | 支持 x86/ARM/Power/国产 CPU；官方宣传"无任何虚拟化开销"、兼容第三方 Hypervisor；**硬限制：官方限制页写明仅弹性裸金属服务器与超级计算集群支持二次虚拟化，其他规格族不支持安装虚拟化软件与二次虚拟化**；后续代际与 MOC 最新规格未证实 |
| **华为云** | **擎天 QingTian** | QingTian Cards（专用 ASIC、独立供电、硬件信任根、IO 加解密）+ 精简 QingTian Hypervisor | 白皮书 2025-10 更新；2017 年首发实例；一套架构同时支持虚机/裸机/容器；官方称服务器资源 **100%** 呈现给租户 |
| **腾讯云** | **CubeSandbox** | rust-vmm + KVM；CubeHypervisor = Cloud Hypervisor fork；Apache 2.0 | 面向 AI Agent 沙箱，见 `docs/rust-kunpeng/cubesandbox-rust-analysis.md` |
| **Google Cloud** | KVM（自研组件未证实）+ **gVisor** | gVisor 是**用户态应用内核**（OCI 运行时 `runsc`），官方明确否认它是 VM、seccomp 过滤器或 AppArmor 包装 | 另有 Confidential VM、Shielded VM、Confidential GKE Nodes、Google Cloud VMware Engine |

**演进主线（六阶段）**：全虚拟化 → virtio 半虚拟化 → 硬件辅助 + SR-IOV 直通 → **DPU/IPU 全卸载** → microVM 轻量化 → 机密计算。
**分水岭是 2017 年**：AWS Nitro/C5、阿里云神龙、华为云擎天在同一时期出现了同一种"**专用卡 + 极简 hypervisor**"结构——这不是巧合，而是硬件虚拟化开销已被压到个位数百分比后，成本中心转移到 I/O 与中断路径的必然结果。

### 四、沙箱：按隔离边界分层

| 档 | 隔离边界 | 代表 | 关键代价 |
|---|---|---|---|
| ① 硬件 TCB / 机密计算 | CPU 硬件 + 受信固件 | Intel SGX / TDX、AMD SEV-SNP、Arm TrustZone / CCA | TDX 的 PAMT 在模块初始化时静态占用约 **1/256 系统内存**（约 0.4%，Dynamic PAMT 可降至约 0.004%） |
| ② 硬件辅助虚拟机级 | **独立 guest 内核** | Firecracker、Cloud Hypervisor、crosvm、**StratoVirt**、CubeSandbox、Kata | 启动与内存开销，见下表 |
| ③ 用户态内核 | 源码层拦截 syscall | **gVisor**（`runsc`） | syscall 结构性开销，强依赖平台 |
| ④ OS 级隔离 | 共享宿主内核 | namespaces + cgroups + capabilities + seccomp-bpf + Landlock + AppArmor/SELinux、bubblewrap | 隔离强度**不可能超过宿主内核自身漏洞状况** |
| ⑤ 应用 / 语言级 | 进程内或运行时内 | 浏览器渲染进程沙箱、WASM（Wasmtime/WAMR）、iOS/Android 应用沙箱、Windows AppContainer | **正交维度**，可叠加在任意层之上 |
| ⑥ 网络 / 数据级 | 出口策略 | 网络出口控制、只读叠加层 | 同样是正交维度 |

**有出处的量级数字**：

| 方案 | 启动 | 内存开销 | 来源与条件 |
|---|---|---|---|
| Firecracker | `InstanceStart`→guest init **≤125 ms**；VMM 自身启动 8 CPU ms（墙钟 6–60 ms，典型 12 ms） | VMM 线程 **≤5 MiB**（1 vCPU/128 MiB） | 官方 SPECIFICATION，M5D.metal（关超线程）/M6G.metal |
| StratoVirt（openEuler） | microvm 机型 **50 ms 内** | **<4 MB** | 官方；运行时系统调用数 <46 |
| CubeSandbox | README 称 **<60 ms**；**自家 benchmark 实测串行 avg 47.8 ms、50 并发 avg 276.1 ms（p95 508 ms）** | README 称 **<5 MB**；**benchmark 实测每 VM 摊销 ~21.5 MB@100 实例、~25.7 MB@1000 实例** | ⚠️ **同一厂商两份官方材料口径相差 4~5 倍，必须并列引用** |
| Docker 容器 | **~150 ms** | — | SOSP'17《My VM is Lighter (and Safer) than your Container》，同一台机器 |
| unikernel VM / LightVM | 4 ms；save ~30 ms / restore ~20 ms（标准 Xen 为 128/550 ms） | — | 同上 |
| gVisor（syscall 微基准） | — | 空容器净增约 **19.6 MiB** | 官方 `syscall.csv`：runc 1939 ns / runsc-ptrace **38219 ns（≈19.7×）** / runsc-KVM **763 ns** |

**两条读法**：
- **"VM 比容器慢一个数量级"是旧印象**。同一机器上 Docker 容器约 150 ms、unikernel VM 仅 4 ms——瓶颈原本在 XenStore 与设备创建的软件路径上，可被工程消除。
- **引用 gVisor 性能必须写明平台**。"gVisor 慢 20 倍"只在 **ptrace** 平台成立（官方自述该平台 "no way represents an ideal scenario"）；KVM 平台下同一微基准甚至快于 runc。另需注意**平台代次**：**ptrace 平台自 2023 年中已被 systrap 取代，现不再支持并将被移除**，引用旧数据时须说明。

### 五、沙箱 vs 虚拟化：对照

| 维度 | ④ OS 级（容器） | ③ 用户态内核（gVisor） | ② 硬件辅助 VM（microVM） | ① 硬件 TCB（机密计算） |
|---|---|---|---|---|
| 隔离边界 | 共享宿主内核 | 内核不共享，但不靠硬件虚拟化 | **独立 guest 内核** | 硬件 + 受信固件 |
| 单实例内存开销 | ≈0 | 约 +20 MiB | 约 5–25 MiB | 约 0.4% 系统内存（TDX 静态 PAMT） |
| 冷启动 | ~10–150 ms | 中等 | 50–125 ms（microVM）；Kata 取决于所选 hypervisor | 更高 |
| 稳态性能开销 | 低 | ptrace 平台极高 / KVM 平台低 | 低（CPU 密集个位数 %） | 未证实 |
| 逃逸难度 | **最低**（见下） | 中 | 高 | 最高（但信任根被攻破过） |
| 异构内核/OS | ❌ | ❌ | ✅ | ✅ |

**"容器 = 沙箱"在工程上是错的——而且这不是本笔记的推断，是标准机构与生态自己的定性**：

- **NIST SP 800-190 §3.5.2**：共享内核 "invariably results in a **larger inter-object attack surface than seen with hypervisors**"，且容器运行时提供的隔离级别 "**not as high as that provided by hypervisors**"。
- **Kubernetes 官方多租户文档**：容器 "offer a **weaker isolation boundary than virtual machines**"，并指出容器是共享内核上的进程、挂载宿主 `/sys` 与 `/proc`；**"运行不可信代码"需要 VM 或用户态内核沙箱**。
- **Docker Engine 安全文档**：默认 capabilities 与挂载 "may provide **incomplete isolation, either independently, or when used in combination with kernel vulnerabilities**"。
- **gVisor 安全模型**：**"A sandbox is not a substitute for a secure architecture."**

有 NVD 原文佐证的逃逸案例（**不是孤例**——2024-01-31 的 "Leaky Vessels" 是一个 runc + BuildKit 的批次披露）：

| CVE | CVSS | 机制 |
|---|---|---|
| CVE-2019-5736 | 8.6 | 容器内 root **覆写宿主 runc** 二进制 → 取得宿主 root |
| CVE-2024-23652（BuildKit） | **10.0** | `RUN --mount` 可**删除宿主文件** |
| CVE-2024-23653（BuildKit） | 9.8 | API 可请求高权限容器 |
| CVE-2019-14271 | 9.8 | `docker cp` 在 chroot 内经 nsswitch 加载库 → 代码注入 |
| CVE-2021-25741 | 8.8 | kubelet **subPath** 符号链接交换 → 读写卷外/宿主文件 |
| CVE-2024-21626 | 8.6 | fd 泄漏 → 工作目录落在**宿主文件系统命名空间** |
| CVE-2022-0492 | 7.8 | cgroup v1 `release_agent` 提权并 "bypass the namespace isolation"（已入 CISA KEV） |
| CVE-2020-15257 | 5.2 | containerd shim 的 abstract UDS 只校验 `euid=0` |

**seccomp 与 Landlock 只缩小攻击面，不能兜底——内核官方文档就是这么写的**：

- **kernel.org seccomp 文档**原文：**"System call filtering isn't a sandbox. It provides a clearly defined mechanism for minimizing the exposed kernel surface."** 同一文档还明确：BPF **不能解引用指针**（`struct seccomp_data` 只含寄存器参数值），且 **`SECCOMP_RET_TRACE` 可被 ptracer 用来逃逸**（"seccomp-based sandboxes MUST NOT allow use of ptrace … without extreme care; ptracers can use this mechanism to escape"）。
- **io_uring 让 seccomp 完全失明**：LWN《Task-level io_uring restrictions》指出 seccomp 对 io_uring 提交的操作"no visibility into — and thus no way to control"，**放行 `io_uring_setup`/`enter` 几乎等于放弃沙箱**。这也是 gVisor **默认禁用 io_uring** 的原因。
- **seccomp 自身也被绕过过**：CVE-2026-89603（CVSS 8.4，`SECCOMP_FILTER_FLAG_TSYNC` 与 ptrace 停驻的竞态使新过滤器被静默绕过）、CVE-2022-30594（`PTRACE_SEIZE` 绕过 `PT_SUSPEND_SECCOMP`）。
- **开销不可忽略**：LWN 实测 6 条 BPF 的 deny-open 过滤器使 `getppid()` 多耗 **25%**（JIT 关）/ **约 15%**（JIT 开）。
- **Landlock 官方**列出当前无法限制的 syscall（`chdir`/`stat`/`flock`/`chmod`/`chown`/`setxattr`/`utime`/`fcntl`/`access`）；crosvm 官方 seccomp 文档亦直言 "checking the contents of pointers isn't possible"。

**机密计算也不是"绝对安全"**：SGX 威胁模型**明确排除侧信道**；SGAxe/CacheOut（CVE-2020-0549）从 Intel 签名的 quoting enclave 提取了 attestation 私钥并伪造 quote；CacheWarp（CVE-2023-20592）与 ÆPIC Leak（CVE-2022-21233）是**架构性**（非瞬态执行）缺陷；BadRAM（IEEE S&P'25）仅需物理接触内存条 SPD 芯片即可攻破 SEV-SNP 的 attestation。SEV 官方把可用性攻击、侧信道、物理攻击、信任锚被攻破列为 out of scope。**做 Agent 沙箱时选 ① 档通常是合规驱动，而非安全驱动。**

**② 档虚拟机自身也被逃逸过——这正是"设备模型越小越好"的由来**：

| CVE | 机制 |
|---|---|
| Xen CVE-2007-4993 | `pygrub` 处理 guest 提供的 `grub.conf` 时把内容用于 exec → guest 内高权限用户在 **domain 0** 执行任意命令 |
| **VENOM CVE-2015-3456** | QEMU **软盘控制器**命令越界写 → 影响 Xen 与 KVM，可致宿主任意代码执行（Firecracker 只留 3 个 virtio 设备的直接动机） |
| CVE-2019-14378 | libslirp `ip_reass` 首分片处理错误 → 堆溢出（QEMU SLiRP 用户态网络） |
| KVM CVE-2021-22543 | 对 `VM_IO\|VM_PFNMAP` vma 处理不当绕过只读检查 → 能控制 VM 的用户可读写随机内存页并本地提权 |

**microVM 逃逸的表述纪律（容易写错，必须谨慎）**：NVD 关键词检索 Firecracker 只得到两个**宿主侧 DoS**（CVE-2020-27174 串口缓冲无上限增长、CVE-2020-16843 网络栈重入冻结），**没有公开的 guest→host 逃逸 CVE**。但这不等于安全：**硬件侧信道不认 microVM 边界**——Meltdown（CVE-2017-5754）与 Spectre（CVE-2017-5753/5715）官方说明即承认"it might be possible to steal data from other customers"；L1TF（CVE-2018-3646）可经 terminal page fault + 侧信道读 L1；Rowhammer 借内存去重（KSM）**翻转同宿主其它 VM 的页内位**并以此攻破 OpenSSH 公钥认证（Flip Feng Shui, USENIX Security 2016）。
→ **正确表述是**："尚无公开的 guest→host 逃逸 CVE，但硬件侧信道不受 microVM 边界保护"。**不要写成"microVM 不会被逃逸"，也不要写成"microVM 也会被逃逸"**——两者都超出证据。

### 六、AI Agent 沙箱

**为什么需要**：不可信代码执行、多租户、资源限额、网络出口控制、快照与回滚。

**性能诉求的排序与传统云原生不同**：传统优化稳态吞吐；Agent 沙箱优化**启动延迟 / 并发实例数 / pause-resume 时间 / 内存占用**。snapshot 与 fork 从"锦上添花"变成一等公民。

| 方案 | 形态 | 备注 |
|---|---|---|
| E2B | 云服务 | 公开数字（~150 ms / ~80 ms）**无完整测试条件** |
| AgentENV | 分布式平台（Firecracker + 内存 CoW + fork） | 官方称快照启动 <50 ms、pause <100 ms，已用于 Kimi K3 的 agentic RL 训练；**`≤16` 子沙箱与 `9.6×` 的测量条件未见公开说明** |
| CubeSandbox | 单节点（Cloud Hypervisor fork + XFS reflink） | 唯一条件完整、可复现的公开 benchmark；README 与 benchmark 口径不一致（见上表） |
| Daytona | **默认为 Linux 容器而非 microVM** | 其容器沙箱**不支持 pause/resume 与 fork**；官方文档"dedicated kernel"营销语与"默认容器"并存，极易误读 |
| microsandbox | microVM | 官方称 <100 ms，仅注"M1 机器"；每实例内存未给出 |
| **DSH 自身** | **④ 档 same-world confinement** | 见下 |

**DSH 沙箱的准确位置**：`@deepseek-ai/dsh-sandbox` README 原文承认进程 "still shares the host kernel and filesystem; use a container, microVM, or remote executor when the whole environment must be isolated"。三模式 `read-only`（默认）/ `workspace-write` / `danger-full-access`，无法强制时 **fail-closed 返回 `SANDBOX_UNAVAILABLE`**；runner 链为 Linux `bwrap → Landlock`、macOS Seatbelt、Windows ACL。**结构性缺口是网络出口**：bwrap profile 只有 `--unshare-pid`、**没有 `--unshare-net`**，Landlock 授权只覆盖文件路径，因此**没有任何内核强制的出口控制**；出口治理退到进程级 HTTP 代理，而该包 README 自述 **E2B SDK 与 OTLP exporter 因自带 transport 绕过了代理**。对照 Claude Code：Linux 上用 bubblewrap + **socat 把流量中继到沙箱代理以做域名级网络隔离** + 可选 seccomp——这是 DSH 最值得补、且有现成参考的一块。

**语气上要注意**：`@deepseek-ai/dsh-sandbox` 的 README 把 container 与 microVM 并列为"隔离整个环境"的选项，但**二者安全上并不等价**——这不是偏好问题，NIST SP 800-190 与 Kubernetes 官方文档给出的是同一判断（见第五节开头）。

## 延伸

### 三句话记住沙箱的边界

1. **NIST SP 800-190**：共享内核 "invariably results in a larger inter-object attack surface than seen with hypervisors"，容器隔离 "not as high as that provided by hypervisors"。
2. **kernel.org**：**"System call filtering isn't a sandbox."** 它只做"minimizing the exposed kernel surface"；而且 **io_uring 让 seccomp 完全失明**。
3. **microVM**：尚无公开的 guest→host 逃逸 CVE，但 **Meltdown/Spectre/L1TF/Rowhammer 这些硬件侧信道不认 microVM 边界**——沙箱能防的是空间维度（谁能访问谁的内存与文件），硬件侧信道在另一个维度上。

### 常见误区（逐条有据）

| 误区 | 事实 |
|---|---|
| 「容器就是沙箱」 | **NIST SP 800-190**：共享内核 "invariably results in a larger inter-object attack surface than seen with hypervisors"，容器隔离 "not as high as that provided by hypervisors"；K8s 官方亦称容器是 "weaker isolation boundary"；且已有成批逃逸 CVE（含 BuildKit CVE-2024-23652，CVSS 10.0） |
| 「gVisor 一定慢」 | 平台决定一切：ptrace 约 19.7×，KVM 平台下甚至快于 runc；且 **ptrace 平台已被 systrap 取代、现不再支持并将移除** |
| 「gVisor 兼容性无忧」 | 官方列出未实现项：沙箱内 cgroup **只记账、不强制限额**；不支持 fat32/ext3/ext4 块设备挂载；**io_uring 默认禁用**；**沙箱内跑 KVM 不受支持**；GPU 需 `--nvproxy` 且严格匹配驱动版本 |
| 「seccomp 能挡住内核漏洞」 | 内核官方文档直接否定：**"System call filtering isn't a sandbox."** 它只是 "minimizing the exposed kernel surface"；BPF 不能解引用指针；**io_uring 让 seccomp 完全失明**；seccomp 自身也有被绕过记录（CVE-2026-89603、CVE-2022-30594） |
| 「WASM 天生安全」 | 有真实逃逸 CVE：CVE-2023-26489（9.9，越界约 34 GB）、CVE-2026-34971（7.8，NVD 明确称 sandbox escape）、CVE-2024-51745（**10.0**，未拦截 `COM¹`/`LPT¹` 等上标数字设备名）；USENIX Security'20 论文证明 wasm 内可端到端完成利用 |
| 「microVM 就安全，不需要再加固」 | 尚无公开的 guest→host 逃逸 CVE，但**硬件侧信道（Meltdown/Spectre/L1TF/Rowhammer）不认 microVM 边界**；VMM 与宿主内核仍在 TCB 内，Firecracker 自身仍配 jailer（seccomp/cgroup）作第二道防线 |
| 「用了机密计算就绝对安全」 | 信任根被攻破过（CVE-2020-0549 提取 attestation 私钥并伪造 quote、BadRAM 攻破 SEV-SNP attestation）；侧信道通常在威胁模型之外 |
| 「有沙箱就不需要出口控制与审计」 | 沙箱管"能碰什么"，出口管"能发什么"；DSH 的缺口恰在此处 |

### 与本仓库存量笔记的关系

- CubeSandbox / AgentENV 属 ② 档"硬件辅助虚拟机级"，是虚拟化的**消费者**：它们不实现虚拟化，而是消费 KVM。映射细节见 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` 与 `research/virtualization/07-existing-notes-map.md`。
- **microVM 的快照/fork 是 hypervisor 架构的直接红利**：KVM 把 guest 物理内存做成 VMM 进程的普通用户态映射，于是内存快照退化为对进程地址空间做页级 CoW。**CubeSandbox 的 CoW 在文件系统层（XFS reflink），AgentENV 的 CoW 在虚拟化层（内存）**——这是两条技术路线，不是同一个东西的两种实现。

## Q&A

**Q：KVM 是 Type-1 还是 Type-2？**
都不是。它是"Linux 内核内的 hypervisor 层 + QEMU 用户态 VMM"。云上卸载型架构（Nitro/神龙/擎天）同样落不进传统二分法。

**Q：为什么 2017 年成了一个分水岭？**
因为硬件虚拟化的 CPU 与内存开销此时已被 EPT/NPT 压到个位数百分比，剩下的成本集中在 I/O 与中断路径。把网络/存储卸载到专用卡，成为比继续优化 hypervisor 更划算的选择。三家云在同一时期独立走到"卡 + 极简 hypervisor"。

**Q：Docker 和 microVM 的启动时间差多少？**
没有数量级差距。同一台机器上 Docker 容器约 150 ms，unikernel VM 约 4 ms，microVM 约 50–125 ms。差距主要来自设备模型与镜像加载路径，不是虚拟化本身。

**Q：要跑 AI Agent 的不可信代码，该选哪一档？**
先看是否需要独立内核：需要就选 ② 档（Firecracker/StratoVirt/CubeSandbox），并额外确认**网络出口是否内核强制**；不需要独立内核但要不信任内核漏洞，选 ③ 档（gVisor，且务必跑在 KVM 平台而非 ptrace）；纯容器只在同租户可信代码场景可接受。合规要求才考虑 ① 档。

**Q：为什么引用 Agent 沙箱的性能数字要特别小心？**
该赛道数字普遍缺少测试条件，且**同一厂商的 README 与 benchmark 可以相差 4~5 倍**（CubeSandbox 实例）。引用时必须并列口径并注明条件。

## 参考资料

访问日期均为 **2026-09-16**。

1. KVM 官方 FAQ（"KVM is part of Linux…"） — https://www.linux-kvm.org/page/FAQ
2. Microsoft Learn：Hyper-V 架构（VMBus / Enlightened I/O / SLAT） — https://learn.microsoft.com/virtualization/hyper-v-on-windows/
3. Microsoft Learn：Azure Boost（FPGA + MANA、Cerberus 信任根） — https://learn.microsoft.com/azure/azure-boost/
4. Microsoft Learn：Azure 机密计算机型（SEV-SNP / TDX） — https://learn.microsoft.com/azure/confidential-computing/
5. AWS：Nitro System 白皮书与《Use nested virtualization to run hypervisors in Amazon EC2 instances》 — https://docs.aws.amazon.com/ec2/ 与 https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html
6. Firecracker 官网与 SPECIFICATION — https://firecracker-microvm.github.io/
7. gVisor 官方性能与密度文档（syscall.csv、平台说明） — https://gvisor.dev/docs/architecture_guide/performance/
8. SOSP'17《My VM is Lighter (and Safer) than your Container》(LightVM) — https://dl.acm.org/doi/10.1145/3132747.3132763
9. Kata Containers 官方架构文档 — https://github.com/kata-containers/kata-containers/blob/main/docs/design/architecture.md
10. StratoVirt 官方仓库（openEuler Rust VMM） — https://github.com/openeuler-mirror/stratovirt
11. 腾讯云 CubeSandbox 官方 README 与 benchmark 报告 — https://github.com/TencentCloud/CubeSandbox
12. 阿里云 ACK 安全容器文档、神龙 X-Dragon 架构说明 — https://www.alibabacloud.com/help/ack
13. 华为云擎天 QingTian 架构白皮书、鲲鹏 BoostVirt 文档 — https://www.hikunpeng.com/document/
14. XCP-ng 官方博客（8.3 为 8.x 末版、9.0 新平台）与 Vates 订阅说明 — https://xcp-ng.org/blog/
15. Proxmox VE 9.0 发布说明 — https://www.proxmox.com/en/news
16. NVD：CVE-2019-5736、CVE-2022-0492、CVE-2024-21626、CVE-2020-0549、CVE-2023-20592、CVE-2022-21233 — https://nvd.nist.gov/
17. Landlock 官方文档（当前无法限制的 syscall 列表）与 crosvm seccomp 文档 — https://docs.kernel.org/userspace-api/landlock.html
18. `@deepseek-ai/dsh-sandbox` 与 `@deepseek-ai/dsh-http-proxy` 包内 README（只读检视） — 本机 DSH 安装目录
19. 本仓库中间调研稿：`research/virtualization/05-vendors-and-products.md`、`06-sandboxes.md`、`07-existing-notes-map.md`
20. NIST SP 800-190《Application Container Security Guide》§3.5.2（共享内核攻击面与隔离强度定性） — https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-190.pdf ｜ ⚠️ 该文档为 PDF，本环境无法抓取原文，**引文由分册调研取证、未逐字复核**
21. Kubernetes 官方《Multi-tenancy》（容器隔离弱于虚拟机） — https://kubernetes.io/docs/concepts/security/multi-tenancy/
22. Linux 内核《Seccomp BPF》文档（**"System call filtering isn't a sandbox."**、BPF 不解引用指针、ptrace 逃逸警告） — https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html ｜ ✅ 本次已抓取原文逐字核实
23. LWN《Task-level io_uring restrictions》（io_uring 使 seccomp 失去可见性） — https://lwn.net/Articles/1054225/
24. gVisor 官方《Security Model》《Compatibility》《Platforms》 — https://gvisor.dev/docs/architecture_guide/security/
25. Lehmann et al., *Everything Old is New Again: Binary Security of WebAssembly*, USENIX Security 2020 — https://www.usenix.org/conference/usenixsecurity20/presentation/lehmann
26. Agache et al., *Firecracker: Lightweight Virtualization for Serverless Applications*, NSDI 2020 — https://www.usenix.org/conference/nsdi20/presentation/agache
27. Flip Feng Shui（Rowhammer + KSM 跨 VM 内存位翻转）, USENIX Security 2016；L1TF CVE-2018-3646；Meltdown CVE-2017-5754 / Spectre CVE-2017-5753、CVE-2017-5715
