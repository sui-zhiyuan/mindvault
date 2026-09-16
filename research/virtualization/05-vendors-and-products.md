# 虚拟化领域的公司与产品 —— Hyper-V / VMware / VirtualBox / Azure / AWS / 阿里云 / 华为云 / 腾讯云 / Google Cloud

> 类型：中间调研稿（研究稿，非正式知识库笔记）
> 撰写日期：2026-09-16
> 信息时效：所有结论均标注来源与访问日期；联网核查完成于 2026-09-16。
> 说明：本文属于 `research/` 下的调研稿，未写入 `docs/`，未修改 `docs/SUMMARY.md`。

---

## 核心结论

1. **"Type-1 / Type-2" 二分法已经不足以描述今天的虚拟化产品**。至少需要四类：Type-1（裸金属 hypervisor）、Type-2（宿主型）、**KVM 型**（Linux 内核模块 + 用户态 QEMU VMM）、**混合/微内核型**（Xen 的 dom0 模型、Hyper-V 的 microkernel + root partition 模型）。公有云自研架构（AWS Nitro、华为 QingTian、阿里神龙、Azure Boost）实际上已经是**第五类：卸载型（offloaded）架构**，hypervisor 被刻意做小，I/O 与控制在专用硬件上跑。

2. **KVM "算 Type-1 还是 Type-2" 的争论没有唯一正确答案，但有可操作的结论**：KVM 是**内核模块**，本身不是完整 hypervisor，它借用 Linux 的调度器与内存管理（KVM 官方 FAQ 原文："KVM is part of Linux and uses the regular Linux scheduler and memory management"）。因此在架构语义上它既不是经典 Type-1（不接管整机），也不是经典 Type-2（不依赖宿主 OS 的普通进程去模拟硬件）。工程上最准确的描述是"**Linux 内核内的硬件虚拟化层 + 用户态 QEMU 作为 VMM**"。厂商宣传里的 "Type-1" 是市场定位，不是架构定义。

3. **hypervisor ≠ VMM ≠ 容器运行时 ≠ microVM monitor**。hypervisor 负责 CPU/内存分区与隔离（KVM 内核模块、Hyper-V 的 `hvix64.exe`、Xen 的 Xen 本体）；VMM 负责设备模型与 VM 生命周期（QEMU、Firecracker、Cloud Hypervisor、CubeHypervisor）；容器运行时负责 OCI 生命周期与 rootfs 挂载（runc、containerd、CRI-O）；microVM monitor 是"极简 VMM"，只保留 virtio 的一小部分设备（Firecracker 只保留约 3 种 virtio 设备）。把 QEMU 叫 hypervisor、把 gVisor 叫 VM 都是常见误称。

4. **VMware 的商业模式已被 Broadcom 彻底改写**。Broadcom 2023-11 完成收购后，2023 年底起取消永久授权、改为**按核订阅**，并重新打包为 **VMware Cloud Foundation (VCF)** 与 **VMware vSphere Foundation (VVF)** 两个捆绑 SKU；vCenter 等管理组件被限制只能在 VCF 环境内使用；2025-05 的 SPD 进一步引入**每 180 天必须提交合规报告**、否则 270 天后开始限制 VCF 管理面功能。VMware 永久授权被"封顶在 5.x"（即无法升级到后续订阅制版本）。

5. **VirtualBox 的"KVM 后端"说法需要精确表述，常见解读是错的**。已证实的是：VirtualBox **7.2.0（2025-08-14）** 是重大版本，7.2.2（2025-09-10）的 changelog 明确写着 "Linux host: Use KVM APIs on kernel 6.16.0 and newer for acquiring/releasing VT-x"。其真实含义是 **Linux 6.16 内核把 KVM 的 `kvm.enable_virt_at_load` 默认改为在模块加载时就申请 VT-x**，导致 VirtualBox/VMware 等其他 hypervisor 再也拿不到 VMX root 权限（社区大量 "can't operate in VMX root mode / disable the KVM kernel extension" 报错）；VirtualBox 的修法是**改走 KVM 提供的 API 去申请/释放 VT-x**，而不是"VirtualBox 把 KVM 当引擎用"。7.2 线的实际主题是 ARM（Windows 11 on Arm、Apple Silicon）与 NVMe/webcam 开源化。

6. **Hyper-V 是微内核型 hypervisor**：微软官方文档描述为 hypervisor + root partition（管理栈与硬件直访）+ child partitions，跨分区通信走 **VMBus**，设备请求由 child 的 VSC 经 VMBus 转给 root 的 VSP；**Enlightened I/O** 让 VMBus 上直接跑 SCSI 这类高层协议、绕过设备模拟层。同一 hypervisor 被复用为 WSL2、Windows Sandbox、VBS/HVCI、Credential Guard 的底座——这是"一个 hypervisor，多种产品形态"的典型。

7. **AWS Nitro 是卸载型架构的样板**。官方白皮书把 Nitro 系统分成三块：**Nitro Cards**（自研 SoC，跑固件，做 VPC/EBS/本地 NVMe 的 I/O 与硬件加解密；主卡叫 Nitro Controller，是硬件信任根）、**Nitro Security Chip**（拦截固件写、控制主 CPU/BMC 复位引脚，裸金属模式下替代 hypervisor 保护固件）、**Nitro Hypervisor**（"intentionally minimized"，无网络栈、无通用文件系统、无外设驱动、无 shell）。I/O 通过 Nitro Card 的 **SR-IOV VF** 直接分给 VM。Nitro Hypervisor 支持在线整机热更新。**"Nitro Hypervisor 基于 KVM"是业界广泛引用的说法，但当前 AWS 官方白皮书未出现 KVM 字样——标注为"广泛报道、官方白皮书未证实"**。

8. **AWS 嵌套虚拟化是"新一代 Intel 实例 + 显式开关"的产品**，不是全线能力。官方文档：仅支持 M7i/M7i-flex/M8i/M8id/M8i-flex、C7i/C7i-flex/C8i/C8id/C8i-flex、R7i/R7iz/R8i/R8id/R8i-flex/X8i、I7i/I7ie；L1 只支持 **KVM 与 Hyper-V**；开启嵌套后 Windows 实例的 VSM/Credential Guard 自动关闭、不支持休眠、CPU >192 的实例不支持；架构分三层 L0（Nitro hypervisor）/L1（你的实例）/L2（实例内的 VM）。AWS 官方仍建议性能敏感场景直接用 `.metal`。

9. **Azure 的卸载卡叫 Azure Boost，且明确"用 Rust 写新代码"**。官方文档：把传统上由 hypervisor 与宿主 OS 做的**网络与存储处理卸载到专用软硬件**（可编程 FPGA），存储卸载后本地盘可达 36 GB/s、660 万 IOPS，远端盘 14 GB/s、75 万 IOPS，网络走自研 **MANA** 网卡、带宽 200 Gbps 起；安全侧用 **Cerberus** 芯片做独立硬件信任根（NIST 800-193）、SELinux 最小权限、FIPS 140 内核、Rust 内存安全作为新代码首选语言（该文档 `ms.date: 03/18/2025`）。机密计算侧提供 AMD SEV-SNP（DCasv5/ECasv5/DCasv6/ECasv6）与 Intel TDX（DCesv6/ECesv6）两套 CVM，并有配 NVIDIA H100 的 NCCadsH100v5。

10. **中国三大云的架构命名与主线已确认**：阿里云 **神龙 X-Dragon**（2017-10 发布首款融合物理机/虚拟机特性的云服务器，2018-05 首次公开架构细节：自研 X-Dragon 虚拟化芯片 + Hypervisor 系统软件 + 服务器硬件架构三件套，定位"性能零损耗、体验与裸金属一致"）；华为云 **QingTian（擎天）**（官方白皮书：2017 年发布基于 QingTian 系统的实例；由自研 **QingTian Cards** + 自研精简 **QingTian Hypervisor** 组成，Cards 提供硬件信任根、固件防篡改、IO 加解密加速与 I/O 直通，一套架构同时支持虚机、裸机、容器，白皮书页面更新于 2025-10）；腾讯云 **CubeSandbox**（2025 年前后开源的 AI Agent 安全沙箱，RustVMM/KVM microVM；仓库内已有笔记见下）。

11. **"安全容器/microVM"是云厂商的共同收敛点**：阿里云 ACK 安全沙箱容器（runV，基于轻量 VM，V2 相对 V1 声称开销降 90%、启动快 3 倍、单机密度提升 10 倍；沙箱启动约 150 ms，而社区 Kata Containers 约 500 ms；部署限制明确——**仅支持 ECS 裸金属实例规格**、集群版本 1.16–1.34、仅 Alibaba Cloud Linux 3）；Google Cloud 走的是**另一条路**：gVisor 不是 VM 也不是 seccomp 过滤器，而是 Go 写的**用户态应用内核**（OCI runtime `runsc`），官方明确把"机器级虚拟化（KVM/Xen via VMM）"和"规则执行（seccomp/SELinux/AppArmor）"列为另外两条路。

12. **Xen 系正在经历所有权与产品形态的重组**。XCP-ng 项目 CEO 公开表态：**8.3 是 8.x 线最后一版**，**即将发布的 9.0 是完全新平台、不再基于 Citrix XenServer**；Vates 团队已超 100 人（2026-08 的官方博客称接近 150 人），比现在的 XenServer 团队更大。Vates 于 2025-10-28 宣布**旧版 Xen Orchestra Starter/Enterprise 订阅于 2026-07-31 退役**，统一到 Vates VMS 捆绑（开源版 Xen Orchestra 不受影响）。即：Xen 生态的商业重心已从 Citrix 转到 Vates。

13. **开源集成/管理层的现状**：Proxmox VE **9.0 于 2025-08-05 发布**，基于 Debian 13 "Trixie"，默认内核 6.14.8-2，QEMU 10.0.2 / LXC 6.0.4 / Ceph Squid 19.2.3 / ZFS 2.3.3，新增厚置备 LVM 共享存储快照、SDN Fabrics（OpenFabric/OSPF）、HA 亲和规则、Rust/Yew 重写的移动端 UI；AGPLv3 开源，订阅从 EUR 115/年/CPU 起。Proxmox VE 9.1 亦已发布（见参考资料）。

14. **云厂商虚拟化演进主线**：全虚拟化 → 半虚拟化 virtio → 硬件辅助 + SR-IOV 直通 → **DPU/IPU 卸载**（AWS Nitro Cards / Azure Boost / 华为 QingTian Cards / 阿里神龙 MOC 卡）→ **microVM 轻量化**（Firecracker、CubeSandbox、ACK 安全容器）→ **机密计算**（Intel TDX / AMD SEV-SNP / NVIDIA 机密 GPU）。这条主线的分水岭是 2017 年：AWS Nitro（C5，2017-11 re:Invent）与华为 QingTian（2017）、阿里神龙（2017-10）几乎同期启动，标志着"hypervisor 变薄、控制面下沉到专用硬件"成为公有云共识。

---

## 类型学

### 1. 四种基本类型（+ 一种云上变体）

