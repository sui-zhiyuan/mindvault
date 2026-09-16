# 硬件虚拟化依赖硬件的哪些能力 —— ARM 架构 与 华为鲲鹏 (Kunpeng)

> 更新：2026-09-16
> 定位：**中间调研稿**。ARM 架构部分为可核对的架构/内核知识汇总；鲲鹏部分严格区分「已证实（官方资料）」与「未证实/推断」。凡未找到可靠公开来源的鲲鹏能力，一律写「未找到公开资料证实」，不做架构版本反推后的事实陈述。
> 相关：`docs/arm-kunpeng/kunpeng-920b-specs.md`、`docs/rust-kunpeng/cubesandbox-rust-analysis.md`

## 核心结论

1. ARM 上的硬件虚拟化不是单一开关，而是一组**按架构版本递增叠加**的可选/必选扩展：EL2 + Stage-2 翻译来自 ARMv8.0（ARMv7 VE 已有 32 位 HYP），**VHE 在 ARMv8.1**，**FEAT_NV（嵌套虚拟化）在 ARMv8.3**，**FEAT_NV2 + VNCR_EL2（低开销嵌套）在 ARMv8.4**，RAS/SPE/SVE 在 ARMv8.2，PAuth 在 ARMv8.3，MPAM/Secure EL2/TRBE 在 ARMv8.4，MTE 在 ARMv8.5。（ARM 官方 Learn the architecture / ARM ARM）
2. **没有 VHE 也能跑 KVM**（`kvm-arm.mode=nvhe`，host 内核留在 EL1、KVM 在 EL2），但每次 host↔guest 切换要做 hyp 寄存器跳板（`__kvm_hyp_vector` 手工保存/恢复），代价是切换路径变长、可维护性差。VHE（E2H=1 + TGE=1）让 host 内核直接跑在 EL2，省掉这条跳板。
3. **嵌套虚拟化的门槛是 FEAT_NV2（ARMv8.4），不是 ARMv8.1/8.2**。上游 KVM/arm64 的 NV 支持（Marc Zyngier 系列，标题即 "ARMv8.3/8.4 Nested Virtualization"）明确以 **ARMv8.4-NV（FEAT_NV2 + VNCR_EL2）** 为目标；2025 年的 v11 版本标题直接写明 **"(FEAT_NV2 only)"**，即只支持 NV2。ARMv8.2 及更早的核上做 nested，上游没有任何「纯 trap 模拟」的成品路径。
4. **GIC 虚拟化是 ARM 中断虚拟化的核心分水岭**：GICv2 靠 `GICH_*`/`GICV_*` 内存映射接口 + list registers；GICv3 改用 `ICH_*_EL2` 系统寄存器（`ICH_LR<n>_EL2`、`ICH_HCR_EL2`、`ICH_VMCR_EL2`）+ maintenance interrupt；GICv4/4.1 才做到 vLPI（乃至 vSGI）**直接注入**，省掉 VM-Exit/Entry。没有这些扩展时，中断只能由 hypervisor 捕获后用软件注入。
5. **SMMU 是 ARM 侧的 VT-d**：SMMUv2/v3 的 Stage-2 等价于 Intel VT-d 的二级翻译表（EPT/SPT），让直通设备用 GVA→IPA→PA 两级翻译；SMMUv3 用 CD/STE 表 + 命令/事件队列，比 SMMUv2 更接近 VT-d 的架构化设计。设备直通的前提是「SMMU 存在且被使能 + VFIO 把设备 stage-2 绑定到 guest」。
6. **鲲鹏 920 是 ARMv8.2（已证实）**：海思官方产品页 Key Features 明写 `Architecture: ARM v8.2`，最高 64 核/芯片、7nm、2.6/3.0 GHz、8 通道 DDR4、PCIe 4.0/CCIX/100G。鲲鹏社区同源页面写「Armv8.2 指令集」。
7. **鲲鹏 920 的硬件嵌套虚拟化：未找到任何华为官方资料证实支持**。结合 ①官方架构版本 ARMv8.2、②上游 KVM/arm64 NV 只支持 FEAT_NV2（ARMv8.4）两条已证实事实，可判定**在原版 920 上用上游 KVM 做嵌套虚拟化不可用（推断，高置信）**；官方也从未把「嵌套虚拟化」列为 920 特性。**「KVM in VM」在 920 上没有硬件路径**，替代只有裸金属、容器隔离，或 guest 内跑 QEMU TCG（纯软件模拟，性能不可接受）。
8. **鲲鹏 920 的部分虚拟化能力有「型号分层」**：华为 BoostVirt 文档反复出现限定语「**仅支持鲲鹏 920 新型号处理器**」——GICv4.1 中断直通、GICv4.1 超分优化、SMMU HTTU 硬件标脏（vKAE 直通热迁移）都只对 920 新型号开放（已证实）。**原版 920 是否具备 GICv4.1/SMMU 标脏，未找到公开资料证实。**
9. **鲲鹏侧的设备直通热迁移有硬约束**：官方明确「**不支持 KAE 设备以外的直通设备热迁移**」（已证实），且源/目的端 **BIOS 与固件版本必须一致**；vKAE 迁移依赖 SMMU 的 HTTU 标脏，停机时间目标 <200 ms。
10. **鲲鹏 950：官方已公布规格但未公布架构版本**。华为 Connect 2025 主题演讲宣布 2026 Q1 推出 **96 核/192 线程** 与 **192 核/384 线程** 两型号，支持通用计算超节点，**「安全方面新增四层隔离，成为鲲鹏首颗实现机密计算的数据中心处理器」**（已证实）。其 ARM 架构版本（是否 Armv9/RME）、是否支持硬件嵌套虚拟化：**未找到公开资料证实**。
11. **鲲鹏虚拟化软件栈的三条主线**：① KVM/QEMU/libvirt（openEuler 主线）；② **StratoVirt**——openEuler 社区的 Rust VMM，官方 README 明确支持 `aarch64` 的 `virt` 机型，Mulan PSL v2 许可；③ **BoostVirt**（鲲鹏 BoostKit 虚拟化加速套件，Apache-2.0）——覆盖中断直通、拓扑感知调度、vKAE、热迁移 KAE 压缩、vCPU/内存热插拔、MPAM、SR-IOV 直通等（均已证实）。
12. **MPAM 在鲲鹏上确实落地，但绑定了 openEuler 内核**：BoostVirt 文档的硬件要求为「鲲鹏 920 新型号处理器、鲲鹏 950 处理器」，需 `arm64.mpam` 内核参数 + `resctrl` 挂载，libvirt/QEMU 需 9.10.0/8.2.0（已证实）。鲲鹏 950 额外支持 MPAM 的 `L3MAX/L3MIN`（已证实）。这意味着该能力不是纯上游行为。
13. **热迁移与跨厂商兼容性**：鲲鹏热迁移要求同型号、BIOS/固件版本一致（已证实）；直通设备仅 KAE 可迁移（已证实）；跨 SoC/跨厂商迁移在 ARM 上受 GIC ITS/固件/非架构化外设影响，业界通常走 vDPA/软件设备而非直通（openEuler 技术白皮书提到 vDPA 面向「支持跨硬件厂商热迁移」，白皮书原文未能完整取证，**待证实**）。
14. **仓库内部一处待消解的矛盾**：`docs/arm-kunpeng/kunpeng-920b-specs.md` 记录的机器（920B，320 逻辑核 = 2×80 核×2 线程）在 flags 中含 `sve/svei8mm/svebf16`，而海思官方 920 页面只写 ARMv8.2、未提 SVE。**该实测记录与「920 = ARMv8.2 且无 SVE」的常见表述不一致**，可能因为 920B/920 新型号是不同型号。本文不据此下结论，列入未决问题。

## 术语列表

