# 虚拟化：发展过程、硬件虚拟化的定义与软件虚拟化的性能损失

> 中间调研稿（research/，尚未进入 docs/）· 更新：2026-09-16
>
> 范围：① 虚拟化演进时间线；② 模拟 / 全虚拟化 / 半虚拟化 / 硬件辅助 / OS 级虚拟化的概念辨析；
> ③ 硬件虚拟化的**可判据**定义；④ **软件（无硬件辅助）虚拟化的性能损失量化**——本稿最重要的部分。
>
> 引用原则：所有性能数字都标注**测试条件**与**出处**；找不到一手来源的结论显式标注「未能证实」或「存在分歧」。
> 本稿中的 Xen 数字来自 SOSP 2003 原文 PDF（用纯 Python 解压 PDF 流提取文本，表格数字已与 HTML 镜像交叉核对）。

---

## 核心结论

1. **虚拟化不是新概念，而是 1960 年代大型机的产物**：IBM CP-40/CP-67/CP/CMS（1967 起）与 VM/370（1972）已经实现了"在裸机上运行多个独立操作系统"，比 x86 上可用早了 30 多年。
2. **Popek & Goldberg（CACM 1974）给出了可判据的理论**：等价性、资源控制、效率三条件；核心定理是"**每条敏感指令都必须是特权指令**"的架构才能经典 trap-and-emulate 虚拟化。
3. **x86 长期不可虚拟化**，原因是那 17 条"用户态可见/可改特权状态、但不陷入"的指令（`popf`/`iret`/`sgdt`/`sldt`/`sidt`/`smsw`/`lar`/`lsl`/`pop [seg]`/`far call,jmp,ret`/`int N` 等）。因此 1999–2005 年只能用**二进制翻译（BT）**或**半虚拟化**绕过，直到 2005 年 Intel VT-x、2006 年 AMD SVM 才补上硬件。
4. **硬件虚拟化的可判据定义**：CPU 新增一层比 guest 内核更高的特权级/模式（Intel root/non-root + VMCS，AMD host/guest + VMCB，ARM EL2）；"陷入—模拟"中的**陷入、状态保存/恢复、地址翻译由硬件做**，而不是靠改写 guest 代码。**没有"必须用硬件才能虚拟化"这回事**，也不等于"VM 一定比容器慢"。
5. **软件虚拟化的开销高度依赖负载类型，单一百分比没有意义**。同一篇 Xen 论文里，CPU 密集的 SPEC INT2000 上 XenoLinux **与原生打平**（567 vs 567），而 VMware Workstation 3.2 的 TCP 发送吞吐只有原生的 **32%**（291 vs 897 Mb/s，MTU 1500，ttcp）。
6. **CPU 密集负载：软件虚拟化几乎无损失**。XenoLinux 的 SPEC INT2000 分数 = 原生；Linux 内核编译（`make` 2.4.21）耗时 271s vs 原生 263s，论文自述"**mere 3% overhead**"。
7. **特权/系统调用/进程创建密集负载：软件虚拟化损失可达数倍**。Adams & Agesen（ASPLOS 2006）的 `forkwait`（fork+waitpid 共 4 万次）在 Pentium 4 上：原生 6.0s、BT 软件 VMM **36.9s（6.1×）**、无 EPT 的 VT-x 硬件 VMM **106.4s（17.7×）**。
8. **MMU 是软件虚拟化的最大痛点**。影子页表（shadow page table）下每次 `pgfault` 要陷入 VMM：原生 1093 cycles、BT 软件 VMM 3927 cycles（3.6×）；而**没有 EPT 的早期硬件 VMM 反而更差，11242 cycles**——这是"硬件辅助初期比软件还慢"的根因。bhyve 论文转引 AMD 白皮书称，**某些负载下影子页表可占整个 hypervisor 开销的 75%**。
9. **I/O 密集负载：设备"全模拟"是主要瓶颈**。同类对比（Xen SOSP 2003 的 ttcp）中，半虚拟化网卡（Xen）在 MTU 1500 下 TCP 收发吞吐 **与原生相同（897 Mb/s，0%）**，而 VMware Workstation 3.2 的模拟 pcnet32 发送只有 **291 Mb/s（−68%）**；MTU 500 时模拟网卡恶化到 **−83%～−90%**。半虚拟化（virtio 思想的前身）几乎抹平了这部分差距。
10. **硬件辅助并没有立刻取胜**：ASPLOS 2006 用同一套负载对比得出结论——**没有 MMU 虚拟化的 VT-x/SVM 在多数负载上不如成熟的 BT**；Linux 内核编译原生 265s、软件 VMM 393s（67.4%）、硬件 VMM 484s（54.8%）。真正扭转局面的是 2007–2008 年的 **AMD NPT / Intel EPT**（二级地址转换）。
11. **内存"虚拟化开销"要区分两件事**：① MMU 翻译开销（如上，影子页表 vs EPT/NPT）；② 内存**容量**管理。Waldspurger（OSDI 2002）证明 ESX 的内容哈希页共享在生产环境可回收 **7%～33%** 的 VM 内存（最佳同类负载实验接近 **60%**），且 CPU 开销可忽略（吞吐 −1.6%～+1.8%）。这是"省内存"而不是"变慢"。
12. **"软件虚拟化性能损失大概多少"的诚实回答**：CPU 密集 **≈0–5%**；半虚拟化 I/O **≈0–15%**；全模拟 I/O **30%～90%**；MMU/进程创建密集 **3–18×**；系统调用级别 **每条多约 2000 cycles**（占单次 syscall 成本的比例很大，但端到端是否显著取决于 syscall 频率）。所以正确的问题不是"损失多少"，而是"**哪一类负载、用 BT 还是半虚拟化、有没有 EPT/NPT、设备是模拟还是 virtio**"。

---

## 1. 演进时间线

### 1.1 总表

