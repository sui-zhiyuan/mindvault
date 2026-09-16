# 硬件虚拟化：发展过程、多层方案与硬件能力依赖

> 更新：2026-09-16

## 简介

硬件虚拟化指 **CPU 在架构层面新增一套虚拟化模式与陷入机制**，使"陷入—模拟"由硬件完成，从而让虚拟机监视器（VMM）不必再靠二进制翻译或修改 guest 内核来维持隔离。它解决的是 1990 年代 x86 无法被高效虚拟化的问题：当时只能靠软件绕行，代价集中在特权指令陷阱与影子页表同步上，页表与进程创建密集的负载可慢数倍到十几倍。2005–2008 年 Intel VT-x / AMD SVM 与 EPT / NPT 相继落地后，虚拟化开销降到个位数百分比，虚拟化从此成为云与多租户的默认底座，并派生出两个分支：**多层化**（嵌套虚拟化）与**轻量化**（microVM 沙箱）。

前置知识：需要了解页表、特权级与中断的基本概念。
相关：[厂商产品与沙箱全景](vendors-and-sandboxes.md)｜本仓库既有笔记 `docs/rust-kunpeng/cubesandbox-rust-analysis.md`

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| VMM / hypervisor | Virtual Machine Monitor | 负责 CPU/内存分区与隔离的软件层；KVM 是内核模块，QEMU/Firecracker 是用户态 VMM |
| VMCS / VMCB | Virtual Machine Control Structure / Block | Intel VMX 与 AMD SVM 的 VM 状态区，保存 guest/host 状态与陷入控制位 |
| EPT / NPT | Extended / Nested Page Table | Intel / AMD 的硬件二级地址翻译（GPA→HPA） |
| Stage-2 | Stage-2 translation | ARM 的对应机制，`VTTBR_EL2` + `VTCR_EL2` |
| HVM | Hardware Virtual Machine | 依赖硬件虚拟化扩展运行的 guest，与半虚拟化相对 |
| PV | Paravirtualization | 半虚拟化：guest 内核配合修改，以换取更低开销 |
| BT | Binary Translation | 二进制翻译：改写 guest 特权指令以插入陷阱 |
| VHE | Virtualization Host Extensions | ARMv8.1 起，host 内核可直接运行在 EL2，免除寄存器跳板 |
| FEAT_NV2 | Nested Virtualization 2 | ARMv8.4 的硬件嵌套扩展（含 `VNCR_EL2`），上游 KVM 唯一支持的嵌套形态 |
| IOMMU / SMMU | I/O Memory Management Unit | 设备侧 DMA 翻译与隔离；x86 为 VT-d / AMD-Vi，ARM 为 SMMU |

## 核心内容

### 一、发展过程：从软件绕行到硬件补位