| Term | Full Name | Meaning |
|------|-----------|---------|
| EL | Exception Level | ARM 异常级别 EL0（应用）/EL1（内核）/EL2（Hypervisor）/EL3（Secure Monitor） |
| VHE | Virtualization Host Extensions | ARMv8.1 扩展，host 内核可运行在 EL2（E2H+TGE） |
| NV / NV2 | Nested Virtualization (1/2) | ARMv8.3 的 FEAT_NV / ARMv8.4 的 FEAT_NV2（含 VNCR_EL2） |
| IPA | Intermediate Physical Address | Stage-1 输出、Stage-2 输入的中间物理地址 |
| VTTBR_EL2 | Virtualization Translation Table Base Register | Stage-2 页表基址 + VMID |
| VTCR_EL2 | Virtualization Translation Control Register | Stage-2 翻译控制 |
| GIC | Generic Interrupt Controller | ARM 通用中断控制器（v2/v3/v4/v4.1） |
| LR | List Register | `ICH_LR<n>_EL2`，GICv2/v3 虚拟中断的软件投递槽 |
| vLPI / vSGI | virtual LPI / virtual SGI | GICv4 的直通虚拟中断 / GICv4.1 的直通核间中断 |
| SMMU | System MMU | ARM 的 IOMMU（v2/v3），等价 x86 VT-d |
| HTTU | Hardware Translation Table Update | SMMU 硬件标脏/更新访问位，直通设备热迁移依赖它 |
| MPAM | Memory System Resource Partitioning and Monitoring | ARMv8.4 缓存/内存带宽分区与监控 |
| RME | Realm Management Extension | Arm CCA 机密计算的架构扩展（Realm 世界） |

## ARM 架构虚拟化能力

### 1. 异常级别模型（EL0/EL1/EL2/EL3）与 AArch64/AArch32

- **EL0**：用户态；**EL1**：Guest OS / host 内核；**EL2**：Hypervisor（KVM 本体与 hyp 代码）；**EL3**：Secure Monitor（ATF/TF-A，负责 Secure/Non-secure 世界切换）。
- **AArch64**：EL0–EL3 全 64 位。EL2 是 KVM 的立足点，**没有 EL2 就没有 KVM**。
- **AArch32**：异常级别模型相同，但 EL2 需要 ARMv7 VE（Virtualization Extensions）才存在，EL3 需要 Security Extensions（TrustZone）才存在。AArch64 host 通过 `HCR_EL2.RW` 选择下级 EL（EL0/EL1）的运行态：`RW=1` 则 EL1 为 AArch64，`RW=0` 则 EL1 为 AArch32。VHE 模式下 `HCR_EL2.RW` 被强制为 1。
- **引入时间**：EL2 随 ARMv7 VE（约 2011，32 位 HYP 模式）进入 ARM；AArch64 的 EL2/EL3 随 **ARMv8.0-A（2013）**确立。
- **缺失代价**：无 EL2 → 只能全虚拟化/半虚拟化（如早期 Xen on ARM 用 EL1 域 + hypercall），或纯软件模拟；这意味着每次特权操作都要陷出。

### 2. HCR_EL2 关键控制位

`HCR_EL2` 是 EL2 的「路由与陷阱总开关」。下表位号以 Linux `arch/arm64/include/asm/kvm_arm.h` / ARM ARM 为准；不同架构版本可能把某位从 RES0 变为有效。

| 位 | 名称 | 首次失效/生效版本 | 作用 |
|----|------|------------------|------|
| bit 31 | `RW` | ARMv8.0 | 下级 EL 的运行态（1=AArch64，0=AArch32）；VHE 下强制 1 |
| bit 27 | `TGE` | ARMv8.0 | Trap General Exceptions：EL0/EL1 的通用异常全部路由到 EL2；VHE 下与 E2H 配合让 host EL0 继续正常工作 |
| bit 34 | `E2H` | **ARMv8.1 (VHE)** | EL2 Host enable：EL2 以「host 内核」语义运行，EL1 系统寄存器被别名到 EL2 |
| bit 3 | `FMO` | ARMv8.0 | 物理 FIQ 路由到 EL2 |
| bit 4 | `IMO` | ARMv8.0 | 物理 IRQ 路由到 EL2（KVM 借此在 EL2 直接注虚拟中断） |
| bit 5 | `AMO` | ARMv8.0 | 物理 SError 路由到 EL2 |
| bit 42 | `NV` | **ARMv8.3 (FEAT_NV)** | 开启嵌套虚拟化：Guest hypervisor 的 EL2 状态被影子化 |
| bit 43 | `NV1` | ARMv8.4 时代细化（E2H0 语义） | 控制 EL2 寄存器是否走 VNCR 访问路径；上游在 6.9 周期加入 NV1/E2H0 探测 |
| bit 45 | `NV2` | **ARMv8.4 (FEAT_NV2)** | 允许 Guest hypervisor 通过 **VNCR_EL2** 指向的 4 KB 页读写其 EL2 寄存器，**不产生陷阱** |
| 其余 | `VM/PTW/VI/VF/VSE/TWI/TWE/TVM/TRVM/TTLB/TSC/TID*/TEA/TERR/API/APK/AT/...` | 按版本递增 | 细粒度陷阱：MMU 寄存器、缓存维护、TLBI、调试、PAuth 指令、AT 指令等 |

- **实现要点**：KVM 用 `IMO/FMO/AMO` 把物理中断截到 EL2，从而不依赖 host 内核的 IRQ 分发；`TGE/E2H` 决定 host 内核是在 EL2（VHE）还是 EL1（nVHE）。
- **缺失代价**：VHE 缺失 → 走 nVHE，切换时寄存器跳板开销 + 代码分叉（`hyp/vhe` 与 `hyp/nvhe` 两套）；IMO/FMO/AMO 缺失（极早期实现）→ 中断必须经 host 内核转发，延迟显著上升。

### 3. Stage-2 地址翻译

- **两级翻译**：Guest 虚拟地址 **GVA** --(Stage-1，Guest OS 的页表，`TTBR0/1_EL1`)--> **IPA** --(Stage-2，Hypervisor 的页表，`VTTBR_EL2`)--> **PA**。硬件在 TLB 中缓存合并结果；KVM 修改 stage-2 后需按 VMID 做 TLBI。
- **关键寄存器**：
  - `VTTBR_EL2`：Stage-2 页表基址 + **VMID**（区分不同 VM 的 TLB 条目）；
  - `VTCR_EL2`：Stage-2 的翻译控制（`T0SZ`、`SL0` 起始级别、`TG0` 粒度、`PS` 物理地址宽度、`SH/ORGN/IRGN` 等）；
  - `HPFAR_EL2`：stage-2 缺页时给出出错 **IPA**（配合 `FAR_EL2` 的 GVA 定位）。
- **引入时间**：**ARMv8.0-A**（ARMv7 VE 的 32 位 HYP 也已有 stage-2）。这是 ARM 虚拟化的地基能力，**没有它就只能影子页表（shadow page table）**，即每个 guest 页表更新都要陷出并同步影子表——这正是 x86 在 EPT/NPT 之前的处境。
- **Stage-2 fault 处理**：guest 访问未映射 IPA → 陷入 EL2，`ESR_EL2.EC=0x10`（lower EL 的 Data Abort）→ KVM 用 `HPFAR_EL2` 取 IPA，走 stage-2 fault 路径分配/映射物理页（大页合并、`MMIO` 则交回用户态 VMM）。`ESR_EL2.EC=0x14` 为 instruction abort。
- **缺失代价**：无 stage-2 → 影子页表，guest 每次页表写入都 VM-Exit；内存密集型负载会因此掉一大截。

### 4. VHE（Virtualization Host Extensions，ARMv8.1）