| 年代 | 里程碑 | 代表系统 / 论文 | 关键技术 | 解决了什么 | 遗留问题 |
| --- | --- | --- | --- | --- | --- |
| 1960s 中后期 | 第一代虚拟机 | IBM CP-40（1967）、CP-67、CP/CMS（S/360-67） | VMM 直接跑在裸机；为每个用户提供一台"独立机器" | 分时共享 + 兼容旧 OS；CP-40 可同时跑 14 台 S/360 VM | 只存在于大型机；无形式化理论 |
| 1972 | 商用化 | IBM VM/370 | 虚拟化成为正式产品；CMS 交互 + 其他 OS 共用 | 大规模商用分时/迁移 | 依赖 S/360 恰好满足虚拟化条件，经验无法直接搬到微处理器 |
| 1974 | 理论奠基 | Popek & Goldberg, *CACM* 17(7) | 等价性 / 资源控制 / 效率三条件；"敏感指令必须特权"定理 | 明确"什么架构可被经典虚拟化" | 被微处理器工业界忽视近 30 年 |
| 1978–1980s | 微处理器踩坑 | Motorola 68000 → 68010 | 68000 读状态寄存器不陷入，无法做虚拟内存系统；68010 修复 | 说明"事后小修"可行但代价高 | x86 等主流架构仍然不可虚拟化 |
| 1997 | 学术复兴 | Disco（Bugnion, Devine, Rosenblum, SOSP'97 / TOCS 1997） | 在 cc-NUMA 上用 VMM；直接执行 + 陷入模拟（MIPS 有软件管理 TLB） | 让商品 OS 跑在可扩展多处理器上 | 依赖 MIPS 特性；x86 上仍难；需改 guest OS 少量代码 |
| 1999–2001 | PC 上可行 | VMware Workstation（1999）、VMware ESX Server（2001） | **直接执行 + 二进制翻译（BT）**；影子页表 | 在未修改的商品 OS 上跑 x86 虚拟机 | BT 复杂；影子页表 trap 开销大；性能随负载剧烈波动 |
| 2003 | 半虚拟化标杆 | Xen（Barham et al., SOSP 2003） | Hypercall + 事件通道 + I/O ring；guest 页表被 Xen 只读注册，避免影子页表 | CPU 密集负载接近原生（论文自述"at most a few percent"） | 需改 guest OS；未改 Windows 支持不完整；无硬件 MMU 虚拟化 |
| 2005 | x86 硬件辅助落地 | Intel VT-x（Pentium 4 6x2 系列，如 662/672） | root/non-root 模式 + VMCS + VM entry/exit | 经典 trap-and-emulate 在硬件上可行，不再必须 BT | 初期**没有 MMU 虚拟化**，某些负载比 BT 更慢（见 ASPLOS'06） |
| 2002–2006 | 与 VT-x 不同的另一条线 | Intel **VT-i**（Itanium / IA-64） | 面向 IA-64 的虚拟化扩展；首批量产在 Montecito（2006） | Itanium 平台的虚拟化 | **与 VT-x 是不同架构、不同指令**，不能混为一谈；Itanium 生态已退场 |
| 2006 | AMD 方案出货 | AMD SVM（"Pacifica"，Athlon 64 世代） | host/guest 模式 + VMCB + `vmrun`/exit | 与 VT-x 等价的 x86 硬件虚拟化 | 同 VT-x，初期缺二级地址转换 |
| 2006–2007 | 开源 hypervisor 进主线 | KVM（Avi Kivity / Qumranet），Linux 2.6.20（2007-02） | 把 Linux 内核本身变成 hypervisor，复用 VT-x/SVM | 随内核分发、零额外内核 | 强依赖硬件辅助（无 VT/SVM 时不可用） |
| 2007–2008 | 半虚拟化 I/O 标准化 | virtio（Rusty Russell, *SIGOPS OSR* 2008；Linux 2.6.24/2.6.25） | 统一的半虚拟化设备接口（descriptor ring + 驱动） | 用一个标准替换各家自定义半虚拟化设备，显著优于全模拟设备 | 仍需 guest 驱动；无驱动的老 OS 退回模拟 |
| 2007+ | I/O 直通 | SR-IOV（PCI-SIG 规范 2007；Intel 82576 等 2009 起） | 一个物理功能（PF）导出多个虚拟功能（VF），VF 直通 guest | I/O 数据面绕过 hypervisor，接近线速、低延迟 | 需硬件 + PF/VF 驱动；可迁移性/隔离性受限 |
| 2007–2008 | 硬件二级地址转换 | AMD **NPT**（Barcelona/K10，2007）、Intel **EPT**（Nehalem，2008） | guest 页表 + 宿主页表由硬件两级遍历 | 消除软件影子页表的 trap/写保护开销 | 二维页表遍历本身有成本；大页（2MB/1GB）可缓解 |
| 2014 规范 / 约 2016 落地 | ARM 侧减少陷阱 | **ARM VHE**（Virtualization Host Extensions，随 ARMv8.1-A 引入） | 让 host 内核直接以 EL2 运行，去掉 shadow EL1 的陷阱与上下文切换 | 大幅降低 ARM KVM 的陷入开销 | 需新硬件；旧核不可用（确切落地年份见 §1.3 注） |
| 2018 | ARM 嵌套虚拟化 | **ARMv8.4-A** 的 nested virtualization | EL2 内再跑一个 EL2（NVHE），硬件支持"VM 里的 hypervisor" | 云/边缘里在 VM 内跑 KVM/Xen | 复杂度高；软件生态跟进慢 |
| 2016 | 机密计算起步 | AMD **SEV**（Zen/EPYC 世代） | 每 VM 独立内存加密密钥，hypervisor 不可读 guest 内存 | 多租户下对 hypervisor 也不信任 | 初期无完整性保护（可被重放/篡改） |
| 2020 | 机密计算增强 | AMD **SEV-SNP** | 反向映射表（RMP）+ 完整性/重放保护 | 机密性 + 完整性 | 加密与标记带来性能开销；生态适配 |
| 2023 | Intel 路线落地 | Intel **TDX**（Sapphire Rapids） | Trust Domain 硬件隔离 + 内存加密/完整性 | Intel 平台的机密 VM | 与 SEV-SNP 的证明/生态路线存在分歧；性能开销需实测 |
| 2018 | microVM | AWS **Firecracker**（NSDI 2020 论文） | 基于 KVM 的极简 VMM（Rust），最小设备模型 | 高密度、低启动延迟的 serverless 多租户 | 设备模型被刻意简化，通用性/兼容性受限（具体延迟与内存数字见 §1.3） |