| 时期 | 里程碑 | 关键技术 | 解决了什么 | 遗留问题 |
|---|---|---|---|---|
| 1960s–70s | IBM CP-40(1967)/CP-67/CP/CMS、VM/370(1972) | 大型机原生虚拟化 | 多 OS 共享一台机器 | 与商品化 x86 无关 |
| 1974 | Popek & Goldberg《Formal Requirements for Virtualizable Third Generation Architectures》 | 虚拟化三条件 + 敏感指令定理 | 给出"什么架构可被虚拟化"的判据 | x86 不满足该判据 |
| 1990s | Disco(1997)、VMware Workstation/ESX(1999–2001) | 二进制翻译 + 影子页表 | 在不可虚拟化的 x86 上跑未修改 guest | 翻译与页表同步开销高 |
| 2003 | Xen 半虚拟化（SOSP'03） | 改 guest 内核 + 页表只读注册 + 共享环 I/O | CPU 密集负载开销压到"几个百分点" | 需改 guest；Windows 移植困难 |
| 2005–2006 | Intel VT-x（含 Itanium 的 VT-i 是不同技术）、AMD SVM | root/non-root 或 host/guest 模式 + VMCS/VMCB | 特权指令陷入由硬件完成，不再需要翻译 | **MMU 仍靠影子页表** |
| 2007–2008 | AMD NPT(Barcelona, 2007)、Intel EPT(Nehalem, 2008)、KVM 并入 Linux 主线 | 硬件二级地址翻译；VPID/ASID | 消除影子页表，guest 写页表不再陷入 | 设备与中断路径仍是瓶颈 |
| 2007–2015 | virtio(2007–08)、SR-IOV(2007+)、vhost、VFIO | 半虚拟化驱动与设备直通 | I/O 开销从"模拟"降到接近原生 | 直通需 IOMMU，牺牲可迁移性 |
| 2010–2013 | VMCS shadowing、APICv、Posted Interrupt（Haswell, 2013） | 嵌套与中断虚拟化硬件化 | 嵌套可用；中断投递免 VM-exit | 功能子集与 exit 放大仍在 |
| 2016–2021 | AMD SEV(2016) → SEV-SNP(2020–21)；Intel TDX(2023) | 内存加密 + 硬件信任根 | 把宿主移出信任边界 | 开销与生态成熟度 |
| 2017– | AWS Nitro、阿里云神龙、华为云擎天 | 专用卡 + 极简 hypervisor | 把虚拟化与 I/O 卸载出主 CPU | 各家绑定自有硬件 |
| 2018– | Firecracker / Cloud Hypervisor / StratoVirt / CubeSandbox | Rust 实现的 microVM | 秒起、快照、高密度 | 快照生态与跨架构迁移 |

### 二、什么是硬件虚拟化：一个可判据的定义

**定义**：CPU 在架构层面新增一套**低于 guest 内核的特权模式**，并提供硬件级的陷入与状态切换机制，使"陷入—模拟"由硬件完成。

| 体系 | 模式 | 状态区 | 陷入方式 |
|---|---|---|---|
| Intel x86 | VMX root / non-root | VMCS | VM entry / VM exit |
| AMD x86 | host / guest | VMCB | `VMRUN` / `#VMEXIT` |
| ARM | EL0/EL1/EL2/EL3 | 系统寄存器 | 异常级别切换 |

**理论判据（Popek & Goldberg 1974）**：等价性、资源控制、效率三条件；核心定理是"**每条敏感指令都必须是特权指令**"。x86 不满足该定理——有 17 条指令（`popf`、`iret`、`sgdt`、`sldt`、`sidt`、`smsw`、`lar`、`lsl`、`pop [seg]`、远 `call/jmp/ret`、`int N` 等）属用户态敏感却不产生陷阱。两种补救都失败：**ring compression**（把 guest 降到 ring 3）被 ring aliasing 破坏；**ring aliasing**（把 guest 提到 ring 1）又被 `pushf/popf` 等指令暴露特权级而破坏。VMware 只能放宽等价性：不支持 CPL 1/2，并忽略 `sgdt/sldt/sidt/smsw`。

**三个必须避免的误解**：
- 硬件虚拟化 **不是**"虚拟化必须有硬件"——半虚拟化与二进制翻译都能工作，只是代价不同。
- 硬件虚拟化 **不等于** VM 一定比容器慢——现代硬件上 CPU 密集负载的虚拟化开销是低个位数百分比，差距主要来自内存与 I/O 路径。
- **有 VT-x 不等于快**——第一代 VT-x/SVM 在缺少 EPT/NPT 时，页表密集负载甚至**慢于成熟的二进制翻译**（见下一节）。

### 三、软件虚拟化的性能损失：必须分档回答

"软件虚拟化损失多少"**没有单一答案**。下表按负载类型分档，数字均标注来源与测试条件。