- **机制**：`HCR_EL2.E2H=1` + `HCR_EL2.TGE=1` 后，host 内核直接运行在 **EL2**，EL1 的系统寄存器访问被硬件别名到 EL2 寄存器（如 `TTBR0_EL1`↔`TTBR0_EL2`、`SCTLR_EL1`↔`SCTLR_EL2`）。
- **收益**：guest 进出时不再需要在 EL1 与 EL2 之间搬运/重装 hyp 寄存器，消除**寄存器跳板**；host 与 KVM 共享同一个页表基址与上下文，进入 guest 只需切 stage-2 与 vcpu 状态。
- **可发现性**：`ID_AA64MMFR1_EL1.VH`（0=不支持，1=支持）。Linux 侧 KVM 启动日志会打印 `VHE mode initialized successfully`（或 nVHE 的对应信息），这是**判断一台鲲鹏机器实际走 VHE 还是 nVHE 最快的实证方法**。
- **缺失代价**：回落到 nVHE。功能仍然可用（KVM 支持 nVHE），但每次 world switch 都要执行跳板代码（保存/恢复大量 hyp 寄存器），世界切换延迟与代码复杂度上升；pKVM（Protected KVM）等新特性也更强依赖 VHE/新架构。

### 5. 嵌套虚拟化（FEAT_NV / FEAT_NV2，ARMv8.3 / ARMv8.4）

- **FEAT_NV（ARMv8.3-A）**：提供「Guest hypervisor 的 EL2」的硬件辅助。`HCR_EL2.NV=1` 后，L2 guest 的 EL1 实际运行在 host EL1，而 L1 hypervisor 对 EL2 的访问由硬件重定向/陷入给 host EL2 处理；很多 EL2 寄存器访问仍需陷入。**它降低了模拟成本，但仍以陷阱为主。**
- **FEAT_NV2（ARMv8.4-A）**：新增 **`VNCR_EL2`**——一块 4 KB 对齐的内存页，Guest hypervisor 的 EL2 寄存器状态（`HCR_EL2`、`VTTBR_EL2`、`VTCR_EL2`、`SCTLR_EL2`、`VBAR_EL2` 等）映射到这里，L1 hypervisor 读写这些寄存器**不触发任何陷阱**，只有少数必须陷入的操作（TLBI、AT、部分 timer/GIC 操作）才 VM-Exit。这才是可用的嵌套。
- **上游状态**：Marc Zyngier 的系列补丁标题为 **"KVM: arm64: ARMv8.3/8.4 Nested Virtualization support"**，其中包含 `ARM64_HAS_NESTED_VIRT` 与 `ARM64_HAS_NESTED_VIRT_ENHANCED`（ARMv8.4 Enhanced NV）cpufeature，并通过 `kvm-arm.mode=nested` 使能。到 2025 年的 **v11** 版本，标题已变成 **"Nested Virtualization support (FEAT_NV2 only)"**——**上游只做 FEAT_NV2（ARMv8.4）**，不再维护 FEAT_NV（8.3）的降级路径。
- **ARMv8.2 及更早的代价**：不存在硬件辅助，只能**全 trap 模拟**：影子 stage-2 页表、影子 GIC（维护中断转发）、timer 指令模拟、TLBI/AT 指令逐条陷入。代价是 VM-Exit 数量级上升、时间/计数器语义难以保真、TLB 失效风暴，工程上几乎没有可用实现——**上游 KVM/arm64 没有提供这条路径**。
- **实践含义**：在只支持 ARMv8.2 的机器上，**「在 VM 里再跑 KVM」不可行**（KVM 需要真实 EL2）。可行替代：物理机直跑（裸金属/裸机云）、容器/命名空间隔离、或微虚拟机（microVM）但这仍然只解决「一层」隔离。

### 6. 中断虚拟化：GICv2 / GICv3 / GICv4 / GICv4.1

| 方案 | 机制 | 引入 | 关键寄存器/接口 |
|------|------|------|----------------|
| 无虚拟化扩展 | Guest 访问 GIC 触发 MMIO/系统寄存器陷入，hypervisor 软件注入 | — | 全软件模拟 |
| GICv2 + 虚拟化扩展 | `GICH_*`（Hypervisor 内存映射接口）+ `GICV_*`（Virtual CPU 接口）+ **list registers** 投递虚拟中断 | ARMv7 VE 时代（2011 前后） | `GICH_LR`、`GICH_HCR`、`GICH_MISR` |
| GICv3 | 系统寄存器接口，`ICH_*_EL2`；虚拟中断投递仍靠 **list registers** + maintenance interrupt；LPI/ITS 支持 MSI | ARMv8 时代（GICv3 规范 2013 起） | `ICH_LR<n>_EL2`、`ICH_HCR_EL2`、`ICH_VMCR_EL2`、`ICH_AP0R/AP1R`、`ICH_VTR_EL2` |
| GICv4 | **vLPI 直接注入**：物理 LPI 的 ITS 翻译结果先指向 vPE，由 GIC 硬件直接写入 vCPU 的虚拟 LPI；不再需要 VM-Exit | GICv4（约 2017） | `ICH_VMCR`/vPE 表（GITS 的虚拟化支持） |
| GICv4.1 | 追加 **vSGI 直接注入**（核间中断直通）与更多超分支持 | GICv4.1（约 2019–2020） | vSGI 直通、`ICH_*` 扩展 |

- **没有 GIC 虚拟化扩展的代价**：每次中断都要「物理中断→EL2/host→注入虚拟中断」两跳，高 IOPS/网络场景下 VM-Exit 频率直接压垮吞吐；GICv3 的 list registers 已经是硬件队列（比纯软件快很多），但仍受 LR 数量（典型 4–16 个）与 maintenance interrupt 频率限制；GICv4/4.1 才是把中断路径彻底移出 hypervisor。
- **虚拟 timer**：`CNTVOFF_EL2` 提供虚拟计数偏移，`CNTHCTL_EL2` 控制物理 timer 是否陷入 EL2；无硬件虚拟 timer 时需要逐次模拟（旧架构），代价高。

### 7. SMMU（SMMUv2/SMMUv3）与设备直通

- **作用**：给 DMA 设备做地址翻译与隔离。Stage-1（设备驱动视角的 VA→PA）与 **Stage-2（Hypervisor 掌控的 IPA→PA）**，后者与 CPU 的 stage-2 使用同一套映射语义——**这就是 SMMU 与 VT-d 的对应关系**。
- **SMMUv2**：以「一个设备一个 stage-1 上下文 + 全局 stage-2」的方式工作，实现较简单；x86 对应物是 VT-d 的二级页表（SLPT/EPT）。
- **SMMUv3**：架构化设计——**Stream Table Entry (STE)** 描述设备、**Context Descriptor (CD)** 描述地址空间、命令队列/事件队列由软件填充，天然支持 stage-1+stage-2 组合、PCIe ATS/PASID、MSI 翻译。**SMMUv3 的两层翻译就是 VT-d 二维翻译的 ARM 版**。
- **嵌套翻译（vSMMU）**：要让 guest 自己管 stage-1（guest 内核的 IOMMU 驱动），而 host 管 stage-2，需要 **SMMUv3 的 nested translation** 能力（SMMUv3.3 引入）与内核/QEMU 侧支持（Linux 社区有 "Add Nested Translation Support for SMMUv3" 系列，QEMU 侧有 vSMMUv3 + VFIO 两阶段集成）。**在缺少该能力的平台上，guest 里只能看到一个被模拟的 SMMU 或被隐藏**。
- **HTTU（Hardware Translation Table Update）**：SMMU 硬件自动更新页表的 Access/Dirty 位。**这是直通设备热迁移的关键**：迁移需要知道设备写过哪些页（脏页跟踪），没有 HTTU 就要靠设备自身标脏或软件替代，很多设备因此不可迁移。
- **缺失代价**：没有 SMMU → 直通设备的 DMA 无法隔离和翻译，只能靠 32 位 SMMU 窗口/IOMMU 旁路（不安全）或干脆不直通；有 SMMU 但无 stage-2 集成 → guest 看不到 IOMMU，直通设备的地址空间仍由 host 决定；有 SMMU 但无 HTTU → 直通设备热迁移基本不可行。