### 1.2 x86 为什么难：早期两条失败路线

- **Ring compression（环压缩）**：x86 只有 4 个环（0–3），guest 内核本该在 ring 0、hypervisor 却必须占据 ring 0，于是把 guest 内核"压"到 ring 1/3。问题：ring 1 与 ring 0 的语义并不等价——很多指令/段保护检查在 ring 1 下仍以 ring 0 的方式生效或直接失败。
- **Ring aliasing（环别名）**：guest 在非 ring 0 执行时，`pushf`/`popf`、`iret`、段寄存器访问、CPL 相关指令会**观察到"我不是 ring 0"**，等价性（equivalence）被破坏；而它们又**不陷入**，hypervisor 无法介入。
- 结论：x86 不满足 Popek-Goldberg 定理（至少 17 条指令用户态敏感却不特权），也**不是** hybrid-virtualizable。VMware 的应对是"放宽等价性"——直接不支持 guest 使用 CPL 1/2，并忽略少数"可用但无用"的指令（`sgdt`/`sldt`/`sidt`/`smsw`）。

> 出处：Harvard CS161 Lecture 25（基于 Bugnion/Nieh/Tsafrir《Hardware and Software Support for Virtualization》）列出的 17 条指令与 VMware 的取舍；ccl.cse.nd.edu 的博客给出 68000→68010 与"29 年后才有硬件扩展"的叙述。

### 1.3 需要区分的几组时间点

- **VT-x ≠ VT-i**：VT-x 是 x86（IA-32/Intel 64）的虚拟化扩展，2005 年随 Pentium 4 6x2 系列出货；VT-i 是 Itanium（IA-64）的虚拟化扩展，公布更早、首批量产在 Montecito（2006）。两者指令集、数据结构和适用平台完全不同。
- **NPT 早于 EPT**：AMD Barcelona（K10，2007）先有 NPT；Intel Nehalem（2008）才有 EPT。任务描述"2008 前后"作为整体时间窗是准确的，但"谁先"要分开说。
- **ARM VHE 的年份**：VHE 是 **ARMv8.1-A** 引入的特性，规范发布于 2014 年；"2016"通常指硬件/内核生态普遍可用（Linux 的 VHE 支持也在此前后成型）。本稿把两者都写出，**单一确切年份存在分歧**。
- **Firecracker 的具体指标未能证实**：常见引用为"启动约 125 ms、单实例内存开销 <5 MiB"，但本轮**未取得 NSDI 2020 原文核对**，故只记录"论文声称极低启动延迟与内存开销"，不给具体数字。
- **SEV/SEV-SNP/TDX 的"年份"指公布或首批硬件**：SEV 2016（Zen 公布）、SEV-SNP 2020（公布，Milan 世代量产于 2021）、Intel TDX 2023（Sapphire Rapids）。不同二手来源对"哪一年"口径不一，此处标注为**口径差异**。

---

## 2. 概念辨析：五种"虚拟化"到底差在哪

### 2.1 机制对照表

| 机制 | 指令**谁在执行** | 特权/敏感操作**怎么处理** | 代表实现 | 是否需要改 guest | 典型开销量级 |
| --- | --- | --- | --- | --- | --- |
| **模拟 / Emulation** | 几乎不执行 guest 指令，而是**逐条解释** | 全部由模拟器用软件实现 | QEMU TCG（动态翻译成宿主指令但语义完全软件模拟）、Bochs、旧式模拟器 | 不需要 | 最慢；可跨指令集（如 x86 on ARM） |
| **全虚拟化 / Full virtualization（二进制翻译 BT）** | 大部分指令**直接执行**；内核态"问题指令"被**动态改写**成安全代码或陷阱 | 翻译时插入对 VMM 的调用；影子页表维护 MMU | VMware Workstation/ESX（早期）、Virtual PC | 不需要（未修改 OS 即可跑） | CPU 密集接近原生；MMU/进程创建/全模拟设备开销大 |
| **半虚拟化 / Paravirtualization** | guest 指令直接执行；**敏感操作改为显式 hypercall** | 改 guest 内核源码，把特权操作换成 hypercall；设备用 ring 通信 | Xen（SOSP 2003）、virtio 驱动 | **需要改 guest OS**（ABI 不变） | CPU 密集 ≈ 原生（Xen 自述几个百分点）；I/O 接近原生 |
| **硬件辅助 / HVM** | guest 指令直接执行；陷入由硬件完成 | CPU 新增 root/non-root（Intel）/ host/guest（AMD）/ EL2（ARM）；VMCS/VMCB 保存状态 | VT-x、SVM、ARM 虚拟化扩展 | 不需要 | 依赖实现世代：**无 EPT/NPT 时可能不如 BT**；有 EPT/NPT 后接近原生 |
| **OS 级虚拟化 / 容器** | 与宿主**同一内核**、同一指令流 | 没有 hypervisor；用 namespace + cgroup 做隔离 | Linux namespaces/cgroups、Docker、LXC | 不适用（共用内核） | 几乎零虚拟化开销；隔离边界是内核而非硬件 |