| 负载类型 | 软件虚拟化代价 | 典型数字与来源 |
|---|---|---|
| CPU 密集（译码块可缓存） | ≈0–5% | Xen 半虚拟化 SPEC INT2000 = 567 vs 原生 567（≈0%）；Linux 2.4.21 内核编译 271s vs 263s，论文自述 "mere 3% overhead" |
| 半虚拟化 I/O（virtio 式共享环） | ≈0–15% | Xen `ttcp` MTU1500 收发 897/897 Mb/s（≈原生） |
| 全模拟 I/O（模拟网卡/IDE） | **30%–90%** | VMware Workstation 3.2 模拟 `pcnet32` 发送仅 291 Mb/s（**−68%**）；MTU500 时恶化到 **−83%~−90%** |
| MMU（页表操作密集） | **3–18×** | `pgfault`：原生 1093 / BT 3927（3.6×）/ 第一代硬件 11242 cycles；`ptemod`：原生 1 次 store / BT 391 / 第一代硬件 12733（≈30× BT） |
| 进程创建（fork/wait 密集） | **6–18×** | `forkwait`（fork+waitpid×40000，P4）：原生 6.0s / BT 36.9s（6.1×）/ 第一代硬件 **106.4s（17.7×）** |
| 每次系统调用（BT 路径） | 约 +2000 cycles | ASPLOS 2006 对 BT 与硬件路径的对照 |

**关键反直觉结论**：**第一代硬件辅助并不自动更快**。Adams & Agesen（ASPLOS 2006）同一负载下，内核编译原生 265s、二进制翻译 393s、第一代 VT-x **484s**；Linux Apache 得分软件 45% vs 硬件 38%（原生 100%）。原因正是 EPT/NPT 缺席——MMU 仍靠影子页表，而 VM exit 本身在当时非常昂贵（VM entry 2409 cycles @P4 → 937 @Core 2）。这直接催生了 EPT/NPT。

**VM exit 成本演进**：page-fault vmexit 从 Prescott 1926 cycles 降至 Broadwell 531，是虚拟化开销长期下降的主线之一。

### 四、多层虚拟化：先辨析"多层"的三种含义

| 含义 | 层次 | 隔离边界 | 代价性质 |
|---|---|---|---|
| ① **嵌套虚拟化** | L0 hypervisor / L1 guest hypervisor / L2 guest | 硬件虚拟化扩展 + L0 的 VMCS/VMCB 合并逻辑 | **只有它会引入 exit 放大** |
| ② **多级地址翻译** | GVA→GPA→HPA，设备侧再叠 IOMMU/SMMU | MMU 与 IOMMU 的硬件页表 | 页表走查次数与 TLB 语义变化 |
| ③ **多级虚拟化栈** | KVM → microVM → 容器 → 语言沙箱 | 各层软件策略逐层收窄 | syscall 路径的软件过滤，**不产生 VM exit** |

三者正交、可叠加。把"VM 里跑容器"叫两层虚拟化没错，但它和"VM 里跑 hypervisor"的代价完全不是一个量级。

### 五、嵌套虚拟化：方案族与它解决的问题

**根本矛盾**：CPU 只有一套 VMCS/VMCB，内存只有二级硬件翻译，而 L1 也需要一套来跑 L2。L0 必须做**字段合并**（vmcs12 → vmcs02）或**影子**（shadow EPT / shadow page table）。所有性能与功能子集问题都由此推导。

| 方案 | 机制 | 硬件前提 | 代价 |
|---|---|---|---|
| Intel VMCS shadowing | L1 的 `VMREAD/VMWRITE` 读写 `VMCS link pointer` 指向的 shadow VMCS，不再 exit | Haswell(2013)+ | 消除最贵的一类 exit：Turtles 实测切换开销 13,000 → 2,000（**−84.6%**） |
| Intel nested EPT | L2 两级翻译硬件直通，L0 仅在 L1 的 EPT 页被改时介入 | EPT(2008+) | TLB 未命中时二维走查加长 |
| AMD nSVM + nested NPT | 与 Intel 对偶；L0 需正确清除 VMCB clean bits | AMD-V + NPT | 同构的 exit 放大 |
| 纯软件模拟 | 软件影子维护多层 VMCS + multi-dimensional paging 把 3 级压进 2 级 | 无特殊要求 | 常见负载 6–8%；朴素做法"至少慢 3 倍" |
| 裸金属 + DPU | 取消 L0 或把 I/O 卸载到卡上，让客户 hypervisor 当 L0 | AWS Nitro / 神龙 MOC / 擎天卡 | 无 exit 放大，但绑定厂商硬件 |