| 类型 | 定义 | 典型代表 | 关键判据 |
|---|---|---|---|
| **Type-1 bare-metal** | hypervisor 直接跑在硬件上，先于任何通用 OS 获得控制权 | VMware ESXi、Xen、Hyper-V、Microsoft Hypervisor | 是否需要先启动一个通用 OS |
| **Type-2 hosted** | 作为宿主 OS 上的普通应用运行，依赖宿主提供调度与驱动 | VirtualBox、VMware Workstation/Fusion、QEMU(TCG) | 宿主 OS 崩溃 → VM 崩溃 |
| **KVM 型** | 内核模块提供硬件虚拟化能力，用户态 QEMU 提供设备模型 | KVM + QEMU、Proxmox VE、oVirt、OpenStack | 复用宿主内核的调度器/MMU/驱动 |
| **混合 / 微内核型** | hypervisor 本体极小，管理栈跑在一个特权分区里 | Xen（dom0）、Hyper-V（root partition） | 是否存在"特权 guest" |
| **卸载型（云上变体）** | hypervisor 极简化，I/O 与控制面下沉到专用卡/DPU | AWS Nitro、Azure Boost、华为 QingTian、阿里神龙 | 是否有自研 SoC/FPGA 承担 I/O 与控制 |

### 2. 为什么要单独分出 "KVM 型"

经典二分法的判据是"hypervisor 之下有没有一个通用 OS"。KVM 的答案是模糊的：

- Linux 内核在 KVM 场景下**确实**就是那个"通用 OS"，`modprobe kvm_intel` 之后宿主机还是普通 Linux；
- 但 KVM 又不是 Type-2：VM 不是靠宿主上的一个用户态模拟器跑起来的，`VMXON`/`VMRUN` 是内核直接执行的，QEMU 只是配置与设备模型那一半。

KVM 官方 FAQ 自己的措辞最能说明问题：

> "Xen is an external hypervisor; it assumes control of the machine and divides resources among guests. On the other hand, KVM is part of Linux and uses the regular Linux scheduler and memory management."
> "What is the difference between KVM and QEMU? — QEMU uses emulation; KVM uses processor extensions (HVM) for virtualization."

**结论**：
- 若按"是否接管整机"判据 → KVM **不是** Type-1；
- 若按"是否需要宿主 OS 上的用户态模拟器才能跑 VM"判据 → KVM **不是** Type-2；
- 若按"是否在特权级直接执行硬件虚拟化指令、能否在无宿主 OS 干预下隔离 VM"判据 → KVM **是** Type-1 的等价物（这也是 Red Hat/IBM 等厂商称其为 Type-1 的理由）。
- 工程上建议：**不要说 KVM 是 Type-1 或 Type-2，说它是"Linux 内核内的 hypervisor 层 + QEMU 用户态 VMM"**。这样既不误导，也解释了为什么 KVM 的"hypervisor 攻击面"里包含整个 Linux 内核（这与 ESXi/Nitro Hypervisor 的极小 TCB 形成鲜明对比）。

### 3. hypervisor / VMM / 容器运行时 / microVM monitor 的边界

| 组件 | 职责 | 不负责 | 例子 |
|---|---|---|---|
| **hypervisor** | CPU 虚拟化、内存分页/隔离、中断注入、VM 生命周期原语 | 设备模型、磁盘格式、镜像管理 | KVM 内核模块、Xen 本体、Microsoft Hypervisor、Nitro Hypervisor |
| **VMM** | 设备模型（virtio 等）、VM 配置、启动/暂停/快照、迁移 | 全机资源调度 | QEMU、Firecracker、Cloud Hypervisor、CubeHypervisor（Cloud Hypervisor fork）、VirtualBox 的 VMM |
| **容器运行时** | OCI 生命周期、rootfs 挂载、namespace/cgroup 配置 | 提供第二个内核 | runc、containerd、CRI-O、`runsc`（gVisor） |
| **microVM monitor** | 只保留最小 virtio 设备集的 VMM，为高密度/快启动优化 | 通用设备兼容性、热迁移等富功能 | Firecracker（无 BIOS、无 PCI、极简设备模型） |

**容易混的三处**：
1. **QEMU 被叫成 hypervisor**：QEMU 单独运行（TCG 模式）时是纯模拟器/Type-2 意义上的 VMM，没有硬件加速；`qemu-system-x86_64 --enable-kvm` 时才由 KVM 提供硬件虚拟化。QEMU 是 VMM 那一半。
2. **gVisor 被叫成 VM**：官方文档明确 "gVisor is also not a VM in the everyday sense of the term (e.g. VirtualBox, QEMU)"，它是用户态应用内核，拦截系统调用而不是虚拟化硬件。
3. **Kata/runV 被叫成"容器"**：它们是把 OCI 语义**翻译**到 VM 上的运行时（一个 Pod 一个 microVM），隔离边界是 guest 内核而非 namespace。

---

## 厂商与产品

> 每节统一按：**底层 hypervisor 架构 / 授权与商业模式 / 嵌套虚拟化 / 代表特性 / 生态与现状** 五段写。
> 无法从可靠来源证实的点一律标注「未能证实」。

### 一、VMware（Broadcom）

**底层 hypervisor 架构**
- **ESXi**：Type-1 裸金属 hypervisor，自有内核（VMkernel），不基于 Linux；原生驱动模型 + vMotion 热迁移。
- **Workstation / Fusion**：Type-2 托管型，跑在 Windows/Linux（Workstation）与 macOS（Fusion）上，面向桌面/开发。
- **vSphere**：ESXi + vCenter 的服务器虚拟化套件（管理面）。
- **vSAN**：软件定义存储，与 vSphere 内核集成。
- **NSX**：软件定义网络 / 网络虚拟化叠加层。
- 与 KVM/Xen 不同，VMware 的 hypervisor 与硬件之间有自研 VMkernel 与驱动生态，这是其长期性能口碑的基础，也是迁移成本高的原因。

**授权与商业模式（2023 → 2026 时间线）**
| 时间 | 事件 |
|---|---|
| 2022-05 | Broadcom 宣布拟以约 610 亿美元收购 VMware |
| **2023-11** | **Broadcom 完成对 VMware 的收购** |
| 2023 年底 | 引入**按核订阅制**，替代传统的永久授权 + 按 CPU 授权；推出两个捆绑 SKU：**VMware Cloud Foundation (VCF)** 与 **VMware vSphere Foundation (VVF)**；下架单点永久授权 |
| 2024 全年 | 渠道与合作伙伴体系重整（大量经销商/托管商合同变动），客户开始评估替代方案（XCP-ng、Proxmox VE、Hyper-V、Nutanix 等） |
| **2025-05** | Broadcom 发布新 SPD：VCF 用户**每 180 天必须提交经验证的合规报告**；逾期触发 270 天倒计时，之后开始限制平台功能（**首先封锁 VCF 管理面访问**） |
| 2026 年现状 | 仅订阅制；VCF 含 1 TiB/core 的 vSAN 权益，VVF 含 0.25 TiB/core（且只能在启用 vSAN 的 vSphere 部署中使用）；vCenter / vRealize Operations / VMware Automation 等管理组件被限制为**只能在 VCF 环境内使用**；云上使用被限定在受认可的 MaaS 平台（Azure VMware Solution、Google Cloud VMware Engine、Oracle Cloud VMware Solution） |

**关键授权细节（据 Anglepoint 2025-06-13 文章与 Redress 2026 文章）**
- 永久授权"**封顶在 5.x**"：持有永久授权的客户无法升级到该版本之后的订阅制版本。
- **可移植性受限**：完整 VCF 栈必须整体迁移，不再允许部分组件单独迁移。
- **托管权受限**：VMware 托管权仅允许用于自研应用，且有严格条件。

**嵌套虚拟化**：ESXi 支持向 guest 暴露 VT-x/AMD-V 以运行嵌套 hypervisor，需要在 VM 设置中勾选 "Expose hardware assisted virtualization to the guest OS"（该能力长期存在；**具体版本行为随版本变化，本条未逐版本核对官方文档，标注为「部分未证实」**）。Workstation/Fusion 亦提供 "Virtualize Intel VT-x/EPT or AMD-V/RVI" 选项。

**代表特性**：vMotion、DRS、HA、vSAN、NSX、VCF 的私有云一体化栈（含 Aria 管理组件）、VMware Cloud on 各公有云。

**生态与现状**：Broadcom 之后的核心叙事是"只推 VCF 捆绑、以订阅与合规报告约束客户"。直接后果是替代方案市场活跃：XCP-ng/Vates、Proxmox VE、Harvester、Nutanix AHV、Hyper-V 都在承接 VMware 迁移需求（这一判断来自 XCP-ng 论坛中管理员的第一手迁移讨论，属社区证据而非统计报告）。

---

### 二、Microsoft

**底层 hypervisor 架构**
- **Microsoft Hypervisor**：微内核型。官方架构文档（`ms.date: 01/21/2026`）描述为：
  - **root partition（父分区）**：跑 Windows，管理栈在此，直接访问硬件设备，通过 hypercall API 创建 child partition；
  - **child partition**：无硬件直访，只看到虚拟设备（VDevs），请求经 **VMBus** 或 hypervisor 转发到 root partition；
  - **VMBus**：分区间的逻辑通信通道；root 侧是 **VSP**（Virtualization Service Provider），child 侧是 **VSC**（Virtualization Service Consumer）；
  - **Enlightened I/O**：在 VMBus 上直接跑 SCSI 等高层协议，**绕过设备模拟层**，要求 guest 是 hypervisor/VMBus aware（由集成服务提供）；
  - 中断由 hypervisor 接管并重定向；**IOMMU** 做地址重映射；**SLAT 是 Windows Server 2016+ 的硬性要求**。
- 这个"microkernel + 特权 root partition"结构是 Hyper-V 被归为混合型的原因，也是它安全问题（root partition 在内核态、且有完整 Windows 栈）常被与 Xen dom0 类比的原因。

**同一 hypervisor 的多种产品形态**
- **Hyper-V（Windows Server / Windows 客户端 Pro）**：完整服务器虚拟化。
- **Windows Hypervisor Platform (WHP)**：把 Microsoft Hypervisor 作为 API 暴露给第三方 VMM（如 QEMU、Android 模拟器）使用。
- **WSL2**：微软官方将发行版跑在一个**轻量级 utility VM**（轻量实用虚拟机）里，共享同一 hypervisor 与内核。
- **Windows Sandbox**：Windows 10/11 专业版及以上，一次性、隔离的桌面环境，同样基于 Hyper-V 技术栈。
- **VBS / HVCI（基于虚拟化的安全 / Hypervisor 强制代码完整性）**：用 hypervisor 制造 VTL（虚拟信任级别），把代码完整性检查、LSASS 凭据保护（Credential Guard）挪到比内核更高的特权层，以抵御内核级攻击。代价是嵌套虚拟化/第三方 hypervisor 共存场景下的兼容性问题（AWS 官方文档就写明：Windows 实例开启嵌套虚拟化后 VSM 自动关闭）。

**授权与商业模式**：Hyper-V 不单独售卖，随 Windows Server（标准版含 2 个 OSE，数据中心版不限）与 Windows 10/11 Pro/Enterprise 授权；Azure 侧按 VM 计费。**"Hyper-V Server"（免费独立 hypervisor SKU）已被微软终止**（该产品线在 2022 年后进入退役流程；本条未在本次核查中取得官方页面，标注「未逐条证实」）。

**嵌套虚拟化**
- Hyper-V 自身支持嵌套（guest 内再跑 Hyper-V），要求宿主机为 Windows Server 2016+，并需在 VM 上开启 `ExposeVirtualizationExtensions`（该 cmdlet 长期存在）。
- Azure 上嵌套虚拟化按 VM 系列支持（见下）。