### 2.2 一句话记忆

- **模拟**：指令由软件"演"出来 → 最慢、最通用。
- **全虚拟化（BT）**：指令直接跑，问题指令被"改台词" → 兼容但复杂。
- **半虚拟化**：直接跑，敏感操作"主动报告" → 快但要改 OS。
- **硬件辅助**：直接跑，陷阱"硬件帮你抓" → 不用改 OS，但效果随代际变化。
- **容器**：根本没有第二套内核 → 最快，隔离最弱。

---

## 3. 什么是硬件虚拟化（可判据的定义）

### 3.1 判据：满足以下两条才算

1. **CPU 新增了一层虚拟化模式/特权级，位于 guest 内核之下**：
   - Intel VT-x：**VMX root / non-root** 两种操作模式；VMM 在 root，guest 在 non-root；状态保存在 **VMCS**（Virtual Machine Control Structure）；`VM entry`/`VM exit` 切换。
   - AMD SVM：**host / guest** 模式；状态保存在 **VMCB**（Virtual Machine Control Block）；`vmrun` + exit 切换。
   - ARM：**EL2**（Hypervisor 异常级）；VHE 之后 host 内核可直接跑在 EL2。
2. **"陷入—模拟"由硬件加速**：guest 执行到敏感操作时，**硬件**完成陷入、保存/恢复状态、并在支持二级地址转换时**由硬件完成两级地址翻译**；VMM 只负责语义模拟。

**推论（重要）**：
- 硬件虚拟化**不是"虚拟化必须有硬件"**——Xen 半虚拟化、VMware BT 都在没有硬件扩展的年代跑得很好。
- 硬件虚拟化**不等于"VM 一定比容器慢"**——KVM/virtio + EPT 的 VM 在网络/CPU 上可以非常接近原生；容器的优势主要在启动时间、内存密度和同一内核的零翻译开销，而不是"凡是 VM 都慢一个数量级"。
- 硬件虚拟化**不等于"一定更快"**——见 §4.2：没有 MMU 虚拟化的第一代 VT-x 在多数负载上慢于成熟 BT。

### 3.2 Popek & Goldberg 三条件与"敏感指令"

- **等价性（equivalence）**：guest 无法分辨自己是在真机还是 VMM 上（"essentially identical"）。
- **资源控制（resource control）**：guest 不能影响 VMM 或其它 guest 的资源。
- **效率（efficiency）**：所有**安全（innocuous/safe）指令**由硬件直接执行，不经过 VMM。

指令分类：
- **特权指令（privileged）**：只有特权态能执行，用户态执行必须陷入。
- **敏感指令（sensitive）**：会读取或修改特权机器状态。分为 **control-sensitive**（改特权状态）与 **behavior-sensitive**（行为依赖特权状态）。
- **安全指令（innocuous）**：两者都不是。

**定理 1**：若某架构中**每条敏感指令都是特权指令**，则可构造经典 VMM（guest 跑非特权态，敏感指令全部陷入）。
**定理 3（hybrid VMM）**：若每条 **user-sensitive** 指令是特权指令，可构造 hybrid VMM（放宽效率条件，允许部分安全指令被解释执行）。

### 3.3 x86 早期为什么 ring compression / ring aliasing 都失败

x86 有 4 个环，但真正的"内核态"只有 ring 0；VMM 必须占用 ring 0，guest 内核只能下移到 ring 1/3。于是：
- **ring compression**：ring 1 的段保护/权限检查语义与 ring 0 不同，guest 内核被降级后行为改变。
- **ring aliasing**：guest 能通过 `pushf`/`popf`/`iret`/段指令**观察到 CPL 不是 0**，等价性被破坏；而这些指令**不陷入**，VMM 无从拦截。
- 结果是 x86 **既不满足** Popek-Goldberg 定理，也**不满足** hybrid VMM 定理（部分 user-sensitive 指令不特权）。VMware 的唯一出路是**放宽等价性**：不支持 CPL 1/2，忽略 `sgdt`/`sldt`/`sidt`/`smsw` 这类"可用但无用"的指令。

### 3.4 三个常见误解

| 误解 | 事实 |
| --- | --- |
| "做虚拟化必须有 CPU 硬件支持" | Xen/VMware 在 2005 年前就商用了；半虚拟化与 BT 都是纯软件路线 |
| "VM 一定比容器慢很多" | 容器省掉的是第二套内核与地址翻译；在 CPU 密集与 virtio+EPT 的网络场景，VM 与原生/容器差距可小到几个百分点；差距主要在启动、密度、全模拟设备 |
| "有了 VT-x 就快了" | 第一代 VT-x/SVM 没有二级地址转换，`forkwait`、内核编译、页表修改密集负载上**比 BT 慢**（ASPLOS 2006） |

---

## 4. 软件虚拟化的性能损失量化（本稿核心）

> 记号：**BT** = 二进制翻译（VMware Workstation 3.2 / ESX 早期，全虚拟化）；**PV** = 半虚拟化（Xen）；
> **HW** = 第一代硬件辅助（VT-x/SVM，**无** EPT/NPT）。
> 除非另注，数据均为"相对原生"。

### 4.1 CPU 密集型负载

**Xen SOSP 2003，Figure 3**（单 CPU，双路服务器只用 1 颗；`L` = 原生 Linux，`X` = XenoLinux 半虚拟化，`V` = VMware Workstation 3.2，`U` = User-Mode Linux）：

| 基准 | 原生 Linux (L) | XenoLinux (X) | VMware WS 3.2 (V) | UML (U) |
| --- | ---: | ---: | ---: | ---: |
| SPEC INT2000（score，越高越好） | 567 | **567（≈0%）** | 554（−2.3%） | 550 |
| Linux 2.4.21 内核编译（秒，越低越好） | 263 | **271（论文自述 "mere 3% overhead"）** | 334（−21%） | 535 |