**Turtles（OSDI 2010）实测**（KVM 同时作 L0/L1、x86 无嵌套硬件支持）：相对单层虚拟化，kernbench 开销 **14.5%**、SPECjbb 得分 **−7.82%**；扣除 `VMREAD/VMWRITE` 后为 **10.3% / 6.3%**；**L0 自身 CPU 占用从 2.28% 升到 5.17%**——"多一层"放大的是**宿主开销**。论文摘要自述常见负载可做到"**within 6-8% of single-level virtualization**"。I/O 侧代价远大于 CPU：netperf 64B 消息，裸机 900 Mb/s 时 L2 + multi-level device assignment 为 837 Mb/s（约 −7%），而 virtio-on-direct 仅 469 Mb/s（−50%）。

**解决什么问题**：云上跑 KVM/Hyper-V、Docker Desktop、WSL2、CI 里跑模拟器或测试 hypervisor、VDI 与教学实验、云上 Android 模拟器、Kata/gVisor 的分层部署。**三家云官方都把嵌套定位为开发/测试能力，并一致提供"逃逸"路径——建议性能敏感场景改用裸金属实例。**

### 六、内存虚拟化的三条路线

| 维度 | Shadow page table（软件） | EPT / NPT / Stage-2（硬件） | 半虚拟化 MMU |
|---|---|---|---|
| 翻译级数 | 压成 1 级 | 2 级 | 通常 1 级（guest 直接写页表） |
| guest 改页表 | 写保护 + 陷入同步 | 自由改，无需陷入 | 自己写 + hypercall 通知 |
| 写 CR3 | 换 shadow root + flush TLB | TLB 按 EPTP/VMID 打标，通常免 flush | 需 hypercall 换基址 |
| 主要失效场景 | 页表频繁切换/写入、`INVLPG` 风暴 | 内存超分、dirty logging、大页被写保护页打散 | guest 不可改、已有 EPT/NPT |

**EPT/NPT 路线的三个结构性痛点**：①每个写保护页都会使 memory slot 的 `->disallow_lpage` 递增，**阻止大页实例化**——dirty logging 与内存热插拔会直接削弱大页的 TLB 收益；②dirty logging 靠写保护 EPT，每次写都产生 EPT violation；③EPT 无法表示"不存在但可换出"的 GPA，**内存超分必须靠 balloon/swap 配合**。

### 七、硬件能力矩阵：能力 → 首次引入 → 作用 → 缺失代价