**Azure（底层 Hyper-V）**
- **底层**：Azure 计算节点跑 Microsoft Hypervisor（即 Hyper-V 的技术血缘）。
- **Azure Boost**（官方文档 `ms.date: 03/18/2025`）：把传统上由 hypervisor 与宿主 OS 执行的网络/存储处理**卸载到专用软硬件**。
  - 网络：自研 **MANA（Microsoft Azure Network Adapter）** 网卡，>200 Gbps，原生支持 DPDK，active/active 上行；
  - 存储：卸载到**可编程 FPGA**，暴露 NVMe 接口；远端盘最高 14 GB/s、75 万 IOPS；本地/缓存盘最高 36 GB/s、660 万 IOPS（Azure Boost SSD）；
  - 安全：**Cerberus** 芯片做独立硬件信任根（NIST 800-193）、Secure Boot + Attestation（不可信主机不得承载负载）、SELinux 最小权限、FIPS 140 内核、**Rust 为所有新代码的首选语言**；
  - 价值主张：把 CPU 还给 guest；对 Azure Dedicated Host 用户可提升单机 VM 密度。
- **机密计算**（Azure Confidential VM options 官方文档）：
  - AMD 系：**AMD SEV-SNP**（第三代 EPYC 引入）→ DCasv5/DCadsv5/ECasv5/ECadsv5、DCasv6/DCadsv6/ECasv6/ECadsv6；
  - Intel 系：**Intel TDX**（第五代 Xeon 引入）→ DCesv6/DCedsv6/ECesv6/ECedsv6；
  - **NCCadsH100v5**：AMD SEV-SNP + NVIDIA H100，即机密 GPU 实例；
  - 官方口径：无需改代码；内存优化型 CVM 提供双倍 vCPU 内存比。
- **嵌套虚拟化支持情况**：Azure 在部分 Dv3/Ev3 及之后的系列上支持嵌套虚拟化用于开发测试（如 Docker Desktop、WSL2、模拟器），官方文档长期以"特定系列 + 特定代"的形式列出。**具体支持矩阵本次未逐条核对，标注「未能完整证实」**。
- **GPU 虚拟化**：**NVv4** 系列使用 AMD 的 GPU 分区（GPU-P，硬件分区把单块 GPU 切给多个 VM，粒度小于完整直通），面向 VDI/图形工作站；另有 ND/NV 系列做完整 GPU 直通与 InfiniBand（**NVv4 的具体分区实现细节本次未取得官方页面逐条证实**）。

**代表特性**：与 Windows 生态/AD 深度集成、WAC（Windows Admin Center）管理、Storage Spaces Direct、Shielded VM、VBS/HVCI、Azure Arc/Azure Local 混合。

**生态与现状**：Hyper-V 是"已有 Windows Server 数据中心授权客户的最省事替代方案"——这也是社区迁移讨论中的主流论点；同时它的短板是管理体验与生态工具链比 vSphere 弱。

---

### 三、Oracle

**VirtualBox**
- **架构**：Type-2 托管型。核心是自研 VMM（`VBoxVMM`）+ 可插拔的虚拟化引擎：x86 上用 VT-x/AMD-V（`HM` 模块），在 Windows 上还能选择**用 Windows Hyper-V 作为虚拟化引擎**（changelog 多处提到 "when using Windows Hyper-V as the virtualization engine"）。ARM 主机（Apple Silicon、Windows on Arm）用 Arm 虚拟化扩展。
- **版本时间线（本次核查，来源：virtualbox.org News 与 Changelog-7.2）**：
  | 版本 | 日期 | 要点 |
  |---|---|---|
  | 7.0.0 | 2022-10-10 | 7.0 线首发 |
  | 7.1.0 | 2024-09-11 | 7.1 线首发 |
  | **7.2.0** | **2025-08-14** | 重大版本：Windows/Arm 主机支持 Arm 虚拟化、Arm 主机可跑 Windows 11/Arm、Linux 主机 3D 开启时视频解码加速、macOS Arm 主机实验性 3D（DXMT）、**NVMe 控制器模拟进入开源基础包**、修复 Intel CPU 嵌套虚拟化；Arm VM 快照与 7.1 不兼容 |
  | **7.2.2** | **2025-09-10** | **"Linux host: Use KVM APIs on kernel 6.16.0 and newer for acquiring/releasing VT-x"**；USB Webcam 进入开源基础包 |
  | 7.2.4 | 2025-10-21 | 维护版 |
  | 7.2.6 / 7.1.16 | 2026-01-20 | 维护版 |
  | 7.2.8 / 7.1.18 | 2026-04-21 | 维护版 |
  | 7.2.10 / 7.2.12 | 2026-06-16 / 06-30 | 维护版 |
  | 7.2.14 | 2026-07-21 | 支持 Linux 内核 7.2；Windows Hyper-V 引擎下不再对无 xsave 的 CPU 读写 xcr0 |
  | 7.2.16 | 2026-08-18 | 支持 FRED；Wayland 剪贴板改进 |
  | **7.2.18** | **2026-09-15** | 最新维护版（本次核查前一日） |
- **"VirtualBox 7.x 在 Linux 上改用 KVM 作为后端"——核实结论**：
  - **已证实**：7.2.2 起，在 **Linux 内核 6.16.0 及以上**，VirtualBox 会**使用 KVM 提供的 API 来申请/释放 VT-x**。
  - **已证实的背景**：Linux 6.16 把 KVM 的 `kvm.enable_virt_at_load` 行为改为**在模块加载时即初始化虚拟化**（VirtualBox 7.1 changelog 里就有该注意事项，建议用 `kvm.enable_virt_at_load=0` 或卸载 `kvm_XXX` 模块）。这直接导致其他 hypervisor 拿不到 VMX root（社区大量 "VirtualBox can't operate in VMX root mode. Please disable the KVM kernel extension" 报错，LWN bug 219602 亦记录"kvm.enable_virt_at_load 的默认值破坏了其他虚拟化方案"）。
  - **不能证实**：VirtualBox 把 KVM 当作完整虚拟化后端（即 VM 由 KVM 执行、VirtualBox 只做前端）。**changelog 措辞限于 VT-x 的 acquire/release**。有德语媒体标题称 VirtualBox 获得"实验性 KVM 支持"，但本次未取得其正文（heise 页面抓取失败），**标注「未能证实」**。
  - **表述建议**：写"VirtualBox 7.2.2 起在 Linux 6.16+ 上通过 KVM API 协调 VT-x 占用"，不要写"VirtualBox 改用 KVM 后端"。
- **授权与商业模式**：**VirtualBox 基础包（Base Package）开源，GPLv3**；**Oracle VM VirtualBox Extension Pack** 为专有许可（个人/教育免费，商业使用需授权），提供 USB 2.0/3.0、RDP、磁盘加密、PXE 等。7.2 起 USB Webcam 与 NVMe 控制器模拟并入开源基础包。Oracle 同时提供商业支持订阅。
- **嵌套虚拟化**：支持（需在 VM 设置里开启对应选项）；7.2.0 明确修复了 Intel CPU 上的嵌套虚拟化。**macOS/Apple Silicon 与 Windows Hyper-V 引擎组合下的嵌套行为未逐条证实**。
- **生态与现状**：桌面/教学/开发场景的事实标准之一，但性能与生产级管理能力弱于 ESXi/KVM 系；7.2 的重心明显转向 Arm 主机（Apple Silicon、Windows on Arm）。

**Oracle VM / Oracle Linux KVM**
- **Oracle VM Server for x86（OVM）** 已进入退役（Oracle 已停止 x86 平台上的 Oracle VM 支持，主推 Oracle Linux KVM 与 Oracle Cloud 上的虚拟化）。**本次未取得官方退役公告页面，标注「未能证实」。**
- **Oracle Linux KVM**：Oracle Linux 上的 KVM 栈（含 `qemu-kvm`、`libvirt`），Oracle 提供 **Oracle Linux Virtualization Manager (OLVM)** 作为 oVirt 血统的管理平台。Oracle 在 OCI 上提供 "Oracle Linux KVM Image" 等镜像（OCI 文档中部分条目标注 EOL）。
- Oracle Cloud Infrastructure (OCI) 自身的裸金属与 VM 能力以自研 + KVM 系技术为主（**具体自研组件本次未核实**）。

---

### 四、Xen 系

**Xen Project（开源）**
- **架构**：Type-1，但采用**微内核 + dom0 模型**：Xen 本体极小（调度、内存、中断），第一个创建的域 **dom0** 是特权域，承载真实设备驱动（或使用 **driver domain / driver stub domain** 把驱动隔离出 dom0），通过 **grant table / event channel / XenBus** 与其他域通信。
- **虚拟化模式**：PV（半虚拟化，需改内核）、HVM（硬件辅助）、**PVH**（HVM 容器 + PV 驱动，现代默认方向）、PV-on-HVM。
- **嵌套/直通**：支持 PCI passthrough、SR-IOV；嵌套（nested Xen）在部分平台上支持。
- **形态**：Xen 4.x 系列持续发布，`Xen Project 4.20 Feature List` 在 Xen wiki 上维护（**本次未取得具体发布日期，4.20 的存在已证实，细节标注「未能证实」**）。
- **现状**：作为基础技术仍在（汽车、嵌入式、云底座），但作为通用企业虚拟化平台的份额被 KVM 与 ESXi 挤压。

**Citrix Hypervisor / XenServer**
- Citrix 的商业 Xen 发行版（XenServer → Citrix Hypervisor）。Citrix 被 Cloud Software Group 收购后，XenServer 团队规模缩小（对照 Vates 方面的公开说法：Vates 人数"比现在的 XenServer 团队更大"）。
- **本次未取得 XenServer 当前版本的官方发布与授权页面，现状标注「未能证实」**。

**XCP-ng（Vates）**
- XCP-ng 8.x 源自 XenServer 的开源分支（xapi 管理栈 + Xen hypervisor），由 Vates 主导。
- **已证实的关键状态（来源：XCP-ng 论坛 Vates CEO 公开回复 + Vates 官方博客）**：
  - **8.3 是 8.x 线的最后一版**，仍在维护（自有更新与 backport）；
  - **即将发布的 9.0 是完全新平台，"not based on Citrix XenServer at all"**；
  - XCP-ng 已**完全独立于 Citrix 多年**，有自己的工程、打包、QA、安装器，并对 Xen 上游有贡献；
  - Vates 团队规模已超 100 人（2026-08 官方博客称"接近 150 人"）；
  - 商业订阅按**物理主机数**计价（不按 socket/core/VM），把 hypervisor + 管理（Xen Orchestra）+ 备份/DR 打包为 **Vates VMS**（Essential / Essential+ / Pro / Enterprise）；
  - **旧版 Xen Orchestra Starter / Enterprise 订阅于 2026-07-31 退役**；XO Premium 继续有效；**开源版 Xen Orchestra 完全不受影响**；
  - 技术路线：改进迁移逻辑、现代 guest 与 vTPM 支持、**新增 qcow2 存储格式**、扩大硬件兼容。
- **代表产品**：XCP-ng（hypervisor）+ Xen Orchestra / XOA（管理 + 备份）+ XOSTOR（超融合存储）+ XO Proxy。
- **嵌套虚拟化**：Xen 支持嵌套（需硬件辅助），XCP-ng 上的具体支持度与开启方式**未逐条证实**。