- **测试条件**：Xen 2.0 原型 / Linux 2.4，SPEC CPU2000 与内核编译；论文声明结果为 7 次试验的中位数。
- **不适用场景**：这是**半虚拟化**，guest 内核被改过；它**不能**代表"未修改 OS + BT"或"全模拟"的 CPU 开销。

**Adams & Agesen, ASPLOS 2006，Figure 2（SPECint2000 / SPECjbb2005）**：

- SPECjbb2005：软件 VMM（BT）达到原生的 **98%**，硬件 VMM 达到 **99%**（Windows 2003 x64 + Sun JVM 1.5.0）。
- SPECint2000：两种 VMM 都接近原生；论文正文提到两种 VMM 的开销均为**个位数百分比**，并特别指出 `mcf` 在两种 VMM 上都**比原生更快**（论文认为可能是 TLB/缓存被"良性扰动"）。
- **测试条件**：VMware 的软件 VMM（BT）vs 一个实验性的、面向 VT-x/SVM 的硬件 VMM；基准只跑用户态计算，几乎不触发特权操作。
- **关键结论**：**纯 CPU 密集负载上，成熟的二进制翻译几乎不落后于硬件辅助，两者都在几个百分点以内。**

### 4.2 特权指令 / 系统调用 / 进程创建密集负载

**Adams & Agesen, ASPLOS 2006，Figure 1/4 的"nanobenchmark"（FrobOS，单条或极短操作，cycles）**：

| 操作 | 原生 | BT 软件 VMM | 第一代硬件 VMM（无 EPT） | 说明 |
| --- | ---: | ---: | ---: | --- |
| `syscall`+`sysret` 往返 | 基础值 | **比原生多约 2000 cycles** | **原生速度**（不经过 VMM） | BT 多一层代码 + 一次特权转换 |
| `call`/`ret`（间接控制流） | 11 | **51（+40）** | 11 | BT 会改写间接调用/返回 |
| `in`（端口 0x80，I/O 指令） | 3209 | **约 214（比原生快 15×）** | **15826** | BT 把 `in` 翻成访问虚拟芯片模型的短指令序列；硬件 VMM 必须做 vmrun/exit 往返 |
| `cr8` 写（不产生中断） | ≈140（由"软件 35 cycles 约为原生 1/4"反推） | **35（约 4× 快于原生）** | ≈原生 | BT 翻成短指令序列 |
| `divzero`（纯 fault，不碰软件 MMU） | 889 | **3223（3.6×）** | **1014（约原生）** | 用来把"fault 开销"与"虚拟 MMU 开销"分离 |
| `pgfault`（真页错误，走软件 MMU） | 1093 | **3927（3.6×）** | **11242（10.3×）** | 硬件 VMM 的 vmrun/exit 往返比软件收 fault 更贵 |
| `ptemod`（修改一条页表项） | 1（单次 store） | **391** | **12733（≈软件 VMM 的 30×）** | 影子页表 + trace 一致性；硬件 VMM 每次修改都 exit |

**进程创建（`forkwait`，fork+waitpid 共 40000 次，Pentium 4 3.8GHz）**：

| 配置 | 时间 | 相对原生 |
| --- | ---: | ---: |
| 原生 | 6.0 s | 1.0× |
| BT 软件 VMM | 36.9 s | **6.1×** |
| 第一代硬件 VMM | 106.4 s | **17.7×** |

- 论文原文措辞：硬件 VMM 引入的**额外开销**约为软件 VMM 的「4.4 倍」；按表中原始时间直接计算，(106.4−6.0)/(36.9−6.0) ≈ **3.25×**。**两个数字口径不同，此处并存**。
- **不适用场景**：`forkwait` 是人为放大的极端微基准（每次循环都创建/销毁地址空间），**不代表真实业务**；论文自己也强调它"magnifies the difference"。

**内核编译（`make -j8 bzImage`，Linux 2.6.11）**：

| 配置 | 时间 | 相对原生 |
| --- | ---: | ---: |
| 原生 | 265 s | — |
| BT 软件 VMM | 393 s | 67.4% |
| 第一代硬件 VMM | 484 s | 54.8% |

- 硬件 VMM 的额外开销 219 s，接近软件 VMM 的 128 s 的 **1.7 倍**。
- **根因**：采样显示硬件 VMM 上 guest 花更多时间在 **page fault 与上下文切换**上（缺 EPT/NPT 时每次页表相关操作都要 exit）。
- **测试条件**：VMware Player 1.0.1（软件 VMM）与实验性硬件 VMM；Linux guest；P4 级硬件。
- **反例（硬件 VMM 更快的负载）**：单地址空间、I/O exit 少的负载，如 Windows 上的 Apache ab（硬件 67% vs 软件 53% 原生）与 2D PassMark（硬件 70% vs 软件 63%）。论文称之为"证明规则的例外"。

**Microarchitecture 演进（Adams & Agesen Table 1，cycles）**：

| 事件 | 3.8GHz P4 672 | 2.66GHz Core 2 Duo | 变化 |
| --- | ---: | ---: | ---: |
| VM entry | 2409 | 937 | −61%（时间上 634 ns → 352 ns，−44%） |
| Page-fault VM exit | 1931 | 1186 | 时间 508 ns → 446 ns，−12% |
| VMCB read | 178 | 52 | — |
| VMCB write | 171 | 44 | — |

**后续微架构的 VM exit 成本（转引自 Bugnion/Nieh/Tsafrir《HSSV》Table 4.3，单位 cycles，page-fault 触发的 vmexit）**：

| 微架构（年份） | 成本 |
| --- | ---: |
| Prescott (2005) | 1926 |
| Merom (2006) | 1156 |
| Penryn (2008) | 858 |
| Westmere (2010) | 569 |
| Sandy Bridge (2011) | 507 |
| Ivy Bridge (2012) | 466 |
| Haswell (2013) | 512 |
| Broadwell (2014) | 531 |