### 8. PMU / SPE / TRBE-ETE

- **PMUv3 + 虚拟 PMU**：Armv8.0/8.1 的 PMUv3 通过 `MDCR_EL2`（`HPMN` 计数器数量、`TPM` 是否陷入、`TPMCR` 溢出路由）做虚拟化。KVM 用 `pmu-emul.c` 做「计数器陷入 + 事件软件模拟」，guest 内 `perf` 因此可用但精度/开销不如裸机。
  - **缺失代价**：guest 无 PMU → `perf`/监控不可用，云上可观测性缺失。
- **SPE（Statistical Profiling Extension）**：**ARMv8.2 可选**扩展，基于采样的剖析（`PMSCR_EL1`、`PMBLIMITR_EL1`、`PMSIDR_EL1` 等）。SPE 的虚拟化（vSPE）实现较晚且复杂（缓冲区 `TRBLIMITR` 类寄存器、数据源过滤），KVM 侧支持在较新内核才逐步完善（**具体合入版本未在本次调研中取证，标待证实**）。
  - **缺失代价**：guest 无 SPE → 云上无法做低开销的指令级剖析。
- **TRBE / ETE**：**ARMv8.4** 引入的追踪扩展（Trace Buffer Extension / Embedded Trace Extension），属 CoreSight 体系。TRBE 缓冲区绑定 CPU 且是物理资源，**虚拟化代价高**，主流做法是不向 guest 暴露。
- **MDCR_EL2 的作用**：PMU/SPE/TRBE 的宿主/客户机可见性与陷阱全部由它集中控制（`HPMN`、`TPM`、`TTRF`、`EnSPE` 等）——这也是判断平台「PMU 虚拟化是否可用」的关键寄存器。

### 9. MTE / PAuth / RAS / MPAM

- **MTE（Memory Tagging Extension，ARMv8.5-A 可选）**：给内存打 4-bit tag。虚拟化涉及时需要 `GMID_EL1`（tag 检查粒度）、`TFSR_EL1/EL2`（同步/异步 tag fault 状态）以及标签存储的生命周期管理。对 guest 暴露 MTE 需要 KVM 与 VMM 共同配合，否则 guest 只能用 non-tagged 内存。
  - **缺失代价**：guest 内无内存安全标签 → 云上无法用 MTE 做 UAF/越界检测；宿主可自行用于 host 侧安全。
- **PAuth（Pointer Authentication，ARMv8.3-A）**：签名的返回地址/指针。对虚拟化基本"透明"，但 `HCR_EL2.API/APK` 可让 PAuth 指令陷入；KVM 需要在世界切换时保存/恢复 key 寄存器（属 EL1 上下文）并正确暴露 ID 能力。
  - **缺失代价**：guest 内核无 PAC 保护（安全削弱），但不影响虚拟化功能本身。
- **RAS（Reliability, Availability, Serviceability，ARMv8.2-A）**：错误记录/上报体系（`ERX*_EL1/EL2/EL3`、`DISR_EL1`、`VDISR_EL2` 虚拟 SError）。虚拟化难点在于「错误记录是每 EL 一份的内存映射资源」，**上游 KVM 在嵌套场景直接对 guest 隐藏 RAS**（补丁 "KVM: arm64: nv: Hide RAS from nested guests"）。
  - **缺失代价**：guest 拿不到 SEA/SError 的虚拟化视图 → guest 内 RAS 处理形同虚设，严重错误由宿主吞掉或直接 panic。
- **MPAM（Memory System Resource Partitioning and Monitoring，ARMv8.4-A 可选）**：控制 L3 容量与内存带宽并做监控。虚拟化需要 `MPAM2_EL2` 及 vPMR 等机制（ARM 官方有 MPAM Overview）。
  - **缺失代价**：多租户下无法隔离 LLC/带宽 → 「吵闹邻居」问题；云上 QoS 无法承诺。

### 10. RME（Realm Management Extension）与 Arm CCA 机密计算

- **概念**：Arm CCA（Confidential Compute Architecture）在原有的 Secure / Non-secure 两世界之外引入 **Root / Realm / Non-secure** 三世界：Realm 由 RMM（Realm Management Monitor，运行在 EL2 的受信固件）管理，Guest 内核在 Realm EL1、应用在 Realm EL0；Non-secure 的 Hypervisor **看不到 Realm 的内存**，从而对云厂商保密。
- **关键能力**：`RME` 提供 Realm 的 Stage-2 隔离与内存加密状态（Granule Protection Table 控制页的归属：Root/Realm/Secure/Non-secure）、Realm 启动度量与 attestation（由 RMM + 验证服务完成）。
- **引入版本**：公开资料普遍把 **RME 归入 Armv9-A（多为 "Armv9.2"）**；本次调研**未能取到 ARM ARM 原文的版本标注**，故**标「待证实」**，只确认 Arm CCA 于 2021 年随 Armv9 公布，且有 Fujitsu MONAKA 等落地实现。
- **缺失代价**：无 RME → 云上只能用「TrustZone 把整个 host 变成 TEE」或软件保护（SEV/TDX 无 ARM 对应物），多租户机密计算无法实现。
- **鲲鹏的机密计算路线**：鲲鹏社区有独立的「BoostKit 机密计算」文档集，含 **TrustZone 技术**与 **「机密虚拟机与机密容器」**（TEE 套件）条目，说明鲲鹏侧机密计算以 **TrustZone 系 TEE** 为核心；**鲲鹏 950 宣称「成为鲲鹏首颗实现机密计算的数据中心处理器」**（已证实）。**是否基于 RME/CCA，未找到公开资料证实。**

### 11. ARM 侧能力 × 首次引入版本速查

| 能力 | 首次引入 | 必选/可选 | 缺失时的回退与代价 |
|------|---------|-----------|-------------------|
| EL2 / HYP | ARMv7 VE（32 位）；AArch64 EL2 = ARMv8.0 | VE 起 | 无 EL2 → 不能跑 KVM，只能模拟/半虚拟化 |
| Stage-2 翻译（VTTBR/VTCR） | ARMv8.0 | 虚拟化必选 | 影子页表，每次 guest 页表写入都 VM-Exit |
| VHE | ARMv8.1 | 可选（`ID_AA64MMFR1.VH`） | nVHE + 寄存器跳板，世界切换变慢 |
| RAS | ARMv8.2 | 可选 | guest 无 RAS 视图，只能被宿主吞掉 |
| SPE | ARMv8.2 | 可选 | guest 无采样剖析 |
| SVE | ARMv8.2 | 可选 | guest 无向量宽可变指令（与虚拟化正交） |
| PAuth | ARMv8.3 | 可选 | guest 无 PAC（安全性削弱，功能不受影响） |
| FEAT_NV | ARMv8.3 | 可选 | 嵌套需全 trap 模拟（工程上基本不可用） |
| FEAT_NV2 + VNCR_EL2 | ARMv8.4 | 可选 | 同上；上游 KVM 只支持本档 |
| Secure EL2（FEAT_SEL2） | ARMv8.4 | 可选 | 无 EL2 的 Secure 世界（如 pKVM 类方案受限） |
| MPAM | ARMv8.4 | 可选 | 无 LLC/带宽隔离与监控 |
| TRBE / ETE | ARMv8.4 | 可选 | guest 内无追踪缓冲区 |
| MTE | ARMv8.5 | 可选 | guest 无内存标签 |
| RME（Arm CCA） | Armv9-A（常标 Armv9.2，**待证实**） | 可选 | 无机密计算 Realm |
| GICv3 | GICv3 规范（ARMv8 时代） | IP 集成决定 | 仍可用 GICv2 虚拟化接口，能力受限 |
| GICv4 / GICv4.1 | GICv4（约 2017）/ v4.1（约 2019–20） | IP 集成决定 | vLPI/vSGI 直通缺失，中断必须经 hypervisor |
| SMMUv3 嵌套翻译 | SMMUv3.3（约 2021，**以 Arm SMMU 规范为准**） | 可选 | guest 内无自管 stage-1，vSMMU 能力受限 |