**Amazon 早期 EC2 用 Xen**
- EC2 早期（2006 起）基于 **Xen** hypervisor（PV 起步，后转 HVM）。2017 年 C5 实例（re:Invent 2017）引入 **Nitro** 架构取代 Xen，此后新实例类型逐步全部迁移到 Nitro；老实例类型（如部分 Xen 时代机型）陆续退役。**"EC2 全线完成 Xen → Nitro 迁移"的确切时间点未取得官方页面，标注「未能证实」**。

**Xen 在汽车/嵌入式**
- **Xen on ARM**：Xen 在 ARMv8 上支持（含 `null` 调度器等实时配置），被用于汽车（如作为虚拟机管理程序承载仪表盘/娱乐域）、嵌入式与安全隔离场景，也是 Automotive Grade Linux 相关讨论中的常见选项。
- 相关汽车/嵌入式项目（如 COQOS、ACRN、Jailhouse 等）中部分与 Xen 同源或同类，**本次未逐一核实，标注「未能证实」**。

---

### 五、QEMU

**定位：QEMU 是 VMM（虚拟器/模拟器），不是 hypervisor。**

- **两种模式**：
  1. **TCG（Tiny Code Generator）纯模拟**：动态二进制翻译，可在任意架构上模拟任意架构（如 x86 上跑 ARM 系统）。速度慢（数量级），但可跨架构、可调试内核。**无需任何硬件虚拟化支持**。
  2. **硬件加速模式**：`-accel kvm`（Linux/KVM）、`-accel whpx`（Windows Hypervisor Platform）、`-accel hvf`（macOS Hypervisor.framework）、`-accel hax`（旧 Intel HAXM）。此时 CPU 虚拟化由平台 hypervisor 执行，QEMU 负责设备模型、ROM/固件加载、磁盘格式、迁移。
- 因此 **QEMU 的架构身份取决于加速后端**：TCG 时它是模拟器 + Type-2 意义上的 VMM；`-accel kvm` 时它是 KVM 的用户态 VMM 前端。
- **关键结论**："QEMU/KVM"是一个组合名词，不是两个 hypervisor。KVM 官方 FAQ 的表述可以直接引用：
  > "KVM uses a slightly modified QEMU program to instantiate the virtual machine. Once running, a virtual machine is just a regular process."
  > "QEMU uses emulation; KVM uses processor extensions (HVM) for virtualization."
- **QEMU 的生态价值**：设备模型（virtio-blk/net/scsi/fs/gpu、NVMe、VGA/VirtIO-GPU）、镜像格式（qcow2、raw、VMDK、VHDX…）、快照/迁移、`libvirt` 的默认后端。绝大多数 KVM 系产品（Proxmox VE、oVirt、OpenStack、Oracle Linux KVM）都以 QEMU 为 VM 进程。
- **现代分化**：microVM 场景出现了 **Firecracker**、**Cloud Hypervisor**、**crosvm**、**CubeHypervisor** 等更小的 VMM，共享 **rust-vmm** 组件（`kvm-ioctls`、`vm-memory`、`virtio-queue`），它们**不用 QEMU**，但沿用同一套 virtio 语义。这是"VMM 层被重新实现"的技术主线。
- **嵌套虚拟化**：QEMU 本身不提供嵌套能力，是否可嵌套取决于所依赖的 hypervisor（KVM 的 nested、Hyper-V 的 nested、ESXi 的 VHV 等）。

---

### 六、集成与管理平台

**Proxmox VE（Proxmox Server Solutions GmbH）**
- **架构**：Debian 为基础，**KVM + QEMU 跑 VM，LXC 跑容器**；自带 Web UI、HA、集群（corosync）、Ceph、ZFS。
- **版本**：**9.0 于 2025-08-05 发布**（官方新闻稿）：基于 **Debian 13 "Trixie"**，默认内核 **6.14.8-2**，**QEMU 10.0.2 / LXC 6.0.4 / Ceph Squid 19.2.3 / ZFS 2.3.3**。
- **代表特性（9.0）**：厚置备 LVM 共享存储（iSCSI/FC SAN）快照（volume chain 实现）、**SDN Fabrics**（OpenFabric / OSPF、spine-leaf、多路径与自动故障切换）、HA 资源亲和规则、Rust/Yew 重写的移动端界面。
- **授权商业模式**：**AGPLv3 开源免费**；企业支持订阅提供 Enterprise 仓库与技术支持，**EUR 115/年/CPU 起**；官方数据：1.6M+ 主机、31+ 语言、论坛 22.5 万+ 成员、公司 2005 年成立于维也纳。
- **嵌套虚拟化**：KVM 支持 nested（宿主 `kvm_intel nested=1` / `kvm_amd nested=1`），Proxmox 需在 VM 的 CPU 类型选 `host` 并手动开启嵌套参数（**未逐条核对官方 wiki，标注「未证实细节」**）。
- **9.1**：`Proxmox VE 9.1 Release` 已发布（见参考资料；本次未逐条核对发布内容）。

**OpenStack（libvirt/KVM 编排层）**
- **定位**：IaaS 控制面（Nova 计算、Neutron 网络、Cinder 存储、Glance 镜像、Keystone 认证）。**不是 hypervisor**。
- 默认通过 **libvirt** 驱动 **QEMU/KVM**；也支持 Xen、Hyper-V、VMware vSphere、PowerVM 等其他 virt driver。
- 因此 OpenStack 是"KVM 型虚拟化的编排层"最典型的代表，也是私有云中最常见的开源栈。

**oVirt**
- Red Hat 主导的开源虚拟化管理平台（engine + VDSM），基于 KVM/QEMU/libvirt，管理界面与 RHV（Red Hat Virtualization）同源。
- **RHV 已进入生命周期终点（EOL）**，Red Hat 转向 OpenShift Virtualization（KubeVirt）。**本次未取得官方 EOL 公告，标注「未能证实」**。
- oVirt 上游项目仍在，Oracle Linux Virtualization Manager 为其下游衍生。

**其他集成/管理平台（本次未展开，仅列名）**：Nutanix AHV（自有 KVM 衍生 hypervisor + Prism 管理）、Harvester（SUSE，KubeVirt 系）、KubeVirt/OpenShift Virtualization、Xen Orchestra、Virtuozzo、SmartX、ZStack 等。

---

### 七、云厂商自研架构（重点）

#### 7.1 AWS —— Nitro System / Firecracker

**Nitro 架构（来源：AWS 官方白皮书 "Security design of the AWS Nitro System"）**

三个组件：

1. **Nitro Cards**：自研 SoC（由 2015 年收购的 **Annapurna Labs** 设计），带专用 ASIC，**独立于承载客户负载的主板运行**。
   - 承担 EC2 对外的全部控制接口，以及全部 I/O 接口（软件定义网络、EBS、实例存储）；
   - 主卡叫 **Nitro Controller**：系统硬件信任根，管理其他组件的固件，固件存放在挂在 Controller 上的加密 SSD 上（密钥由 TPM + SoC secure boot 保护）；是服务器与 EC2/EBS/VPC 控制面之间的**唯一网关**，对控制面暴露强认证加密的 API（已用形式化方法证明其控制消息解析实现的内存安全）；
   - 其他专用卡：**Nitro Card for VPC / for EBS / for Local NVMe Storage**，在 SoC 内做硬件加解密与密钥安全存储；最近三代 VPC 卡可**透明加密 EC2 实例间的 VPC 流量**（AES-256-GCM，无性能损失）；
   - 密钥明文只存在于 Nitro Card 的受保护易失内存中，**AWS 运维人员与主机 CPU 上的任何代码都不可访问**；
   - 对主机暴露的编程接口：NVMe（块存储）、**ENA**（网络）、串口（带外控制台）。
2. **Nitro Security Chip**：插在主板上的器件，运行时**拦截并审查所有对本地非易失存储与低速管理总线（SPI/I2C）的操作**，即对所有固件的写入；位于 BMC 与主 CPU 之间的 PCIe 通路上，可对 BMC 接口做逻辑防火墙；**控制主 CPU 与 BMC 的物理复位引脚**——Nitro Controller 先完成自身安全启动、验完 BIOS/BMC 固件完整性，才释放 CPU。其关键作用之一：**裸金属模式下没有 hypervisor，靠它保证客户无法篡改系统固件**。
3. **Nitro Hypervisor**：被刻意最小化的组件。
   - 只做三件事：接收来自 Nitro Controller 的 VM 管理命令（start/stop 等）、用 CPU 硬件虚拟化特性划分 CPU/内存、把 Nitro 硬件接口的 **SR-IOV 虚拟功能（NVMe for EBS/实例存储、ENA 网络）**分配给对应 VM；
   - 设计上**没有网络栈、没有通用文件系统、没有外设驱动**，不是通用系统，**没有 shell、没有任何交互式访问方式**；
   - 作为签名固件存放在 Nitro Controller 的加密存储上，由 Controller 通过一个只读 NVMe 设备"注入"到主板（相当于 boot drive）；
   - 支持**整机在线热更新**（in-place 替换运行中的 hypervisor 代码，客户实例几乎无感知）；
   - AWS 的定位：卸载 I/O 与精简 hypervisor 职责，同时带来性能、隔离安全，以及**让 hypervisor 成为可选组件——这正是裸金属实例类型得以存在的原因**。
- **"Nitro Hypervisor 基于 KVM"**：这是业界与媒体广泛引用的说法（AWS 工程师在早期 re:Invent 演讲中提过其基于 KVM 定制）。**当前官方白皮书未出现 "KVM" 字样，本稿标注为「广泛报道，官方白皮书未证实」。**

**Firecracker（microVM）**
- AWS 开源的 microVM monitor（Rust + KVM），用于 **AWS Lambda** 与 **AWS Fargate** 的隔离执行环境；设计目标：极小设备模型（约 3 类 virtio 设备）、无 BIOS、无 PCI、毫秒级启动、高密度。
- 与 Nitro 的关系：Firecracker 跑在已由 Nitro 卸载了 I/O 的实例内部，上层是"每个函数/任务一个 microVM"的进一步细分。
- 生态影响：Firecracker 与 crosvm/Cloud Hypervisor 一起催生了 **rust-vmm** 共享组件生态，后续几乎所有新 microVM 项目（含腾讯 CubeSandbox、AgentENV）都在其技术谱系内。

**裸金属 `.metal` 与嵌套虚拟化**
- `.metal` 实例：**不加载 Nitro Hypervisor**，客户拿到整机 CPU/内存，但仍受 Nitro Cards/Security Chip 的 I/O 与固件保护。这使 `.metal` 天然"能跑任意 hypervisor"。
- **虚拟实例上的嵌套虚拟化**（来源：AWS 官方用户指南）：
  - 定义 L0（物理基础设施 + Nitro Hypervisor）/ L1（你的实例，跑 hypervisor）/ L2（实例内的 VM）；
  - Nitro System **把 Intel VT-x 等处理器扩展透传给实例**；
  - **支持的实例类型**：M7i / M7i-flex / M8i / M8id / M8i-flex；C7i / C7i-flex / C8i / C8id / C8i-flex；R7i / R7iz / R8i / R8id / R8i-flex / X8i；I7i / I7ie；
  - **L1 只支持 KVM 与 Hyper-V**；
  - Windows 限制：VSM/Credential Guard 自动禁用、不支持休眠、CPU >192 的实例（如 m8i.96xl）不支持；
  - **不额外收费**；
  - AWS 仍建议对硬件虚拟化扩展性能敏感的场景使用裸金属实例。