- **结论**：VM exit 从 ~2000 cycles 降到 ~500 cycles 量级，是 2006 之后硬件辅助虚拟化"由劣转优"的重要背景，但**真正解决 MMU 问题的是 EPT/NPT**。

### 4.3 I/O 密集型负载

**Xen SOSP 2003，Table 6：ttcp 带宽（Mb/s，越高越好）**：

| 配置 | TCP MTU 1500 TX | TCP MTU 1500 RX | TCP MTU 500 TX | TCP MTU 500 RX |
| --- | ---: | ---: | ---: | ---: |
| 原生 Linux | 897 | 897 | 602 | 544 |
| **XenoLinux（半虚拟化网卡）** | **897（−0%）** | **897（−0%）** | 516（−14%） | 467（−14%） |
| VMware Workstation 3.2（模拟 pcnet32） | **291（−68%）** | 615（−31%） | **101（−83%）** | 137（−75%） |
| UML | 165（−82%） | 203（−77%） | 61.1（−90%） | 91.4（−83%） |

- **测试条件**：同一台双路服务器、同一 guest 镜像与磁盘分区；VMware 用其模拟网卡驱动。
- **不适用场景**：这是 **VMware Workstation 3.2（2003 年的托管型产品）**，**不是 ESX Server**；论文明确指出 ESX Server 的 `vmxnet` 专用驱动"能显著改善网络性能"，且 ESX 在上下文切换等微基准上比 Workstation 好。**不要把 −68% 当作现代 KVM/virtio 的数字。**
- **机制解释**：模拟网卡每个包都要 VMM 介入模拟寄存器/DMA；virtio 式的 ring + 共享内存 + 批量通知把 per-packet 开销降到接近原生。

**Apache 服务器吞吐（Adams & Agesen Figure 3，相对原生）**：

| guest / VMM | 相对原生 |
| --- | ---: |
| Windows + 软件 VMM | 53% |
| Windows + 硬件 VMM | 67% |
| Linux + 软件 VMM | **45%** |
| Linux + 硬件 VMM | 38% |

- **根因**：VMware Player 采用**托管型 I/O 模型**，所有网络包都穿过宿主 OS 的 I/O 栈；Windows 单地址空间、Linux 多地址空间导致两者表现相反。
- **测试条件**：Apache `ab`，128 客户端，Windows 2003 / Linux guest，VMware Player 1.0.1。
- **QEMU e1000/IDE vs virtio 的具体百分比**：本轮**未能找到可考证的一手实测数字**（任务要求的这一条）。可用的**同构代理证据**是上表 Xen 的"半虚拟化 NIC vs 全模拟 NIC"对比（897 vs 291 Mb/s）。virtio 的设计动机与接口见 Russell《virtio》（SIGOPS OSR 2008），但该文以设计与标准化为主，本稿不为其编造吞吐数字。→ **待补：QEMU/virtio 实测（建议用同一宿主的 iperf3/fio 自行复现，比较 e1000 与 virtio-net、IDE 与 virtio-blk）。**

### 4.4 内存 / MMU

**（a）影子页表：软件 MMU 虚拟化的开销**

- Xen/VMware 的自述：QEMU 全虚拟化路径下，guest 页表对 MMU 不可见，VMM 维护"影子页表"，**每次 guest 修改页表都要陷入/校验**，并需要显式同步 accessed/dirty 位；Xen 之所以快，是因为它**直接让 guest 页表对 MMU 可见（只读注册）**，绕开了影子页表。
- 量化：ASPLOS 2006 的 `pgfault`（原生 1093 → BT 3927 cycles，3.6×）与 `ptemod`（原生 1 次 store → BT 391 cycles；**第一代硬件 VMM 12733 cycles**）。
- bhyve 论文转引 AMD-V Nested Paging 白皮书：**"在某些负载下，影子页表可占整个 hypervisor 开销的 75%"**（原始出处为 AMD 白皮书，属**二手转引**，未核对原文）。
- 次生代价：**guest 页表 write-protect + TLB flush** 在影子方案下会产生大量 exit/陷入；Xen 通过批量 hypercall 把 2048 条页表更新合并成一次调用（"每次 hypercall 构造 8MB 地址空间"）来摊销。

**（b）EPT/NPT 之后**

- 机制：guest 页表 + 宿主页表两级遍历由硬件完成，不再需要影子页表与写保护 trap。
- bhyve（FreeBSD，Xeon E3-1220 v3，guest 2 vCPU/8GB）实测：
  - `make -j4 buildworld`：guest 内存 wired 基线 **2207 s**；不 wired **2225 s（+18 s，+0.8%）**；A/D 位软件模拟 **2276 s（+51 s，+2.3%）**。
  - GUPS（1 vCPU/24GB，12GB 工作集，CPU 时间）：guest 4KB + host 4KB 禁用大页 = **500 s**；guest 与 host 都启用大页 = **102 s**（**约 4.9× 提速**）。
- TPT（ATC'23）结论：在页面翻译密集路径上，**EPT 优于影子页表，但仍有二维遍历开销**；其提出的 Translation Pass-Through 声称接近原生。→ 说明 EPT/NPT 不是"零成本"，只是把成本从 trap 换成硬件遍历，**大页是缓解关键**。
- **测试条件**：以上都是特定 CPU/内存配置下的结果，**不能外推到所有代际**。

**（c）内存容量管理（不是"变慢"，是"省内存"）——Waldspurger, OSDI 2002**