## 鲲鹏平台

> 证据标注规则：**【已证实】** = 有官方/一手公开资料（海思/华为官网、华为 Connect 演讲、鲲鹏社区官方文档、openEuler/BoostVirt 官方仓库）；**【推断】** = 由已证实事实推出的结论，已注明依据；**【未证实】** = 未找到可靠公开来源，不作事实陈述。

### 1. 鲲鹏 920（TaiShan v110 核）

**【已证实】官方规格**（海思产品页）：

| 项目 | 值 |
|------|-----|
| Architecture | **ARM v8.2** |
| Core | up to 64 / 芯片 |
| 典型频率 | 2.6 GHz / 3.0 GHz |
| 内存 | 8 × DDR4 通道 |
| 一致性互连 | 2S & 4S |
| I/O | PCIe 4.0、CCIX、100G、SAS/SATA 3.0 |
| 工艺 / 功耗 | 7 nm / 最高 180 W |

**【已证实】官方页面未列出的内容**：海思与鲲鹏社区的鲲鹏 920 产品页**没有列出任何虚拟化特性**（EL2、Stage-2、vGIC、SMMU 版本、VHE 均未提及）。因此：

- EL2 / Stage-2：**推断支持**（ARMv8.2-A 的服务器级实现且鲲鹏实际承载 openEuler KVM 生产负载，官方 BoostVirt 文档中大量以 KVM/QEMU/libvirt 为前提）。依据强，但仍属推断。
- **VHE：未找到华为官方资料证实**。需要实测：`dmesg | grep -i kvm` 看是 `VHE mode initialized successfully` 还是 nVHE 信息，或读取 `ID_AA64MMFR1_EL1.VH`。
- **SMMU：已证实存在且需在 BIOS 使能**——BoostVirt「中断直通」文档要求 `BIOS → Advanced → MISC Config → Support SMMU = Enabled`；vKAE 文档称「**鲲鹏 920 新型号处理器 SMMU 支持 HTTU 硬件标脏特性**」。**具体 SMMU 版本（v2/v3、v3.x）未找到公开资料证实。**
- **GIC：已证实 920 新型号支持 GICv4.0/GICv4.1 且 BIOS 可选**——官方文档：「BIOS 默认配置是 GICv4.0，需要手动设置为 GICv4.1」，`kvm-arm.vgic_v4_enable=1` 后 `dmesg` 出现 `kvm [1]: GICv4.1 support enabled`。**原版 920 的 GIC 版本未找到公开资料证实。**

**【已证实】嵌套虚拟化 = 不支持（本项为本次调研最关键结论）**：

- 事实一（已证实）：鲲鹏 920 官方架构版本为 **ARMv8.2**（海思官网 Key Features）。
- 事实二（已证实）：上游 KVM/arm64 的嵌套虚拟化**只支持 FEAT_NV2（ARMv8.4）**——Marc Zyngier 系列补丁自 v1 起标题即为 "ARMv8.3/8.4 Nested Virtualization support"，到 2025 年的 v11 收敛为 **"(FEAT_NV2 only)"**；ARMv8.2 平台在补丁作者的测试说明中根本不在支持列表（原文点名需要 "anything else with ARMv8.4-NV"）。
- 事实三（已证实）：华为官方 BoostVirt/鲲鹏文档中**从未把「嵌套虚拟化」列为 920 的虚拟化特性**；鲲鹏社区论坛有用户提问「鲲鹏920支持 KVM 嵌套虚拟化吗」（该帖正文为 JS 渲染，抓取不到答案内容，无法作为证据）。
- **结论（推断，高置信）**：**鲲鹏 920 没有硬件嵌套虚拟化路径，上游 KVM 下「VM 里再跑 KVM」不可用**。这不是配置问题，而是架构版本缺口。**至今未找到任何华为官方资料声明 920「支持嵌套虚拟化」，因此"不支持"的否定性结论虽无官方正面声明，也可按已证实事实组合判定。**
- **可行路径**：① 裸金属直接部署（放弃多一层虚拟化）；② 用容器/命名空间隔离（iSula、Kubernetes）替代 VM 嵌套；③ 在 guest 内跑 QEMU TCG 纯模拟（性能量级不可接受，仅用于功能验证）；④ 等鲲鹏 950/后续型号——**是否支持 FEAT_NV2 未找到公开资料证实**。

**【待消解的矛盾】** 仓库既有笔记 `docs/arm-kunpeng/kunpeng-920b-specs.md` 记录一台 **920B**（2 socket × 80 核 × 2 线程 = 320 逻辑核，SMT 开启）实测 flags 含 `sve / svei8mm / svebf16 / svef64mm`。这与「原版 920 官方最高 64 核、ARMv8.2」以及「920 不支持 SVE」的常见表述**不一致**。合理推测是该机器属于 **920 新型号/920B**（更晚的衍生型号），但**未找到公开资料证实 920B 的架构版本与特性集**，故仅记录矛盾，不据此推断能力（列为未决问题）。

### 2. 鲲鹏 950 与后续路线

**【已证实】华为 Connect 2025（2025-09-18，徐直军主题演讲）**：

- **Kunpeng 950**：2026 Q1 推出，两个型号——**96 核 / 192 线程** 与 **192 核 / 384 线程**（自研双线程 LinxiCore）；支持**通用计算超节点（SuperPoD）**；「安全方面新增四层隔离，**成为鲲鹏首颗实现机密计算的数据中心处理器**」。
- **TaiShan 950 SuperPoD**：基于 Kunpeng 950，最多 16 节点 / 32 处理器 / 48 TB 内存，带内存/SSD/DPU 池化；**虚拟化场景内存利用率提升 20%**；Spark 实时数据处理快 30%；2026 Q1 上市。
- **2028 路线**：高性能型号 96 核/192 线程，单核性能提升 50%+（AI 主机、数据库）；高密度型号**至少 256 核/512 线程**，官方定位明确写「适合 **virtualization**、containers、big data、data warehouses」。

**【已证实】BoostVirt 文档中对 950 的支持**：

- vKAE 直通热迁移：硬件要求含「鲲鹏 950 处理器」；
- MPAM：硬件要求含「鲲鹏 950 处理器」，且「**针对鲲鹏 950 处理器，新增 max、min**」（对应 MPAM 的 `L3MAX`/`L3MIN`）。

**【未证实】鲲鹏 950 的以下信息**（本次调研未找到可靠公开来源，禁止按 Armv9 等自行推断当事实）：

- **ARM 架构版本**（是否 Armv9、是否含 RME/CCA、是否含 FEAT_NV2/SEL2）；
- 是否支持**硬件嵌套虚拟化**（FEAT_NV2）；
- SMMU 版本与是否支持 SMMUv3 嵌套翻译（vSMMU）；
- GIC 版本（是否 GICv4.1 及以上）；
- 「四层隔离」的具体技术构成（TrustZone TEE？RME Realm？还是其他自研隔离）——鲲鹏 BoostKit 机密计算文档集含 TrustZone 与「机密虚拟机与机密容器」条目，**只能证明鲲鹏机密计算路线与 TrustZone TEE 相关，不能证明 950 采用 RME**。

### 3. 鲲鹏虚拟化软件栈【均已证实】