| 能力 | Intel | AMD | ARM | 作用 | 缺失代价 |
|---|---|---|---|---|---|
| 基本虚拟化模式 | VMX + VMCS（2005） | SVM + VMCB（2006） | EL2（ARMv8.0，ARMv7 VE 起有 32 位 HYP） | 特权指令陷入由硬件完成 | 退回二进制翻译或半虚拟化 |
| 硬件二级翻译 | **EPT（Nehalem 2008）** | **NPT（Barcelona 2007）** | **Stage-2（ARMv8.0）** | GPA→HPA 由硬件走查 | 影子页表，页表密集负载 3–10× 以上 |
| TLB 标签 | VPID（2008） | ASID（2006） | VMID | VM 切换不必全刷 TLB | 每次切换全刷 TLB，经验回退 10%–50% |
| 关分页 guest | Unrestricted Guest（Westmere 2010） | NPT 使能后即支持 | — | 实模式/`CR0.PG=0` guest 可被硬件承载 | 引导阶段需 8086 模拟，可能不可用 |
| 嵌套虚拟化 | VMCS shadowing + nested EPT（Haswell 2013） | nSVM + nested NPT | **FEAT_NV2（ARMv8.4）** | VM 里再跑 hypervisor | 只能软件模拟或不可用 |
| 中断虚拟化 | APICv + Posted Interrupt（Haswell 2013） | AVIC（Zen，2017）/ x2AVIC（Zen 4，2022） | GICv3 `ICH_LR` → GICv4 vLPI → GICv4.1 vSGI | 中断直注免 VM-exit | 中断走软件注入，高 PPS 场景成为瓶颈 |
| 增强嵌套 | VMFUNC(EPTP switching, 2013)、HLAT(VT-rp, 12 代起) | GMET | FEAT_NV2 的 `VNCR_EL2` | 免 exit 切换地址视图 / 抵抗重映射 | 每次切换 2 次 transition |
| 脏页与迁移 | EPT A/D 位（Haswell/Broadwell ⚠）、PML（Skylake 2015） | NPT A/D 位 | SMMU HTTU（设备侧） | 硬件标脏，迁移/快照迭代快 | 写保护 + `#PF`，dirty logging 变慢 |
| 设备隔离 | VT-d（2008）+ SR-IOV/ACS | AMD-Vi（2007） | SMMUv2/v3 | DMA 翻译与隔离 | **无 IOMMU 就不能安全直通**（设备可对任意物理地址 DMA） |
| PMU / 追踪 / 分区 | — | — | PMUv3 虚拟化（`MDCR_EL2`）、SPE(8.2)、TRBE(8.4)、MPAM(8.4) | 多租户可观测性与 QoS | guest 无 `perf`/剖析；无法隔离 LLC 与带宽 |
| 机密计算 | SGX、TDX（Sapphire Rapids，2023 GA） | SEV(2017) → SEV-ES → **SEV-SNP（Zen 3/EPYC 7003, 2021）** | TrustZone、RME/Arm CCA（Armv9） | 把宿主移出信任边界 | 多租户场景无法对云厂商保密 |

> ⚠️ 标注 ⚠ 的项存在来源分歧或未证实：EPT A/D 位的首发代际（QEMU 自 Broadwell 起标，Linux 2017 才加 MMU 支持）、MBEC/Bus Lock Detection 的代际、HLAT 的服务器首发代际、AMD GMET 的引入代际。

### 八、鲲鹏平台：能力边界与一条必须说清的结论

**鲲鹏 920 = ARMv8.2（HiSilicon 官方规格页明写 `Architecture: ARMv8.2`，鲲鹏社区官方页写"Armv8.2 指令集"）**。官方规格页**未列出任何虚拟化特性**——这属 ARMv8-A 服务器 SoC 的架构标配、通常不作为卖点，但意味着能力边界只能由架构版本界定。

| 能力 | 鲲鹏 920 | 依据 |
|---|---|---|
| EL2 / Stage-2 | 支持（推断；实际承载 KVM 生产负载） | 官方未列出，但 BoostVirt 文档全栈以 KVM/QEMU/libvirt 为前提 |
| VHE（ARMv8.1） | **未证实** | 官方未提；需在机器上查 `ID_AA64MMFR1_EL1.VH` 或 `dmesg` 的 "VHE mode initialized successfully" |
| **硬件嵌套虚拟化（FEAT_NV2）** | **不支持** | ARMv8.2（已证实）× 上游 KVM 只支持 FEAT_NV2/ARMv8.4（已证实） |
| GICv4.1 中断直通 / SMMU HTTU 标脏 / MPAM | **仅"鲲鹏 920 新型号处理器"** | 华为 BoostVirt 官方文档反复使用该限定语；原版 920 的 GIC/SMMU 版本未证实 |
| 直通设备热迁移 | **仅 KAE 设备支持** | 官方明确"不支持 KAE 设备以外的直通设备热迁移"；源/目的端 BIOS 与固件版本必须一致；停机目标 <200 ms |
| 软件栈 | KVM + QEMU + libvirt；**StratoVirt**（openEuler 的 Rust VMM，支持 aarch64 `virt`）；**BoostVirt**（Apache-2.0 加速套件） | openEuler / gitcode 官方仓库 |