| 场景 | 结果 |
| --- | --- |
| 最佳情况：1–10 台相同 Linux VM 跑 SPEC95，每台 40MB | 共享率随 VM 数上升，10 台时**接近 67%**，**近 60% 的 VM 内存被回收**；单台 VM 也回收近 5MB，其中约 55% 来自零页 |
| 生产负载 A：10 台 Windows NT VM（Oracle/SQL Server/IIS 等） | 回收 **近 1/3** VM 内存，**673 MB** |
| 生产负载 B：9 台 Linux VM（64–768MB） | 回收 **18.7%**，**345 MB**（其中 70MB 为零页） |
| 生产负载 C：5 台 Linux VM（32–512MB） | 回收 **约 7%**，**120 MB**（其中 25MB 为零页） |
| 页共享自身空间开销 | **< 0.5%** 系统内存 |
| 页共享对 CPU 的影响 | **可忽略**：聚合吞吐比关闭共享高 **0.5%**，区间 **−1.6% ~ +1.8%** |
| 空闲内存税（τ 从 0 提到 0.75） | 活跃 Linux VM 的 dbench 吞吐**提升超过 30%** |

- **测试条件**：ESX Server 早期版本；双 933MHz Pentium III（SPEC 实验，每 VM 40MB）/ 双 800MHz Pentium III + 512MB（空闲税实验）。
- **重要区分**：Waldspurger 讲的是**内存容量与分配策略**，**不是** MMU 翻译开销；把它当作"内存虚拟化慢多少"的证据是误读。

### 4.5 Xen 的并发规模开销

- 128 个并发 CPU 密集 domain（SPEC CINT2000 子集），Xen 默认 **5ms 时间片**：聚合吞吐相对原生 Linux **损失 7.5%**；把时间片调到 **50ms**（ESX 默认值）后差距"几乎消失"，但交互延迟变差。
- 该场景下用户态到用户态 UDP 延迟：128 个繁忙 domain 时平均 **147ms**（s.d. 97ms）；对一个空闲 domain 平均 **5.4ms**（s.d. 16ms）。
- **测试条件**：双 CPU 服务器，Xen 2.0 原型；**这是极端过载实验**，不代表常见部署。

### 4.6 分档总表：负载类型 → 软件虚拟化开销量级 → 硬件辅助后 → 数据来源

| 负载类型 | 软件虚拟化（BT / 全模拟） | 半虚拟化（PV） | 硬件辅助（有 EPT/NPT） | 数据来源 |
| --- | --- | --- | --- | --- |
| 纯 CPU 计算（SPECint/SPECjbb） | **≈0–3%** | **≈0–3%** | **≈0–2%** | Xen SOSP'03 Fig.3；ASPLOS'06 Fig.2 |
| 内核编译（I/O+调度+内存混合） | 原生 265s → BT 393s（**67.4%**） | Xen：271s vs 263s（**≈3%**） | 第一代 HW：484s（54.8%）；有 EPT/NPT 后显著改善（未取得同条件数字） | ASPLOS'06 §6.1；Xen Fig.3 |
| 进程创建 / 地址空间切换（forkwait） | **6.1×** 原生 | 未测（Xen 的 fork/exec 因批量 hypercall 而较优） | 第一代 HW **17.7×**（比 BT 还差） | ASPLOS'06 §6.2 |
| 系统调用往返 | **每条 +≈2000 cycles** | 接近原生（fast syscall handler） | 原生速度 | ASPLOS'06 §6.3 |
| 页错误（pgfault） | **3.6×**（3927 vs 1093 cycles） | 较低（页表直接可见，但仍需 hypercall） | 第一代 HW **10.3×**（11242 cycles） | ASPLOS'06 §6.3 |
| 页表项修改（ptemod） | BT 391 cycles/次 | 批量 hypercall 摊销 | 第一代 HW **12733 cycles**（≈30× BT） | ASPLOS'06 §6.3 |
| 网络 TCP 吞吐（MTU 1500 TX） | VMware WS 3.2：**−68%** | Xen：**−0%** | virtio+EPT 接近原生（具体数字待补） | Xen SOSP'03 Table 6 |
| 网络 TCP 吞吐（MTU 500） | −75% ~ −90% | −14% | 接近原生（待补） | Xen SOSP'03 Table 6 |
| Web 服务（Apache ab） | 45%–53% 原生 | Xen SPEC WEB99 单实例 **−0.8%**（514 vs 518） | 38%–67%（第一代，托管 I/O 拖累） | ASPLOS'06 Fig.3；Xen Fig.3 |
| 内存容量（页共享回收） | —（全模拟同样可共享） | — | —（需硬件 RMP/加密时才涉及） | Waldspurger OSDI'02 |
| VM exit/entry 固定成本 | —（BT 无此成本，改为翻译开销 2300 cycles/指令） | —（hypercall 更轻） | ~1900 → ~500 cycles（2005→2014） | ASPLOS'06 Table 1；HSSV Table 4.3 |

> BT 的翻译器吞吐：**2300 cycles/条 x86 指令**（对比某些优化 JIT 的 100–200k cycles/Java 字节码），且翻译块缓存（TC）命中后摊销；**warm-up 未命中**只在冷代码（如 boot）里显著。→ 这是"BT 在 CPU 密集长跑中接近原生、但在短命/高切换负载中变差"的原因。

---

## 5. 对照表（速查）

| 负载 | 软件虚拟化损失量级 | 关键条件 | 一句话 |
| --- | --- | --- | --- |
| CPU 密集 | 0–5% | 成熟 BT 或 PV；长跑、TC 命中 | 几乎无损 |
| 系统调用密集 | 每条 +~2000 cycles | BT；取决于 syscall 频率 | 单次贵，端到端看频率 |
| 进程/地址空间密集 | 3–18× | 无 EPT/NPT 时最差 | 软件虚拟化的真正软肋 |
| MMU/页表密集 | 3–6×（BT）/ 10–30×（第一代 HW） | 影子页表 or 无二级翻译 | EPT/NPT 前后是两个世界 |
| 全模拟 I/O | 30–90% 吞吐损失 | VMware WS 3.2 模拟 NIC | 换 virtio/直通可解决 |
| 半虚拟化 I/O | 0–15% | Xen + 半虚拟化驱动 | 接近原生 |
| 内存**容量** | 省 7–60%（不是损失） | 页共享；高度依赖负载同质性 | 别和 MMU 开销混为一谈 |