| 层次 | 组件 | 关键事实 |
|------|------|---------|
| 内核 | **KVM on ARM64**（openEuler 内核，如 OLK-6.6、5.10 系） | BoostVirt 特性均以 openEuler + KVM 为前提；MPAM 需 `arm64.mpam` 参数（openEuler 内核补丁，非纯上游） |
| VMM | **QEMU**（6.2.0 / 8.2.0 为官方验证版本）、libvirt（6.2.0 / 9.10.0） | 官方文档逐项给出 QEMU/libvirt 版本约束；aarch64 用 `virt` 机型 |
| Rust VMM | **StratoVirt**（openEuler 社区，Mulan PSL v2） | 官方 README：一套架构统一支持虚拟机/容器/Serverless；显式支持 `aarch64`（`-machine virt`，console `ttyAMA0`）；Rust 语言级安全 + 轻量低噪 |
| 加速套件 | **BoostVirt**（鲲鹏 BoostKit 云计算团队，Apache-2.0，代码仓 `gitcode.com/boostkit/boostvirt`、`cloud-virtual`） | 覆盖 KVM/QEMU/libvirt/DPDK/SPDK 的鲲鹏亲和优化 |
| 容器 | **iSula** / Kubernetes（cloud-native 仓） | BoostVirt 云原生侧提供 KAE Device Plugin、K8s 拓扑亲和插件、K8s MPAM 插件、SR-IOV 直通插件 |
| 加速引擎 | **KAE**（Kunpeng Accelerator Engine，2.0） | KAEKernelDriver + UADK + KAEOpensslEngine + KAEZlib/KAEZstd/KAELz4；设备通过 SR-IOV VF + `vfio-pci`/`hisi_acc_vfio_pci` 直通 |

**BoostVirt 特性清单（已证实，来自官方 README，更新于 2026-03-24）**：

- 虚拟化 CPU 加速：**中断直通**（GICv4.1 vLPI/vSGI/虚拟设备中断直注）、**拓扑感知调度**（CPU 拓扑直通 guest、Cluster 级调度、超线程调度与动态绑核、抢占锁优化、硬件死锁检测）；
- 虚拟化 IO 加速：**硬件加速器**（VM 内用 KAE、vKAE 热迁移）；
- 虚拟化管理优化：**热迁移 KAE 加速**（KAEZlib 替代原生 zlib 压缩迁移数据流）、**热插拔**（vCPU 与内存）、**MPAM**（libvirt XML 限制 VM 的 DDR 带宽与 L3 容量）；
- 云原生：KAE Device Plugin、K8s 拓扑亲和、K8s MPAM、SR-IOV 直通。

### 4. 设备直通、vSMMU 与热迁移

**【已证实】**：

- 直通基础：BIOS 使能 SMMU + VFIO（`vfio-pci`，KAE 用 `hisi_acc_vfio_pci`）+ SR-IOV VF。中断侧可叠加 GICv4.1 直注（vLPI）。
- **热迁移硬约束**：vKAE 文档明确「**不支持 KAE 设备以外的直通设备热迁移**」；「迁移源端与目的端 **BIOS 以及固件版本需要保持一致**」；KAE 设备尽量放在同一 NUMA 节点；性能指标为停机时间 **< 200 ms**（用 `all_vcpus_paused`/`all_vcpus_prepared` trace 事件计算）。
- **vSMMU**：**未找到公开资料证实鲲鹏/openEuler 在 guest 内提供可用的 vSMMU（SMMUv3 嵌套翻译）能力**。可确认的只有「host 侧 SMMU stage-2 支撑直通 + HTTU 标脏支撑可迁移设备」。**QEMU 存在 vSMMUv3 设备模型，但其与 KVM 的嵌套 stage-2 集成在 arm64 上并不完整，因此在鲲鹏上 guest 内自管 IOMMU 的能力应视为【未证实/推断受限】。**
- **跨厂商/跨型号热迁移**：官方只承诺同型号 + 固件一致；openEuler 技术白皮书提到 vDPA 作为「性能与直通持平、且支持跨硬件厂商热迁移」的方向（白皮书 PDF 未能完整取证，**待证实**）。**ARM 侧跨 SoC 迁移受 GIC ITS、固件、PMU/SPE 等非架构化资源影响，直觉上不可行——标【推断】。**

### 5. 已知的性能与兼容性问题（尽量取官方记录）

| 问题 | 来源/性质 | 说明 |
|------|----------|------|
| **GICv4.1 + vcpupin/emulatorpin 绑核重叠 → CPU 利用率接近 100%、虚拟机性能下降** | 官方 troubleshooting 文档链接（已证实存在该条目） | 开启 GICv4.1 中断直通后需避免绑核重叠 |
| **GICv4.1 超分场景 VMOVP 串行** | BoostVirt 官方指南（已证实） | `VMOVP` 在 GIC 硬件上串行执行，vCPU 迁移多时性能下降；华为提供 `[GICv4.1 Oversubscription Optimization]0001-KVM-arm64-Optimize-VMOVP.patch`，**仅支持 kernel 6.6 + libvirt 9.10.0 + QEMU 8.2.0 + openEuler 24.03 LTS SP3**——版本绑定很紧 |
| **vCPU 热插拔限制** | BoostVirt 官方指南（已证实） | 迁移/休眠唤醒/快照期间**不支持**热插拔；同一 Socket 的 vCPU 必须放同一 vNode 否则可能软锁死；热插数量建议为 Cores 整数倍；热拔仅 openEuler 24.03 LTS 起支持；新旧热插拔协议不兼容（Guest 内核与 QEMU 必须配套） |
| **直通设备热迁移缺失** | BoostVirt 官方约束（已证实） | 除 KAE 外的直通设备不支持热迁移，根因是很多设备不支持 DMA 标脏 |
| **KAE/加速驱动与 OS 自带驱动冲突** | BoostVirt 官方 FAQ（已证实） | openEuler 22.03 SP4 / 24.03 SP3 自带加速驱动不兼容时，重启后查不到设备文件，需卸载后重载手动编译的 KAE |
| **MPAM 依赖 openEuler 内核补丁** | BoostVirt 官方指南（已证实） | 需 `arm64.mpam` 启动参数；主线上游支持情况未在本次取证 |
| **vGIC 性能差的通用认知** | 架构层面【推断】 | 未开 GICv4 时，中断注入走 list registers + maintenance interrupt，LR 数量有限；高 PPS 场景下 VM-Exit 仍是瓶颈。**本次未取到鲲鹏 vs x86 的实测对比数据** |
| **KVM on ARM64 与 x86 的功能差距** | 【推断/定性】 | ARM 侧上游缺失：嵌套虚拟化仅 NV2 档、SMMU 嵌套翻译支持较新、pKVM 等仍在演进；x86 侧有 VMCS/EPT、VT-d 成熟生态。仓库既有笔记亦指出「microVM 不可跨架构迁移」（`docs/rust-kunpeng/rust_kunpeng_planning_outline.md`） |

## 对照表