**必须说清的结论**：**鲲鹏 920 上"在 VM 里再跑 KVM"没有硬件路径，这不是配置问题而是架构版本缺口。** 依据是两条已证实事实的组合（架构版本 ARMv8.2 + 上游只支持 ARMv8.4 的 FEAT_NV2）。可替代方案：裸金属直接部署、容器/命名空间隔离、或 guest 内跑 QEMU TCG（纯软件模拟，性能不可接受，仅用于功能验证）。

**鲲鹏 950**（华为 Connect 2025 宣布，2026 Q1）：96 核/192 线程与 192 核/384 线程两型号，官方称"安全方面新增四层隔离，成为鲲鹏首颗实现机密计算的数据中心处理器"。但其 **ARM 架构版本、是否具备 FEAT_NV2、SMMU/GIC 版本、"四层隔离"是否基于 RME——均未找到公开资料证实**，不可按 Armv9 反推。

## 延伸

### 为什么"嵌套虚拟化"在 ARM/鲲鹏上要单独提醒

x86 的 KVM 嵌套 VMX 自 **Linux 3.1（2011）** 起即可用；arm64 的完整嵌套支持**直到 2025 年才进入主线**，且只支持 **FEAT_NV2**。Marc Zyngier 在补丁说明中给出的理由（原文）是：*"No existing hardware supports it without FEAT_NV2, and the architecture is deprecating the former entirely."* 因此在 ARM 侧规划"云上跑 Docker Desktop / CI 跑 hypervisor / VDI"这类场景时，**必须先确认芯片代次是否具备 FEAT_NV2**，不能沿用 x86 的默认假设。

### microVM 与沙箱：虚拟化能力的两个衍生分支