---

## 6. 未决问题

1. **QEMU e1000/IDE vs virtio 的一手实测数字缺失**：本稿只能用 Xen 的模拟 NIC vs 半虚拟化 NIC 作代理。建议在同一宿主上用 `iperf3`（e1000 vs virtio-net）与 `fio`（IDE vs virtio-blk）复现，并记录 CPU 占用。
2. **EPT/NPT 相对影子页表的"同条件"加速比**：bhyve/TPT 给出了方向与个别数字（GUPS 大页 4.9×、buildworld +0.8%~2.3%），但缺少"同一负载、同一硬件、只切换 EPT on/off"的系统性对照。
3. **ASPLOS 2006 的"4.4 倍"与按原始时间算出的 3.25 倍口径不一致**，需核对论文原文上下文。
4. **Firecracker 的启动延迟 / 内存开销具体数字未核对原文**（常见引用 125ms / <5MiB），本稿标为待证实。
5. **ARM VHE 的"2016"**：规范年份（ARMv8.1-A，2014）与硬件落地年份（约 2016）在不同来源里口径不一。
6. **机密计算（SEV-SNP / TDX）的性能开销**：本稿只列时间线，**未取得可信的同条件 benchmark**，不给出百分比。
7. **容器 vs VM 的现代同条件对比**：本稿没有纳入（超出本次范围），但"VM 一定慢"的说法需要现代 NVMe + virtio + EPT + SR-IOV 的数据来证伪。

---

## 7. 参考资料

访问日期均为 **2026-09-16**。

**一手论文 / 官方资料**

1. P. Barham, B. Dragovic, K. Fraser, S. Hand, T. Harris, A. Ho, R. Neugebauer, I. Pratt, A. Warfield, *Xen and the Art of Virtualization*, SOSP 2003. PDF: https://www.cl.cam.ac.uk/research/srg/netos/papers/2003-xensosp.pdf
   - HTML 镜像（用于交叉核对表格）：https://docshare.tips/xen-and-the-art-of-virtualization_588faf3eb6d87f36078b4e55.html
2. K. Adams, O. Agesen, *A Comparison of Software and Hardware Techniques for x86 Virtualization*, ASPLOS 2006. PDF: https://web.stanford.edu/class/cs240/readings/asplos235_adams.pdf （VMware 原始 `asplos235_adams.pdf` 链接已 404）
3. C. A. Waldspurger, *Memory Resource Management in VMware ESX Server*, OSDI 2002. HTML: https://static.usenix.org/publications/library/proceedings/osdi02/tech/waldspurger/waldspurger_html/
   - 页共享实现与实测：`node11.html`；内容哈希共享：`node10.html`；空闲税实验：`node16.html`
4. G. J. Popek, R. P. Goldberg, *Formal Requirements for Virtualizable Third Generation Architectures*, CACM 17(7), 1974. DOI: https://dl.acm.org/doi/10.1145/361011.361073
5. E. Bugnion, S. Devine, M. Rosenblum, *Disco: Running Commodity Operating Systems on Scalable Multiprocessors*, SOSP 1997 / ACM TOCS 15(4) 1997. DOI: https://dl.acm.org/doi/10.1145/265924.265930
6. R. Russell, *virtio: towards a de-facto standard for virtual I/O devices*, ACM SIGOPS OSR 42(5), 2008. DOI: https://dl.acm.org/doi/10.1145/1400097.1400108
7. E. Bugnion, J. Nieh, D. Tsafrir, *Hardware and Software Support for Virtualization*, Morgan & Claypool, 2017. DOI: https://www.morganclaypool.com/doi/abs/10.2200/S00754ED1V01Y201701CAC038
8. N. Asama 等，*Nested Paging in bhyve*（FreeBSD）。PDF: https://people.freebsd.org/~neel/bhyve/bhyve_nested_paging.pdf
   - 其中"影子页表可占 hypervisor 开销 75%"转引自 *AMD-V Nested Paging White Paper*（二手转引）
9. L. Vilanova 等，*Translation Pass-Through for Near-Native Paging Performance in VMs*, USENIX ATC 2023. PDF: https://www.doc.ic.ac.uk/~lvilanov/publications/files/atc23_tpt.pdf

**教学 / 二手综述（用于时间线与概念交叉核对）**

10. Harvard CS161 (2018), *Lecture 25: Virtual machines*（基于 HSSV 教材；17 条敏感指令、VM 切换成本表）。 https://read.seas.harvard.edu/cs161/2018/lectures/lecture25/
11. D. Thain, *The Virtualization Theorem Ignored for Three Decades*, Notre Dame CCL, 2010. https://ccl.cse.nd.edu/blog/2010/the-virtualization-theorem-ignored-for-three-decades/
12. UW–Madison CS736 学生评述（Disco 开销 3%–16% 等二手转述）。 https://pages.cs.wisc.edu/~swift/classes/cs736-sp15/blog/2015/02/disco_running_commodity_operat.html
13. 搜索结果中给出的 VT-x / SVM / Itanium VT-i 时间线二手来源（InfoWorld/InformationWeek 等报道），**本稿仅用作时间线的旁证，未逐条核对原文**。

**明确标注为未证实 / 存在分歧的项**

- Firecracker 启动延迟与内存开销具体数值（未取得 NSDI 2020 原文）。
- QEMU e1000/IDE 与 virtio 的具体吞吐/延迟差距（未找到可考证一手实测）。
- SEV / SEV-SNP / TDX 的"年份"在不同二手来源口径不一。
- VT-i 的首次公布与首批量产年份（不同来源分别为 2002 与 2006）。