| 能力项 | ARM 架构版本 | 鲲鹏 920（原版） | 鲲鹏 920 新型号 | 鲲鹏 950 / 后续 | 作用 | 缺失代价 |
|--------|-------------|----------------|----------------|----------------|------|---------|
| EL2（Hypervisor 级） | ARMv7 VE / ARMv8.0 | 支持（推断，未官方列出） | 支持 | 支持（未证实细节） | KVM 的立足点 | 无法运行 KVM |
| Stage-2 翻译（VTTBR/VTCR） | ARMv8.0 | 支持（推断，官方未列出） | 支持 | 支持 | GVA→IPA→PA 硬件两级翻译 | 影子页表，页表写入频繁 VM-Exit |
| VHE（E2H/TGE） | ARMv8.1 | **未证实**（建议实测 `ID_AA64MMFR1.VH` / dmesg） | 未证实 | 未证实 | host 内核跑 EL2，免寄存器跳板 | 回落 nVHE，世界切换变慢 |
| **硬件嵌套虚拟化（FEAT_NV2）** | **ARMv8.4** | **不支持（已证实事实组合推出的高置信结论）** | 未证实 | 未证实 | VM 内再跑 KVM | 嵌套只能用纯模拟（不可用）；上行替代是裸金属/容器 |
| Secure EL2（FEAT_SEL2） | ARMv8.4 | 未证实 | 未证实 | 未证实 | Secure 侧 EL2（pKVM 类方案） | 受保护虚拟化方案受限 |
| GICv3 虚拟化（ICH_LR） | GICv3 规范 | 推断支持（未官方列出） | 支持 | 未证实 | 虚拟中断投递 | 只能软件注入，延迟高 |
| GICv4.0 | GICv4（约 2017） | 未证实 | **已证实（BIOS 默认 GICv4.0）** | 未证实 | vLPI 直注 | 中断需 VM-Exit |
| **GICv4.1（vLPI + vSGI 直注）** | GICv4.1 | **未证实**（文档限定「仅 920 新型号」） | **已证实（BIOS 可切 4.1；需 SMMU Enabled）** | 未证实 | 中断直通，免 VM-Exit/Entry | 网络/IO 密集场景吞吐与延迟受损 |
| SMMU（存在 + 可 BIOS 使能） | SMMUv2/v3 | 推断存在（中断直通文档要求使能 SMMU） | 已证实 | 已证实（vKAE 硬件要求含 950） | 设备 DMA 翻译与隔离 | 设备直通不安全或不可行 |
| SMMU HTTU 硬件标脏 | SMMUv3 一代能力 | 未证实 | **已证实（文档明确「920 新型号 SMMU 支持 HTTU」）** | 未证实 | 直通设备热迁移脏页跟踪 | 直通设备不可热迁移 |
| SMMUv3 嵌套翻译（vSMMU） | SMMUv3.3（待核实） | 未证实 | 未证实 | 未证实 | guest 自管 stage-1 | guest 内 IOMMU 能力受限 |
| 虚拟 PMU（MDCR_EL2 + 模拟） | PMUv3（ARMv8.0/8.1） | 未证实 | 未证实 | 未证实 | guest 内 perf | guest 无可观测性 |
| SPE 虚拟化 | ARMv8.2 可选 | 未证实 | 未证实 | 未证实 | guest 采样剖析 | guest 无低开销剖析 |
| TRBE / ETE | ARMv8.4 可选 | 未证实 | 未证实 | 未证实 | guest 追踪 | 通常不暴露给 guest |
| MTE | ARMv8.5 可选 | 未证实 | 未证实 | 未证实 | guest 内存标签安全 | guest 无 MTE |
| PAuth | ARMv8.3 可选 | 未证实（官方页未列） | 未证实 | 未证实 | guest 内核 PAC | guest 安全性削弱 |
| RAS 虚拟化 | ARMv8.2 可选 | 未证实 | 未证实 | 未证实 | guest 错误处理 | guest RAS 无效 |
| **MPAM（缓存/带宽隔离）** | ARMv8.4 可选 | 未证实 | **已证实（BoostVirt MPAM 硬件要求）** | **已证实（并新增 L3MAX/L3MIN）** | 多租户 LLC/带宽 QoS | 吵闹邻居，QoS 无法承诺 |
| RME / Arm CCA 机密计算 | Armv9-A（版本标注待证实） | 未证实 | 未证实 | **宣称支持机密计算（已证实）；是否基于 RME 未证实** | Realm 级机密计算 | 云上无法对厂商保密 |
| 设备直通（VFIO/SR-IOV） | 依赖 SMMU + GIC | 已证实可用（BoostVirt 全栈前提） | 已证实 | 已证实 | 高性能 IO | 只能 virtio，性能有损 |
| 直通设备热迁移 | 依赖 SMMU HTTU + 设备标脏 | **仅 KAE 支持（已证实）** | 仅 KAE（已证实） | 仅 KAE（已证实） | 运维不中断 | 通用直通设备迁移不可行 |
| vCPU / 内存热插拔 | VMM + Guest OS 能力 | 已证实（openEuler 22.03 SP4 起 vCPU 热插；24.03 LTS 起热拔） | 已证实 | 已证实 | 弹性伸缩、加速启动 | 无法在线调规格 |

> 表中「未证实」一律表示：**本次调研未找到可靠公开来源，不等于硬件不支持**。确认方法见「未决问题」。

## 未决问题

1. **鲲鹏 920 是否实现 VHE**：需在一台 920 上执行 `dmesg | grep -i "kvm"` 看 `VHE mode initialized successfully`（或读 `ID_AA64MMFR1_EL1.VH`）。
2. **「鲲鹏 920 新型号」的确切型号标识**：是 920B？920 某步进？与仓库笔记中的 320 逻辑核机器（2×80×2）是什么关系？这直接决定 GICv4.1 / SMMU HTTU / MPAM 的可用性。
3. **920B 的 `sve` 实测 flag 与官方 ARMv8.2 规格的矛盾**（见核心结论 14）：需要同一台机器上核对 `ID_AA64PFR0_EL1`（SVE 字段）与 `ID_AA64MMFR*`，并与华为确认型号。
4. **原版 920 的 GIC 版本**（GICv3 only？GICv4.0？）与**所有 920 型号的 SMMU 版本号**。
5. **鲲鹏 950 的 ARM 架构版本**及其是否实现 FEAT_NV2 / FEAT_SEL2 / RME。
6. **鲲鹏 950「四层隔离」的技术构成**，以及「机密计算」是 TrustZone TEE、RME Realm 还是自研方案。
7. **鲲鹏平台是否存在可用的 vSMMU（guest 自管 IOMMU）路径**，openEuler 侧是否有 SMMUv3 嵌套翻译的支持计划。
8. **实测数据缺口**：vGIC 中断直通前后（GICv4.0 vs 4.1）的 PPS/延迟对比；virtio vs VFIO 直通在鲲鹏上的吞吐/CPU 开销；鲲鹏与 x86 在 KVM 功能矩阵上的逐项差距。
9. **跨厂商热迁移**：openEuler vDPA 方案在鲲鹏上的实际落地状态（白皮书表述未能完整取证）。
10. **上游 KVM arm64 嵌套虚拟化的最终合入版本**：本次仅取证到补丁系列 v11 明确「FEAT_NV2 only」，未取到合并进主线的具体版本号；**建议以 Linux 主线 `Documentation/virt/kvm/arm/` 与 `arch/arm64/kvm/nested.c` 的 git 历史复核**。

## 参考资料

> 访问日期：2026-09-16（全部链接均为当日访问）。

**ARM 官方与架构**

1. Arm Learn the architecture — AArch64 virtualization（EL 模型、HCR_EL2、Stage-2、VHE）: https://developer.arm.com/documentation/102142/latest/
2. Arm《Armv8-A virtualization》白皮书/PDF: https://developer.arm.com/-/media/Arm%20Developer%20Community/PDF/Learn%20the%20Architecture/Armv8-A%20virtualization.pdf
3. Arm 官方文档服务（架构特性与版本并行更新）: https://documentation-service.arm.com/static/68909099e7f7ce6150e8761d
4. Arm MPAM 概述: https://support.arm.com/documentation/107768/0101/Arm-Memory-System-Resource-Partitioning-and-Monitoring--MPAM--Extension
5. Arm CCA 落地案例（Fujitsu MONAKA confidential computing）: https://developer.arm.com/community/arm-community-blogs/b/servers-and-cloud-computing-blog/posts/how-fujitsu-implemented-confidential-computing-on-fujitsu-monaka-with-arm-cca
6. Trusted Firmware-A 文档（FEAT_SEL2 / Secure EL2 支持）: https://documentation-service.arm.com/static/66e9571563788e5fed16509f

**Linux 内核 / KVM**