- **ENA / NVMe 卸载**：网络与块存储的 I/O 由 Nitro Card 承担并直接以 VF 形式分配给实例（见上）。这是"SR-IOV 直通"在云上的规模化形态。

**AWS 其他虚拟化相关**：Outposts（Nitro Security Key + TPM 的本地形态）、Nitro SSD、Graviton（自研 Arm CPU）、Trainium/Inferentia（自研加速器，由 Nitro Hypervisor 分配给 VM）、EC2 Mac 实例（Nitro Controller 与 Mac Mini 共置、用 Thunderbolt 连接）。

#### 7.2 Azure

见「二、Microsoft」中的 Azure 部分（Hyper-V 底座、Azure Boost、SEV-SNP/TDX 机密计算、NVv4、Azure Local / Azure Arc 混合平台、Azure VMware Solution 作为 VCF 的受认可 MaaS 之一）。

补充：
- **Shielded VM / Trusted Launch**：vTPM + Secure Boot + 启动完整性度量，面向 VM 启动链的完整性保护（**具体 GA 状态与代际本次未核对，标注「未能证实」**）。
- **Confidential Containers / AKS**：在 AKS 上提供机密计算节点（Intel SGX / AMD SEV-SNP）与机密容器方案（**本次未取得官方页面，标注「未能证实」**）。
- **Microsoft 自研 DPU**：2024 年 Microsoft Ignite 上微软公布自研 **DPU** 与数据中心安全芯片计划（Azure Boost 硬件路线的延续）。来源为媒体报道（SDxCentral、StorageReview 等），**微软官方产品页本次未取得，标注「部分未能证实」**。

#### 7.3 阿里云 —— 神龙（X-Dragon）/ 安全容器 / 函数计算

**神龙架构 X-Dragon**
- **公开时间线**：
  - **2017-10**：阿里云发布首个"融合物理机与虚拟机特性"的跨界云服务器（即神龙），背景技术即神龙架构；
  - **2018-05-16**：神龙技术架构**首次全方位公开**（阿里云开发者社区文章，含"开箱直播"）：自研三件套 —— **X-Dragon 虚拟化芯片**（在芯片层解决虚拟机与物理机体系结构不一致的问题，让两者在系统软件层面 100% 兼容）+ **X-Dragon Hypervisor 系统软件** + **X-Dragon 服务器硬件架构**；统称 **MOC 卡**（媒体与社区中"MOC 卡"指该自研虚拟化卡）；
  - 现场测试口径：`ebmhfg5.2xlarge`（8 vCPU）在 Superπ 10000 位为 1 分 26 秒，比基于 Nitro 的 AWS c5.2xlarge（8 vCPU）快 15%（**厂商自测口径，非第三方基准**）；
  - 定位表述：性能"零"损耗、上云"零"障碍、100% 兼容阿里云产品生态；**不只支持 x86，还支持 ARM / Power / 国产 CPU**；
  - 当时已商业化售卖弹性裸金属服务器，支持 8/16/32/96 核与 3.7–4.1 GHz 高主频实例；
  - **2018 之后的代际演进（第八代/第九代等）与 MOC 卡最新规格，本次未取得权威官方页面，标注「未能证实」**；百度百科有"神龙架构"词条可作线索但不宜作为唯一来源。
- **能力集合**：弹性裸金属服务器（EBM）、与 ECS 一致的镜像/网络/存储产品体验、异构算力支持。

**安全容器（ACK Sandboxed Containers / runV）**——来源：阿里云官方帮助文档（英文页 Last Updated: **Jun 15, 2026**）
- 定位：在**轻量 VM** 中运行 Pod，每个沙箱有**独立的 guest 内核**，容器逃逸不影响宿主机；场景为不可信应用隔离、故障隔离、性能隔离、多租户隔离。
- 与社区 **Kata Containers** 的对比（官方口径）：
  | 维度 | ACK 安全容器 V2 | 社区 Kata Containers |
  |---|---|---|
  | 沙箱启动速度 | 约 **150 ms** | 约 **500 ms** |
  | 额外开销 | 低 | 高 |
  | 容器 RootFS | virtio-fs（四星） | 9pfs（一星）/ virtio-fs（四星） |
  | 网络插件 | Terway（比 Flannel 提升 20–30%，支持 NetworkPolicy/限速）、Flannel | Flannel |
  | 监控告警 | 增强的沙箱 Pod 磁盘/网络指标、默认对接云监控 | 缺少沙箱 Pod 磁盘/网络指标 |
  | 稳定性 | 官方给最高评级 | 官方给较低评级 |
- **V2 相对 V1**：在保持强隔离的同时**开销降低 90%、启动速度提升 3 倍、单机密度提升 10 倍**。
- 卷挂载：通过 virtiofs 挂载/共享 NAS、云盘、OSS Volume；NAS 还支持**直接挂载到沙箱**；云盘卷不支持在线扩容、容器 I/O 监控、block/raw 设备、队列设置；NAS 不支持 Samba 挂载卸载、回收站、Quota、容量/I/O 监控、在线扩容。
- **限制（重要，说明该能力依赖裸金属）**：**仅支持 ECS 裸金属实例规格**；仅 ACK 托管版与专有版；集群版本 1.16–1.34；不支持自定义镜像；OS 在 1.30 之前可用 Alibaba Cloud Linux 2/3、1.30+ 仅 Alibaba Cloud Linux 3。
- 运行时接口：支持 Kubernetes **RuntimeClass**（`runC` 与 `runV`）。**血统**：runV 是 Kata Containers 的前身之一（runV + Clear Containers 合并为 Kata），ACK 安全容器是这条血统上的自研分支。

**函数计算 FC 的隔离方案**
- 阿里云函数计算以"沙箱"隔离每次函数调用；**其隔离技术是否为 microVM（如 Firecracker 类）或安全容器，本次未取得官方技术白皮书，标注「未能证实」**。可确证的是阿里云在 Serverless/Agent 场景下有基于安全容器的沙箱化产品方向（如相关 Agent 沙箱工程化讨论文章），但**具体内核隔离机制未证实**。

#### 7.4 华为云 —— QingTian（擎天）/ 鲲鹏 / DPU / StratoVirt·iSula

**QingTian（擎天）架构**——来源：华为云《QingTian 系统安全技术白皮书》（官方文档页更新于 **2025-10-16/17 GMT+08:00**）
- **定位**：华为云新一代**软硬协同**架构，核心能力口径为"**零资源预留、零算力损失、零业务抖动、强安全隔离**"。
- **时间线**：**2017 年华为云发布基于 QingTian 系统的实例**（这是本次取得的最早官方时间点）；此后 QingTian 系统"重塑了华为云基础设施，当前已成为新一代实例的主流底层平台"。
- **组成**（两大自研件）：
  1. **QingTian Cards**：华为云自研专有硬件加速设备，提供**整机系统控制与 I/O 虚拟化直通**，**独立于前端主机系统运行、独立供电**；提供基于硬件的**信任根**（安全启动 + 可信度量 + 固件防篡改）与**基于硬件的 IO 加解密加速**；通过标准 **PCIe** 与主机 CPU 对接，并通过驱动把本地/网络资源模拟为主 CPU 的本地资源；用**专用 ASIC** 处理存储、网络等虚拟化功能，"不仅提升性能，还降低成本"；
  2. **QingTian Hypervisor**：**精简的轻量化虚拟机管理程序**，提供强资源隔离与安全性，且"提供的虚机服务与裸金属服务器几乎无差别的超高性能"。
- **关键设计主张**：一套 QingTian 架构**同时支持虚机、裸机、容器**等多种形态与多元算力；大幅减少云基础设施底座与不同算力的适配工作，提升迭代速度；**更彻底地实现云基础设施与客户应用之间的安全隔离**。
- **与 AWS Nitro / 阿里神龙的可比性**：三者结构高度同构（自研卡/芯片 + 极简 hypervisor + SR-IOV/直通 + 硬件信任根），这是公有云"卸载型架构"收敛的直接证据。
- 华为云曾发布"云原生产业白皮书、云原生 2.0 全景图和行动计划"，以及在 Intel 发布会上宣讲擎天架构（见参考资料，**具体内容本次未展开**）。

**鲲鹏 / 多元算力**
- 华为云 ECS 提供基于**鲲鹏（Kunpeng，Arm）**的云服务器规格族，与 x86 规格并存；QingTian 架构自称同时支持多种算力形态。
- **鲲鹏云服务器与 QingTian 的具体组合方式与代际未逐条核实**。

**DPU 卸载**：QingTian Cards 即是华为云的 DPU 形态（专用 ASIC 做存储/网络虚拟化与加解密）——这一点由官方白皮书直接支持（见上）。

**StratoVirt / iSula（openEuler）**
- **StratoVirt**：openEuler 社区的 Rust 轻量级 VMM（**继承 rust-vmm + KVM 技术路线**），对标 Firecracker/Cloud Hypervisor；**iSula**：openEuler 的容器引擎。两者与华为云商业产品的关系需要区分——它们是**社区/开源项目**；openEuler 社区在 2023 年举办过 Virt Meetup（见参考资料）。
- **StratoVirt 是否作为华为云商用产品的虚拟化底座，本次未取得官方说明，标注「未能证实」**。

**华为虚拟化产品线（补充）**
- 除公有云 ECS 外，华为还有 **FusionCompute / FusionSphere**（私有云虚拟化平台，基于 KVM 系）、**华为云 Stack**（混合云）。**本次未取得官方版本与现状页面，标注「未能证实」**。

#### 7.5 腾讯云 —— CubeSandbox / 星星海 / TKE 安全容器

**CubeSandbox（Agent 沙箱）**
- **性质**：腾讯云开源的 **AI Agent 安全沙箱**服务。
- **技术路线**（来源：仓库内既有笔记 `docs/rust-kunpeng/cubesandbox-rust-analysis.md`、`docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`；以及腾讯云国际站新闻 "Tencent Cloud Cube Sandbox Goes Fully Open-Source"）：
  - **rust-vmm + KVM** 构建 microVM；
  - VMM 为 **CubeHypervisor**（**Cloud Hypervisor 的 fork**，自研，功能较全：virtio-blk/net/fs、VFIO、热迁移、TDX/Hyper-V 支持）；
  - 冷启动 **<60 ms**、单实例内存开销 **<5 MB**；Rust 代码占比约 **48%**；
  - CoW 路径：**磁盘 CoW**（XFS `FICLONE` reflink，O(1) 元数据级克隆）；
  - 生态接入：containerd **Shim v2** + Kata agent、ttrpc/vsock；兼容 **E2B SDK 协议**（换 URL 即可迁移）；
  - 与 AgentENV 的差异：AgentENV 用 Firecracker + 内存 CoW + 分布式集群；CubeSandbox 用自研 VMM + 磁盘 CoW + 单节点为主。
- **开源与版本**：仓库 `TencentCloud/CubeSandbox`，**Apache 2.0**；腾讯云国际站发布"全面开源、五大突破"的消息（**该页面本次抓取未取得正文，仅取得标题与站点存在性；发布日期未证实**）。
- **嵌套虚拟化**：**未取得说明，标注「未能证实」**（CubeSandbox 自身是 L1 上的 hypervisor，是否需要底层嵌套能力取决于部署形态）。