- **多层化**：见本文第五节。
- **轻量化**：KVM 把 guest 物理内存做成 VMM 进程的普通用户态映射，于是"给运行中的 VM 做内存快照/克隆"退化为"对进程地址空间做页级 CoW"。这是 Firecracker（官方指标：启动 **<125 ms**、**150 microVM/秒/宿主**、单实例 **<5 MiB** 开销）与 CubeSandbox / AgentENV 等产品形态的前提。详见 [厂商产品与沙箱全景](vendors-and-sandboxes.md) 与 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`。

### 选型建议（本笔记的落地结论）

1. **能用裸金属/DPU 就不要用嵌套**——嵌套的代价来自 exit 放大，绕开比优化划算。
2. 必须嵌套时，x86 优先"Intel VMCS shadowing + nested EPT"或"AMD nSVM + nested NPT"，并接受功能子集；ARM 优先 ARMv8.4-NV。
3. **多级栈中的各层应落在不同资源维度上**，避免在同一维度叠三层虚拟化（VM → 容器 → 语言沙箱 是合理的；VM → hypervisor → VM 才是贵的）。
4. 讨论"虚拟化开销"时先分清是**CPU、MMU、I/O 还是中断**——四者的数量级完全不同。

## Q&A

**Q：有 VT-x 是不是就代表虚拟化很快？**
不是。第一代 VT-x/SVM 缺少 EPT/NPT 时，MMU 仍靠影子页表，页表与进程创建密集负载甚至慢于二进制翻译。真正的分水岭是 EPT/NPT，不是虚拟化模式本身。

**Q：容器和虚拟机的性能差距到底有多大？**
CPU 密集负载上现代硬件虚拟化开销是低个位数百分比，差距主要来自内存与 I/O 路径以及启动时间，而不是 CPU 指令执行。真正决定隔离强度与代价的是"是否共享宿主内核"。

**Q：为什么云厂商既支持嵌套又劝你别用？**
嵌套是开发/测试便利设施（Docker Desktop、WSL2、模拟器、测试 hypervisor）。三家云都在文档里把性能敏感场景引导到裸金属实例——即官方承认嵌套不是生产性能方案。

**Q：鲲鹏上能跑嵌套虚拟化吗？**
原版鲲鹏 920（ARMv8.2）不能，没有硬件路径；上游 KVM 的 arm64 嵌套只支持 ARMv8.4 的 FEAT_NV2，且 2025 年才进入主线。鲲鹏 950 的架构版本与嵌套能力均未见公开证实。

**Q：半虚拟化"改 guest 内核"这个缺点今天还存在吗？**
以 virtio 的形式普遍存在且被广泛接受——virtio 驱动就是"guest 配合"的现代形态，它把 I/O 从模拟（−30%~−90%）拉回到接近原生（≈0–15%），代价是要有对应驱动。

## 参考资料

访问日期均为 **2026-09-16**。

1. Popek & Goldberg, *Formal Requirements for Virtualizable Third Generation Architectures*, CACM 1974
2. Barham et al., *Xen and the Art of Virtualization*, SOSP 2003 — https://studyres.com/doc/8893844/xen-and-the-art-of-virtualization
3. Adams & Agesen, *A Comparison of Software and Hardware Techniques for x86 Virtualization*, ASPLOS 2006 — https://dl.acm.org/doi/abs/10.1145/1168919.1168860
4. Ben-Yehuda et al., *The Turtles Project: Design and Implementation of Nested Virtualization*, OSDI 2010 — https://www.usenix.org/conference/osdi10/turtles-project-design-and-implementation-nested-virtualization
5. Waldspurger, *Memory Resource Management in VMware ESX Server*, OSDI 2002
6. Linux 内核 KVM 文档《Nested VMX》《The x86 kvm shadow mmu》 — https://www.kernel.org/doc/Documentation/virtual/kvm/
7. `[PATCH v11 00/43] KVM: arm64: Nested Virtualization support (FEAT_NV2 only)`（含 Zyngier 放弃 FEAT_NV 的理由原文） — http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-November/882814.html
8. KVM/arm64 6.17 更新（含 "Nested support for FEAT_RAS and FEAT_DoubleFault2"） — https://git.zx2c4.com/wireguard-linux/log/arch/arm64/kernel?showmsg=1
9. AWS《Use nested virtualization to run hypervisors in Amazon EC2 instances》 — https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html
10. Xen Wiki《Nested Virtualization in Xen》 — https://wiki.xenproject.org/wiki/Nested_Virtualization_in_Xen
11. Firecracker 官网 — https://firecracker-microvm.github.io/
12. **HiSilicon 官方 · Kunpeng 920**（`Architecture: ARMv8.2`） — https://www.hisilicon.com/en/products/kunpeng/huawei-kunpeng/huawei-kunpeng-920
13. **鲲鹏社区官方 · 鲲鹏920处理器** — https://www.hikunpeng.com/zh/compute/kunpeng920
14. 鲲鹏 BoostVirt 文档（中断直通 / GICv4.1 超分优化 / vKAE 直通热迁移 / MPAM / vCPU 热插拔） — https://www.hikunpeng.com/document/ 与 https://gitcode.com/boostkit/boostvirt
15. StratoVirt 官方仓库（openEuler Rust VMM） — https://github.com/openeuler-mirror/stratovirt
16. 华为 Connect 2025 徐直军主题演讲（Kunpeng 950 规格与"四层隔离"表述） — https://www.huawei.com/en/news/2025/9/hc-xu-keynote-speech
17. Intel SDM Vol 3C/3D（VMX / EPT / VMCS shadowing）、AMD APM Vol 2（VMCB / NPT / AVX） — https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
18. Arm《AArch64 virtualization》与 `VNCR_EL2` 寄存器文档 — https://developer.arm.com/documentation/102142/latest/
19. 本仓库中间调研稿：`research/virtualization/01-history-and-overhead.md`、`02-multilayer-virtualization.md`、`03-hw-capabilities-x86.md`、`04-hw-capabilities-arm-kunpeng.md`、`08-verification-notes.md`