7. Marc Zyngier, "[PATCH v5 00/69] KVM: arm64: ARMv8.3/8.4 Nested Virtualization support"（`ARM64_HAS_NESTED_VIRT`、`kvm-arm.mode=nested`、需 ARMv8.4-NV 硬件）: http://lists.openwrt.org/pipermail/linux-arm-kernel/2021-November/699593.html
8. "[PATCH v11 00/43] KVM: arm64: Nested Virtualization support (FEAT_NV2 only)": http://mail.spinics.net/lists/kvm/msg333509.html
9. "[GIT PULL] KVM changes for Linux 6.9 merge window"（HCR_EL2.NV1、ID_AA64MMFR4_EL1.E2H0、E2H RES1 处理）: https://patchew.org/linux/20240315174939.2530483-1-pbonzini@redhat.com/
10. Linux 内核 SMMUv3 嵌套翻译支持系列: https://lore-kernel.gnuweeb.org/lkml/ZBtoj3deE2Y6k9lq@Asurada-Nvidia/T/
11. QEMU vSMMUv3 / pSMMUv3 两阶段 VFIO 集成（Eric Auger）: https://patchwork.ozlabs.org/project/qemu-devel/cover/20200320165840.30057-1-eric.auger@redhat.com/
12. KVM arm64 SPE 支持 RFC: http://yhbt.net/lore/linux-arm-kernel/418e3bf6-159c-6d1f-2b84-2fdb10c3cddd@arm.com/T/
13. Linux MPAM 提交说明（"ARMv8.4 adds support for MPAM"）: http://git.armlinux.org.uk/cgit/linux.git/log/arch/arm64/kernel/cpuinfo.c
14. GICv4 shared VLPI 讨论（KVM/arm64 vgic v4）: https://lkml.indiana.edu/2311.0/04549.html
15. openEuler 内核 MPAM 文档: https://gitee.com/openeuler/kernel/blob/c3f8f5c91794b44b7d65a27371a536a1bc86905e/Documentation/arch/arm64/mpam.md

**华为 / 鲲鹏官方（一手来源）**

16. 海思官网 鲲鹏 920 产品页（Key Features: **ARM v8.2**、up to 64 core、7nm、PCIe 4.0/CCIX/100G）: https://www.hisilicon.com/cn/products/Kunpeng/Huawei-Kunpeng/Huawei-Kunpeng-920
17. 华为 Connect 2025 徐直军主题演讲全文（Kunpeng 950 96C/192T 与 192C/384T、四层隔离与机密计算、TaiShan 950 SuperPoD、2028 路线）: https://www.huawei.com/en/news/2025/9/hc-xu-keynote-speech
18. 鲲鹏 BoostVirt 开源项目 README（特性总表、仓库地址、Apache-2.0，更新 2026-03-24）: https://gitcode.com/boostkit/boostvirt/blob/master/README.md
19. BoostVirt 中断直通 特性指南（GICv4.1 vLPI/vSGI/虚拟设备直注；**仅支持鲲鹏 920 新型号**；BIOS 使能 SMMU 与 GIC Version；`kvm-arm.vgic_v4_enable=1`）: https://www.hikunpeng.com/document/detail/zh/boostvirt/virtcpuacc/interruptpassthrough/docs/zh/interrupt_passthrough_feature_guide.md
20. BoostVirt GICv4.1 超分优化 特性指南（VMOVP 串行问题与 Optimize-VMOVP 补丁，kernel 6.6 + libvirt 9.10.0 + QEMU 8.2.0）: https://www.hikunpeng.com/document/detail/zh/boostvirt/virtcpuacc/gicv/docs/zh/kunpeng_virtualization_gicv4.1_oversubscription_optimization_features_guide.md
21. BoostVirt vKAE 直通热迁移 特性指南（**920 新型号 SMMU 支持 HTTU 硬件标脏**；不支持 KAE 以外直通设备热迁移；BIOS/固件需一致；停机 <200 ms；硬件含鲲鹏 950）: https://www.hikunpeng.com/document/detail/zh/boostvirt/virtioacc/hardwareaccelerator/docs/zh/virtualization_scenario_vkae_passthrough_live_migration_feature_guide.md
22. BoostVirt libvirt 使能 MPAM 用户指南（硬件要求 920 新型号 + 950；`arm64.mpam`；鲲鹏 950 新增 L3MAX/L3MIN）: https://www.hikunpeng.com/document/detail/en/boostvirt/virtmanagopt/mpam/docs/en/mpam_enabled_libvirt_user_guide.md
23. BoostVirt vCPU 热插拔 特性指南（约束：迁移/休眠/快照期间不支持；同 Socket vCPU 同 vNode；24.03 LTS 起支持热拔）: https://gitcode.com/boostkit/boostvirt/blob/master/docs/zh/vcpu_hotplug_feature_guide.md
24. 鲲鹏 BoostKit 虚拟化场景 技术白皮书（方案概述）: https://www.hikunpeng.com/document/detail/zh/kunpengcpfs/twp/kunpengcpfs_19_0080.html
25. 鲲鹏 BoostKit 机密计算 — CCA 套件 / TrustZone 特性指南: https://www.hikunpeng.com/document/detail/zh/kunpengcctrustzone/ccakit/fg-tz/kunpengtee_16_0042.html
26. 鲲鹏 BoostKit 机密计算 — 机密虚拟机与机密容器（TEE 套件）: https://www.hikunpeng.com/document/detail/zh/kunpengcctrustzone/tee/cVMcont/kunpengtee_16_0002.html
27. 鲲鹏 BoostKit 机密计算 — Kunpeng TrustZone 技术原理（英文版）: https://www.hikunpeng.com/document/detail/en/kunpengcctrustzone/trustzone/twp/kunpengtrustzone_19_0005.html
28. 鲲鹏 KAE 鲲鹏加速引擎 项目介绍: https://www.hikunpeng.com/document/detail/zh/kunpengaccel/kae/kae/README.md
29. 鲲鹏硬件常见问题（含「鲲鹏920处理器不支持 SM3/SM4 指令，但集成有加速器」）: https://www.hikunpeng.com/doc_center/source/zh/kunpengfaq/productfaq/hardwarefaq/鲲鹏硬件常见问题.pdf
30. 华为云论坛帖「鲲鹏920支持 KVM 嵌套虚拟化吗」（**正文为 JS 渲染，本次未能取到答案内容，不作证据**）: https://bbs.huaweicloud.cn/forum/thread-0229132917577718008-1-1.html

**openEuler / Rust VMM**

31. StratoVirt 官方 README（Rust、aarch64 `virt` 机型、Mulan PSL v2、统一支持 VM/容器/Serverless）: https://github.com/openeuler-mirror/stratovirt
32. openEuler 22.03 LTS SP4 技术白皮书（vDPA 面向跨硬件厂商热迁移的表述，**PDF 未能完整取证**）: https://www.openeuler.org/whitepaper/openEuler%2022.03%20LTS%20SP4%20技术白皮书.pdf
33. openEuler Kuasar + iSulad + StratoVirt 部署文档: https://raw.githubusercontent.com/kuasar-io/kuasar/8ccfe0d16dc34ba818cf5d2ca6c322e93e2f45e3/docs/vmm/how-to-run-kuasar-with-isulad-and-stratovirt.md

**仓库内部参考**

34. `docs/arm-kunpeng/kunpeng-920b-specs.md`（920B 实测 flags，含 `sve`，320 逻辑核）— 与官方 920 规格存在待消解矛盾
35. `docs/rust-kunpeng/cubesandbox-rust-analysis.md`（第 381 行附近：CubeSandbox v0.5 ARM64 全栈支持，说明 rust-vmm 生态 ARM64 已达生产可用）
36. `docs/rust-kunpeng/rust_kunpeng_planning_outline.md`（microVM 不可跨架构迁移；「需要提升团队对 ARM 硬件虚拟化理解」）