**星星海（Star Lake / 自研服务器）**
- 腾讯云自研服务器品牌"星星海"，用于其数据中心的高密度/定制化机型（含自研 AMD 定制 CPU 平台等）。
- **本次未取得官方技术页面（含与虚拟化/卸载卡的结合方式），标注「未能证实」**。

**TKE 的安全容器方案**
- 腾讯云 TKE（Tencent Kubernetes Engine）提供安全容器/沙箱能力（社区一般对应 **Kata Containers** 路线）。
- **本次未取得 TKE 安全容器的官方技术文档页面，隔离实现细节标注「未能证实」**。

**腾讯云虚拟化整体**：CVM 底层为 KVM 系；**具体自研 hypervisor/卸载卡（如是否有类 Nitro 的 DPU 形态）本次未取得官方资料，标注「未能证实」**。

#### 7.6 Google Cloud

**底层 hypervisor**
- Google Compute Engine 的 hypervisor 是 **KVM**（Google 长期使用 KVM，并有自研的调度/管理组件与部分自研内核/设备组件）。
- **"GCP 在 KVM 之上自研了哪些具体组件"本次未取得官方白皮书，标注「未能证实」**。

**gVisor（用户态内核沙箱）**——来源：gVisor 官方文档
- **定位**：**应用内核（application kernel）**，用 **Go（内存安全语言）** 实现"类 Linux 接口"，**运行在用户态**。
- **不是什么**（官方明确排除）：不是 seccomp-bpf 那样的**系统调用过滤器**，不是 firejail/AppArmor/SELinux 那样的 **Linux 隔离原语包装**，也**不是日常意义上的 VM**（官方点名 VirtualBox、QEMU）。
- **是什么**：把通常由宿主内核实现的系统接口**搬进每个沙箱独立的用户态应用内核**，从而最小化容器逃逸风险；保留**进程式资源模型**、**低固定开销**、**快速启动**。
- **使用方法**：OCI runtime **`runsc`**，与 Docker、Kubernetes（GKE Sandbox）、containerd、CRI-O 集成。
- **官方对三条技术路线的划分**（本稿类型学的直接依据）：机器级虚拟化（KVM/Xen，经 VMM 暴露虚拟硬件给 guest 内核，pd 通常经过半虚拟化增强）／规则执行（seccomp/SELinux/AppArmor，依赖内核 hook）／gVisor 的第三条路（用户态应用内核）。
- **GKE Sandbox**：在 GKE 上用 gVisor 隔离 Pod（官方教程 "Docker in a GKE sandbox"）。**GA 状态与限制本次未逐条核对**。
- **已知取舍**：gVisor 需要自行实现大量 Linux 系统调用（兼容性是主要代价），且官方提供 GPU/TPU/RDMA 支持的专门文档（说明这些设备的支持是分阶段补齐的）。

**Confidential VM / Shielded VM**
- **Confidential VM**：提供内存加密的执行环境，支持 **AMD SEV / SEV-SNP** 与 **Intel TDX**（**具体机型与 GA 时间点本次仅取得 Google Cloud 机密计算 release notes 的 RSS 入口，未逐条展开，标注「未能完整证实」**）。
- **Confidential GKE Nodes**：GKE 上的机密节点能力（官方 API 参考 `ConfidentialNodes` 存在，见参考资料）。
- **Shielded VM**：vTPM + Secure Boot + 完整性度量，防 rootkit/bootkit；是 GCP 较早（2018 起）推出的启动链保护能力（**本次未取得当前官方页面，标注「未能证实细节」**）。
- **Google Cloud 上跑 VMware**：**Google Cloud VMware Engine** 是 Broadcom 认可的 MaaS 平台之一（由 Anglepoint 文章侧面证实），说明 Google 同时提供"托管 VMware"与"自研 KVM/gVisor"两条虚拟化路线。

---

## 云厂商架构演进

### 主线：全虚拟化 → 半虚拟化 virtio → 硬件辅助 + SR-IOV 直通 → DPU/IPU 全卸载 → microVM 轻量化 → 机密计算

| 阶段 | 技术特征 | 代表技术 | 时间锚点（可证实的） |
|---|---|---|---|
| **① 全虚拟化 / 纯软件模拟** | 二进制翻译 + 设备模拟，性能损耗大 | QEMU(TCG)、早期 Xen(HVM+QEMU device model)、ESX 早期 | Xen 2003；EC2 2006 起步（PV 为主） |
| **② 半虚拟化 + virtio** | 改 guest 前端驱动，共享内存环，去掉设备模拟 | Xen PV、**virtio**（1999 Virtio 前身、2008 起标准化）、VMware VMXNET3/vmxnet3、Hyper-V **Enlightened I/O** + **VMBus** | virtio 规范 2008+；Hyper-V 2008 起 |
| **③ 硬件辅助 + SR-IOV 直通** | Intel VT-x/AMD-V + EPT/NPT，IOMMU，VF 直分给 VM | KVM、ESXi、Hyper-V、**AWS Nitro Card 的 SR-IOV VF（NVMe + ENA）**、Xen PCI passthrough | Intel VT-x 2005/2006；AWS C5 + Nitro **2017-11** |
| **④ DPU/IPU 全卸载 + 极简 hypervisor** | 控制面与 I/O 全下沉专用卡；hypervisor 只做 CPU/内存分区 | **AWS Nitro**（Cards + Security Chip + Nitro Hypervisor）、**Azure Boost**（FPGA + MANA + Cerberus）、**华为 QingTian Cards + QingTian Hypervisor**、**阿里神龙 X-Dragon 虚拟化芯片 + Hypervisor** | **2017 年是分水岭**：AWS Nitro（C5，re:Invent 2017）、阿里神龙首款产品 2017-10、华为 QingTian 实例 2017；Azure Boost 公开于 Ignite 2023、文档 2025-03 更新 |
| **⑤ microVM 轻量化** | 每工作负载一个极小 VM；VMM 自研、设备数降到个位、毫秒级启动 | **AWS Firecracker**（2018 开源，Lambda/Fargate）、Cloud Hypervisor、crosvm、**腾讯 CubeSandbox/CubeHypervisor**、**阿里 ACK 安全容器（runV，沙箱启动约 150 ms）**、Kata Containers、rust-vmm 组件库 | Firecracker 2018；ACK 安全容器文档 2026-06 更新；CubeSandbox 2025 前后开源 |
| **⑥ 机密计算** | 内存加密 + 远程证明；机密 VM / 机密容器 / 机密 GPU | **Azure**（AMD SEV-SNP DCasv5/6、ECasv5/6；Intel TDX DCesv6/ECesv6；NCCadsH100v5）/**GCP Confidential VM**（SEV-SNP、TDX）/**AWS Nitro Enclaves**（**本次未核对官方文档，标注「未能证实」**） | AMD SEV 2016/SEV-SNP 第三代 EPYC；Intel TDX 第五代 Xeon；Azure 官方 CVM 文档列出 v5/v6 机型 |

### 分水岭：为什么是 2017 年

- 2017 年之前，云上虚拟化的优化主要在 guest 内部（virtio、驱动）与 hypervisor 内部（半虚拟化、CPU 特性透传）。
- 2017 年起，AWS（C5/Nitro）、阿里（神龙）、华为（QingTian）几乎同期把**控制面与 I/O 搬出主机 CPU**，让 hypervisor 缩到最小。三者独立选择了同一种结构，说明这是被"性能 + 安全 + 裸金属支持"三个需求共同逼出来的必然收敛。
- 2018 年 Firecracker 把这条线推到"每个函数一个 microVM"，同时把 rust-vmm 生态送给整个行业——今天几乎所有新 microVM（CubeSandbox、AgentENV 等）都站在这个生态上。
- 2020 年代后期的主线是**机密计算**：把 SEV-SNP/TDX 从"机密 VM"扩展到"机密容器 / 机密 GPU"。

### 各大云在这条主线上的位置（速查）

| 云 | 卸载层 | 极简 hypervisor | microVM / 安全容器 | 机密计算 |
|---|---|---|---|---|
| **AWS** | Nitro Cards + Nitro Security Chip（自研 SoC，Annapurna） | Nitro Hypervisor（无网络栈/无 FS/无驱动/无 shell，在线热更新） | **Firecracker**（Lambda/Fargate） | Nitro Enclaves（未证实）；SEV-SNP 机型存在（未核实） |
| **Azure** | Azure Boost（FPGA + MANA + Cerberus） | Microsoft Hypervisor（微内核 + root partition） | 机密容器 / AKS 沙箱（未证实细节） | **SEV-SNP + Intel TDX + NVIDIA H100 机密 GPU** |
| **阿里云** | 神龙 X-Dragon 虚拟化芯片（MOC 卡）+ 服务器硬件架构 | X-Dragon Hypervisor | **ACK 安全容器 V2（runV，~150 ms）** | 未证实 |
| **华为云** | **QingTian Cards**（专用 ASIC + 硬件信任根 + IO 加解密） | **QingTian Hypervisor**（轻量精简） | 未取得官方说明 | 未证实（白皮书强调信任根与度量） |
| **腾讯云** | 未证实 | KVM 系（自研组件未证实） | **CubeSandbox**（CubeHypervisor = Cloud Hypervisor fork） | 未证实 |
| **Google Cloud** | 未证实（有自研 NIC/基础设施） | KVM（自研组件未证实） | **gVisor/runsc**（用户态内核，非 VM）、GKE Sandbox | **Confidential VM（SEV-SNP / TDX）**、Confidential GKE Nodes |

---

## 对照表

> 说明：「嵌套」列中 ✅ 表示有官方/权威来源支持；⚠️ 表示支持但本次未逐条核对；❓ 表示未能证实。

| 产品 | 厂商 | 类型 | 底层技术 | 嵌套虚拟化 | 商业模式 / 现状 |
|---|---|---|---|---|---|
| **ESXi / vSphere** | Broadcom (VMware) | Type-1 裸金属 | 自研 VMkernel（非 Linux） | ⚠️ 支持（需勾选向 guest 暴露硬件虚拟化） | 仅按核订阅；打包为 VCF / VVF；永久授权封顶 5.x；VCF 用户每 180 天须提交合规报告（2025-05 SPD） |
| **vCenter / Aria 管理组件** | Broadcom (VMware) | 管理面 | N/A | N/A | 被限制为**仅可在 VCF 环境使用** |
| **Workstation / Fusion** | Broadcom (VMware) | Type-2 | 自研 VMM + VT-x/AMD-V | ⚠️ 有 "Virtualize Intel VT-x/EPT or AMD-V/RVI" 选项 | 桌面授权（按版本/订阅）；Fusion 面向 Apple Silicon |
| **vSAN / NSX** | Broadcom (VMware) | 存储 / 网络叠加 | 与 vSphere 内核集成 | N/A | 随 VCF/VVF 捆绑（vSAN 权益：VCF 1 TiB/core、VVF 0.25 TiB/core） |
| **Hyper-V** | Microsoft | 混合型（微内核 + root partition） | Microsoft Hypervisor + VMBus + Enlightened I/O + SLAT/IOMMU | ✅ 支持（`ExposeVirtualizationExtensions`，Win Server 2016+） | 随 Windows Server / Windows Pro 授权；无独立免费 SKU（Hyper-V Server 已退役，未逐条证实） |
| **Windows Hypervisor Platform** | Microsoft | API 层 | 暴露 Microsoft Hypervisor 给第三方 VMM | 取决于调用方 | 随 Windows 提供 |
| **WSL2** | Microsoft | 轻量 utility VM | Hyper-V 技术栈 + 轻量实用 VM | ⚠️ 可在 Azure 上用嵌套支持 | 随 Windows 免费 |
| **Windows Sandbox** | Microsoft | 一次性隔离环境 | Hyper-V 技术栈 | N/A | 随 Windows 10/11 Pro+ 提供 |
| **VBS / HVCI / Credential Guard** | Microsoft | 基于虚拟化的安全（VTL） | Microsoft Hypervisor | 与第三方 hypervisor/嵌套存在兼容性冲突 | 随 Windows / 企业策略 |
| **Azure VM** | Microsoft | 卸载型云实例 | Microsoft Hypervisor + Azure Boost | ⚠️ 部分系列支持（Dv3/Ev3 起，未完整核对） | 按 VM 计费 |
| **Azure 机密 VM** | Microsoft | 云实例 | AMD SEV-SNP（DCasv5/6、ECasv5/6）、Intel TDX（DCesv6/ECedsv6）、H100 机密 GPU（NCCadsH100v5） | ❓ | 按 VM 计费 |
| **Azure Boost** | Microsoft | 卸载平台 | 可编程 FPGA + MANA 网卡 + Cerberus 信任根；Rust 为主要新代码语言 | N/A | 随 Boost 兼容 VM 规格提供 |
| **VirtualBox** | Oracle | Type-2 | 自研 VMM + VT-x/AMD-V；Windows 上可选 Hyper-V 引擎；**Linux 6.16+ 走 KVM API 协调 VT-x** | ✅ 支持（7.2.0 修复 Intel 嵌套） | 基础包 **GPLv3 开源**；Extension Pack 专有（个人免费、商用需授权）；7.2.18 = 2026-09-15 |
| **Oracle Linux KVM / OLVM** | Oracle | KVM 型 + 管理面 | KVM + QEMU + libvirt（oVirt 血统） | ⚠️ | Oracle Linux 订阅；Oracle VM Server for x86 已退役（未证实） |
| **Xen Project** | 开源（Linux Foundation） | Type-1 微内核 + dom0 | Xen hypervisor，PV/HVM/PVH | ⚠️ | 开源；4.x 持续发布（4.20 存在已证实） |
| **Citrix Hypervisor / XenServer** | Cloud Software Group | Type-1 + 管理栈 | Xen + xapi | ⚠️ | 商业授权；团队规模小于 Vates（对照说法）；当前版本状态未证实 |
| **XCP-ng** | Vates | Type-1 + 管理栈 | Xen + xapi（8.x）；**9.0 为全新平台、不基于 XenServer** | ⚠️ | 开源免费；商业订阅 **Vates VMS**（按物理主机计价）；**8.3 是 8.x 末版**；旧 XO Starter/Enterprise 订阅 2026-07-31 退役 |
| **Xen Orchestra** | Vates | 管理 + 备份 | 管理 XCP-ng/XenServer | N/A | 开源版永久免费；商业版并入 Vates VMS |
| **QEMU** | 开源 | VMM / 模拟器（**不是 hypervisor**） | TCG（纯模拟）或 kvm/whpx/hvf 加速 | 取决于后端 hypervisor | GPLv2 开源 |
| **Proxmox VE** | Proxmox | KVM 型集成平台 | Debian 13 + KVM/QEMU 10.0.2 + LXC 6.0.4 + Ceph + ZFS | ⚠️ KVM nested | **AGPLv3 开源**；订阅 **EUR 115/年/CPU 起**；9.0 发布于 2025-08-05；9.1 已发布 |
| **OpenStack** | 开源（OpenInfra） | 编排/控制面（**不是 hypervisor**） | 默认 libvirt + KVM/QEMU | ⚠️ 取决于 virt driver | 开源；企业发行版（Red Hat、Canonical 等） |
| **oVirt / RHV** | Red Hat / 社区 | KVM 型管理平台 | KVM + QEMU + libvirt | ⚠️ | oVirt 开源；RHV 转向 OpenShift Virtualization（未证实 EOL 时间） |
| **AWS Nitro System** | AWS | 卸载型 | Nitro Cards（Annapurna SoC）+ Nitro Security Chip + Nitro Hypervisor（SR-IOV VF 直通） | ✅ 虚拟实例支持（仅 C7i/C8i/M7i/M8i/R7i/R8i/X8i/I7i 等；L1 仅 KVM/Hyper-V）；`.metal` 天然可跑任意 hypervisor | 按实例计费；嵌套不额外收费 |
| **Firecracker** | AWS（开源） | microVM monitor | Rust + KVM，极简设备模型 | ❓ | Apache 2.0 开源；Lambda / Fargate 的生产底座 |
| **阿里云 ECS / EBM（神龙）** | 阿里云 | 卸载型 | X-Dragon 虚拟化芯片（MOC 卡）+ X-Dragon Hypervisor + 服务器硬件架构；支持 x86/ARM/Power/国产 CPU | ❓ 未证实 | 按实例计费；弹性裸金属 2018 起商业化；后续代际未证实 |
| **ACK 安全容器（runV）** | 阿里云 | 安全容器 / 轻量 VM | 轻量 VM + 独立 guest 内核；virtio-fs；Terway 网络 | N/A | 随 ACK 提供；**仅 ECS 裸金属规格**；沙箱启动 ~150 ms；V2 开销降 90%、启动快 3×、密度 10× |
| **华为云 QingTian（擎天）** | 华为云 | 卸载型 | QingTian Cards（专用 ASIC、独立供电、硬件信任根、IO 加解密）+ 精简 QingTian Hypervisor | ❓ 未证实 | 按实例计费；2017 年首发基于 QingTian 的实例；一套架构支持虚机/裸机/容器 |
| **StratoVirt / iSula** | openEuler 社区 | Rust VMM / 容器引擎 | rust-vmm + KVM | ❓ | 开源；与华为云商用产品关系未证实 |
| **腾讯云 CubeSandbox** | 腾讯云 | microVM 沙箱（Agent） | rust-vmm + KVM；**CubeHypervisor = Cloud Hypervisor fork**；XFS FICLONE CoW；containerd Shim v2 | ❓ | **Apache 2.0 开源**；冷启动 <60 ms、<5 MB/实例；兼容 E2B SDK |
| **腾讯云 星星海 / TKE 安全容器** | 腾讯云 | 自研服务器 / 安全容器 | 未证实 | ❓ | 未证实 |
| **Google Compute Engine** | Google Cloud | KVM 型云实例 | KVM（自研组件未证实） | ⚠️ 部分机型 | 按实例计费 |
| **gVisor / runsc** | Google（开源） | **用户态应用内核**（不是 VM） | Go 实现的 Linux 兼容应用内核；OCI runtime `runsc` | N/A | Apache 2.0 开源；GKE Sandbox |
| **Google Confidential VM** | Google Cloud | 云实例 | AMD SEV-SNP / Intel TDX | ❓ | 按实例计费；机型与 GA 时间未完整证实 |
| **Google Shielded VM** | Google Cloud | 云实例 | vTPM + Secure Boot + 完整性度量 | N/A | 随实例提供（细节未证实） |

---

## 未决问题

1. **Nitro Hypervisor 是否/在多大程度上基于 KVM**——AWS 官方白皮书当前不写 KVM。需要找 AWS re:Invent 演讲原文或 AWS 工程博客来确证，否则只能写成"广泛报道"。
2. **AWS Nitro Enclaves 的当前官方文档与能力边界**（本稿未核对）。
3. **EC2 从 Xen 全面迁到 Nitro 的完成时间点**（各代实例的迁移时间表）。
4. **Azure 嵌套虚拟化的完整支持矩阵**（哪些系列/代际/区域；与 Boost 机型的交互）。
5. **Azure 自研 DPU 的官方产品页与 GA 状态**（2024 Ignite 报道之后的进展）。
6. **阿里云神龙的代际演进与 MOC 卡最新规格**（第八/九代、CIPU 的官方表述与时间线）；**函数计算 FC 的实际隔离机制**（microVM？安全容器？）。
7. **腾讯云的自研 hypervisor/卸载卡情况**（是否有类 Nitro 的卸载层）；**TKE 安全容器的具体实现**；**CubeSandbox 的正式开源日期与嵌套虚拟化要求**。
8. **华为云 StratoVirt 与商用 ECS 的关系**；QingTian Cards 的代际与具体 ASIC 规格；鲲鹏 + QingTian 的组合细节。
9. **Oracle VM Server for x86 的官方退役公告**；**Hyper-V Server 独立 SKU 退役的官方页面**；**RHV 的 EOL 官方公告**。
10. **VirtualBox "KVM 后端"的权威一手确认**——heise 那篇 "VirtualBox erhält experimentellen KVM-Support" 正文未能抓取（页面抓取失败），需要另一来源核对措辞，避免把"通过 KVM API 申请 VT-x"误传成"改用 KVM 后端"。
11. **XenServer 当前版本与授权现状**；**Xen Project 4.20 的确切发布日期**。
12. **GCP 在 KVM 之上的自研组件清单**（是否有官方白皮书）；Confidential VM 的机型与 GA 时间线。
13. **VMware 2025-05 SPD 之后是否还有更新的授权规则**（本稿以 Anglepoint 2025-06-13 与 Redress 2026 文章为据，**未取得 Broadcom 官方 SPD 原文**——这是本稿在 VMware 授权部分的**主要证据缺口**）。
14. **各家"版权/授权"口径的原文出处**：本稿的商业模式结论部分来自第三方许可顾问与媒体报道，建议后续补充 Broadcom 官方许可指南页面。

---

## 参考资料

> 访问日期均为 **2026-09-16**（除个别页面自身标注的更新时间外）。

### 类型学 / KVM / QEMU / gVisor
- KVM 官方 FAQ（KVM 与 Xen/QEMU/VMware 的差异、QEMU 角色）：https://www.linux-kvm.org/page/FAQ
- gVisor 官方文档 "What is gVisor?"（用户态应用内核定位、对三条技术路线的划分）：https://gvisor.dev/docs/
- gVisor GPU 支持：https://gvisor.dev/docs/user_guide/gpu/
- Linux 内核 KVM `enable_virt_at_load` 破坏其他虚拟化方案的记录（LWN/KVM 邮件列表 bug 219602）：https://marc.info/?l=kvm&m=173434032815547&w=3

### VMware / Broadcom
- Anglepoint, "Broadcom's VMware Licensing Has Changed—Here's What You Need to Know in 2025"（2025-06-13；按核订阅、VCF/VVF、180 天合规报告、永久授权封顶 5.x、托管与可移植性限制）：https://www.anglepoint.com/blog/articles/broadcoms-vmware-changes-2025/
- Redress Compliance, "VMware Licensing 2026: VVF vs VCF and the Core Rule"（2026；VCF/VVF 对比与核心计数规则）：https://redresscompliance.com/vmware-licensing-comparison-2026
- Redress Compliance, "VMware Cloud Foundation Licensing: 2026 Guide"：https://redresscompliance.com/vcf-licensing-guide-2026
- Redress Compliance, "VMware New Licensing Model 2026"：https://redresscompliance.com/vmware-new-licensing-model-2026
- CIO Dive, "VMware's first contentious year under Broadcom drives customers to weigh other options"：https://www.ciodive.com/news/broadcom-vmware-acquistion-vcf-private-cloud/733800/
- SoftwareOne, "Broadcom's acquisition of VMware: What you need to know"（2024-03-25）：https://www.softwareone.com/en-us/blog/articles/2024/03/25/broadcoms-acquisition-of-vmware-what-you-need-to-know

### Microsoft / Hyper-V / Azure
- Microsoft Learn, "Hyper-V Architecture"（`ms.date: 01/21/2026`；root/child partition、VMBus、VSP/VSC、Enlightened I/O、IOMMU、SLAT）：https://learn.microsoft.com/en-us/windows-server/virtualization/hyper-v/architecture
- Microsoft Learn, "Overview of Azure Boost"（`ms.date: 03/18/2025`；FPGA 卸载、MANA、Cerberus、SELinux、FIPS、Rust、200 Gbps、36 GB/s / 6.6M IOPS）：https://learn.microsoft.com/en-us/azure/azure-boost/overview
- Microsoft Learn, "Azure Confidential VM options"（SEV-SNP / Intel TDX 机型表、NCCadsH100v5）：https://learn.microsoft.com/en-us/azure/confidential-computing/virtual-machine-options
- Microsoft Learn, "Comparing WSL Versions"（WSL2 的轻量 utility VM 说明）：https://learn.microsoft.com/en-us/windows/wsl/compare-versions
- SDxCentral, "Microsoft announces Azure Boost to offload virtualization processes"：https://www.sdxcentral.com/news/microsoft-announces-azure-boost-to-offload-virtualization-processes/
- SDxCentral, "Microsoft launches DPU and new HSM chips, also launches hybrid infrastructure platform Azure Local"（2024-11）：https://www.sdxcentral.com/news/microsoft-launches-dpu-and-new-hsm-chips-also-launches-hybrid-infrastructure-platform-azure-local/
- StorageReview, "Microsoft Ignite 2024: Advancements in Custom Silicon and AI Infrastructure"：https://www.storagereview.com/news/microsoft-ignite-2024-advancements-in-custom-silicon-and-ai-infrastructure
- Microsoft Learn, "NVv4-series"（GPU 分区机型）：https://learn.microsoft.com/en-us/azure/virtual-machines/sizes/gpu-accelerated/nvv4-series

### Oracle / VirtualBox
- VirtualBox 官方 News（7.2.0 = 2025-08-14、7.2.2 = 2025-09-10、7.2.18 = 2026-09-15 等版本日期）：https://www.virtualbox.org/wiki/News
- VirtualBox 官方 Changelog 7.2（"Linux host: Use KVM APIs on kernel 6.16.0 and newer for acquiring/releasing VT-x"、NVMe 与 USB Webcam 开源化、Arm 支持、7.2.0 修复 Intel 嵌套虚拟化）：https://www.virtualbox.org/wiki/Changelog-7.2
- VirtualBox 官方 Changelog 7.1（内核 6.16 KVM `enable_virt_at_load` 相关注意事项）：https://www.virtualbox.org/wiki/Changelog-7.1
- heise, "VirtualBox erhält experimentellen KVM-Support"（标题可见，正文抓取失败，**未能证实**）：https://www.heise.de/news/VirtualBox-erhaelt-experimentellen-KVM-Support-11168418.html
- 9to5Linux, "VirtualBox 7.2.2 Adds Support for KVM APIs on Linux Kernel 6.16 and Newer"（对 7.2.2 的报道）：https://9to5linux.com/virtualbox-7-2-2-adds-support-for-kvm-apis-on-linux-kernel-6-16-and-newer
- It's FOSS Community 讨论帖（"VirtualBox can't operate in VMX root mode. Please disable the KVM kernel extension"，Linux 6.16 与 VirtualBox 冲突的第一手案例）：https://itsfoss.community/t/virtualbox-7-1-14-virtualbox-cant-operate-in-vmx-root-mode-please-disable-the-kvm-kernel-extension/15175
- Oracle OCI 文档中 "Oracle Linux KVM Image (EOL)"：https://docs.oracle.com/en-us/iaas/oracle-linux/oci/index.htm

### Xen 系
- XCP-ng 论坛 "Goodbye XCP-ng"（Vates CEO 公开回复：8.3 为 8.x 末版、**9.0 完全不基于 Citrix XenServer**、团队规模、按物理主机计价、qcow2 与 vTPM）：https://xcp-ng.org/forum/topic/11403/goodbye-xcp-ng/20
- Vates 官方博客, "Transitioning to the next chapter of Xen Orchestra"（2025-10-28；旧 XO Starter/Enterprise 订阅 2026-07-31 退役、Vates VMS 捆绑、开源版不受影响）：https://vates.tech/blog/transitioning-to-the-next-chapter-of-xen-orchestra/
- Vates 官方博客, "Why infrastructure control matters more than ever in 2026"（2026-08-18；团队近 150 人）：https://vates.tech/blog/why-infrastructure-control-matters-more-than-ever-in-2026/
- Xen Project Wiki, "Xen Project 4.20 Feature List"：https://wiki.xenproject.org/index.php?title=Xen_Project_4.20_Feature_List

### 集成 / 管理平台
- Proxmox 官方新闻稿, "Proxmox Virtual Environment 9.0 with Debian 13 released"（2025-08-05；内核 6.14.8-2、QEMU 10.0.2、LXC 6.0.4、Ceph 19.2.3、ZFS 2.3.3、LVM 共享存储快照、SDN Fabrics、HA 亲和、Rust/Yew UI、AGPLv3、EUR 115/年/CPU 起、1.6M 主机）：https://proxmox.com/en/about/company-details/press-releases/proxmox-virtual-environment-9-0
- Proxmox VE 9.1 Release 讨论（社区，2025/2026）：https://github.com/community-scripts/ProxmoxVE/discussions/9278
- Proxmox VE Roadmap：https://pve.proxmox.com/wiki/Roadmap

### AWS
- AWS 官方白皮书, "Security design of the AWS Nitro System — The components of the Nitro System"（Nitro Cards / Nitro Controller / Nitro Security Chip / Nitro Hypervisor 的完整设计、SR-IOV VF、AES-256-GCM 传输加密、形式化验证、在线热更新、"hypervisor 成为可选组件"）：https://docs.aws.amazon.com/whitepapers/latest/security-design-of-aws-nitro-system/the-components-of-the-nitro-system.html
- AWS 官方用户指南, "Use nested virtualization to run hypervisors in Amazon EC2 instances"（受支持实例类型清单、L1 仅 KVM/Hyper-V、L0/L1/L2 三层、Windows 限制、不额外收费）：https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html
- AWS 官方白皮书, "Serverless and Containers"（Lambda/Fargate 隔离背景）：https://docs.aws.amazon.com/whitepapers/latest/logical-separation/serverless-and-containers.html
- AWS 官方博客, "Bare metal performance with the AWS Nitro System"：https://aws.amazon.com/blogs/hpc/bare-metal-performance-with-the-aws-nitro-system/
- GeekWire, "With Firecracker, Amazon Web Services reinvents its serverless computing infrastructure"（2018）：https://www.geekwire.com/2018/firecracker-amazon-web-services-reinvents-serverless-computing-infrastructure-open-source-reputation/

### 阿里云
- 阿里云开发者社区, "云计算史上的第一次开箱直播 阿里云神龙技术架构首次全方位曝光"（2018-05-17；X-Dragon 虚拟化芯片 + Hypervisor 系统软件 + 服务器硬件架构、MOC 卡、Superπ 对比口径、多架构支持、弹性裸金属商用）：https://developer.aliyun.com/article/593606
- 阿里云官方帮助文档（英文）, "Sandboxed containers overview"（Last Updated: **Jun 15, 2026**；轻量 VM + 独立 guest 内核、V1→V2 三项提升、与 Kata Containers 的对比表、virtiofs 卷、**仅 ECS 裸金属规格**、集群版本 1.16–1.34、RuntimeClass runC/runV）：https://www.alibabacloud.com/help/en/ack/ack-managed-and-ack-dedicated/user-guide/overview-10/
- 阿里云官方帮助文档（中文）, "安全沙箱概述"：https://www.alibabacloud.com/help/zh/ack/ack-managed-and-ack-dedicated/user-guide/overview-10/

### 华为云
- 华为云《QingTian 系统安全技术白皮书》"QingTian 系统简介"（文档页更新时间 **2025-10-16/17 GMT+08:00**；2017 年发布基于 QingTian 的实例、QingTian Cards + QingTian Hypervisor、零资源预留/零算力损失/零业务抖动、硬件信任根与 IO 加解密、一套架构支持虚机/裸机/容器）：https://support.huaweicloud.com/intl/zh-cn/twp-ecs/ecs_twp_0002.html （英文页：https://support.huaweicloud.com/intl/en-us/twp-ecs/ecs_twp_0002.html ）
- 华为云新闻, "华为云擎天架构，引领云基础设施升级"（2020-05）：https://www.huaweicloud.com/news/2020/20200520092904332.html
- 华为云封面故事, "盘点华为云擎天架构年度关键词"：https://www.huaweicloud.com/cloudplus/ninthphase/detail06.html
- openEuler Virt Meetup 北京站活动回顾（2023-10-20；openEuler 虚拟化生态，StratoVirt/iSula 背景）：https://www.openeuler.org/zh/news/openEuler/20231020-vrt/20231020-vrt

### 腾讯云
- 腾讯云国际站新闻, "Tencent Cloud Cube Sandbox Goes Fully Open-Source, with Five Major Breakthroughs Enabling Large-Scale Agent Deployment"（标题与站点已取得，正文抓取未成功，**发布日期未能证实**）：https://intl.cloud.tencent.com/dynamic/news-details/101123
- 腾讯云开发者社区, "Rust × 腾讯云：当系统级语言重塑云计算基因"（CubeSandbox 的 Rust 技术栈背景）：https://cloud.tencent.cn/developer/article/2675765
- 腾讯云 CubeSandbox 开源仓库：https://github.com/TencentCloud/CubeSandbox
- 仓库内既有笔记（本地）：`docs/rust-kunpeng/cubesandbox-rust-analysis.md`、`docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`、`docs/rust-kunpeng/rust-application-patterns.md`

### Google Cloud
- gVisor 官方文档（见上）：https://gvisor.dev/docs/
- Google Cloud 机密计算 release notes（Confidential VM，RSS）：https://docs.cloud.google.com/feeds/confidential-computing-release-notes.xml
- GKE API 参考 "ConfidentialNodes"：https://docs.cloud.google.com/kubernetes-engine/docs/reference/rest/v1/ConfidentialNodes
- Google Cloud 社区博客, "Beyond Confidential: Establishing Trust in Your Computing Environment"：https://security.googlecloudcommunity.com/community-blog-42/beyond-confidential-establishing-trust-in-your-computing-environment-6290

---

## 附：本稿的取证纪律说明

- 本文所有"已证实"结论均可回溯到上列 URL；标注「未能证实」的条目表示本次联网检索未取得可靠一手来源，**未据推测补全**。
- 厂商自测数据（阿里云 Superπ 对比、阿里云安全容器性能对比）已注明为**厂商口径**，不等同于第三方基准。
- 版本日期全部来自官方 changelog / 新闻稿；社区帖子仅用于佐证"现象存在"（如 Linux 6.16 与 VirtualBox 的冲突），不用于断言设计意图。
- 本稿为中间稿，`## 未决问题` 中列出的 14 项缺口建议在正式稿中补齐，尤其是 **Broadcom 官方 SPD 原文**与 **AWS 关于 Nitro Hypervisor 血统的一手说明**。
