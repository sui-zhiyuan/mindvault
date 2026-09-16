# 高风险项交叉验证记录

> 建立：2026-09-16 ｜ 类型：中间稿
> 目的：对本次调研中**最容易被想当然写错**的两项做独立复核，不只依赖单一路径（尤其不只依赖模型记忆）。
> 说明：本文件记录**验证过程与证据链**，结论会被汇总稿引用；每条都标注已验证 / 未证实。

---

## 验证项 1：鲲鹏是否支持硬件嵌套虚拟化

**这是本次调研风险最高的一条**：鲲鹏 + 虚拟化是仓库的重点主题，而"支持嵌套虚拟化"是一个极容易被营销材料含糊过去的卖点。

### 结论（已验证 —— 证据链闭合）

| 能力 | ARM 架构要求 | 鲲鹏 920（TaiShan v110） | 判断 |
|---|---|---|---|
| EL2 / Stage-2 翻译 | ARMv7 虚拟化扩展起 | ✅ | 支持（KVM on ARM64 可跑） |
| VHE（Virtualization Host Extensions） | ARMv8.1 | ✅（8.2 包含 8.1） | 支持 |
| **硬件嵌套虚拟化 FEAT_NV / NV2** | **ARMv8.3 / ARMv8.4** | ❌（仅 ARMv8.2-A） | **不支持** |

即：鲲鹏 920 具备"跑虚拟机"的全部硬件基础，但**没有"在虚拟机里再跑一个 hypervisor"的硬件扩展**。

### 证据链

**证据 A（强）—— arm64 的 KVM 嵌套虚拟化只支持 FEAT_NV2，而 FEAT_NV2 是 ARMv8.4 特性：**
- 补丁系列标题直接写明范围：`[PATCH v11 00/43] KVM: arm64: Nested Virtualization support (FEAT_NV2 only)`（2023-11），
  以及更早的 `[PATCH v10 00/59] KVM: arm64: ARMv8.3/8.4 Nested Virtualization support`（2023-05）。
  → 上游最终只接受 **FEAT_NV2**（ARMv8.4）路线，早期 ARMv8.3 的路径被收窄。
- 来源：[lists.openwrt.org 镜像 v11](http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-November/882814.html)、
  [v10](http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-May/833892.html)、
  [spinics 镜像](http://mail.spinics.net/lists/kvm/msg333509.html)

**证据 B（强）—— 该支持已于 Linux 6.16 合入主线：**
- `Merge tag 'kvmarm-6.16'`，2025-05-26，经 Paolo Bonzini 合入。
  → **2025 年之前，arm64 上根本没有可用的 KVM 嵌套虚拟化**，这与 x86 在 2010 年代早已成熟的局面形成鲜明对比。
- 来源：[linux.git merge tag kvmarm-6.16](http://git.armlinux.org.uk/cgit/linux.git/log/scripts?id=7f904ff6e58d398c4336f3c19c42b338324451f7&showmsg=1)

**证据 C（强，官方一手）—— 鲲鹏 920 官方规格明示架构为 ARMv8.2：**
- HiSilicon 官方产品页 Kunpeng 920 的 Key Features 表格逐字写有：`Architecture • ARMv8.2`。
  来源：[HiSilicon · Kunpeng 920](https://www.hisilicon.com/en/products/kunpeng/huawei-kunpeng/huawei-kunpeng-920)（访问 2026-09-16）
- 鲲鹏社区官方页面同样写明"**Armv8.2 指令集**"，并列出官方规格：2 个 CPU Die + 1 个 IO Die、
  最多 64 核、8 通道 DDR4、PCIe 4.0 / CCIX / 100GE、HCCS 片间一致性互联（最多 4 路、256 物理核 NUMA）、
  TrustZone 可信执行环境（GP API）、内置 SM3/SM4/AES/SHA-2/RSA 加解密引擎、硬件可信根安全启动。
  来源：[鲲鹏920处理器 · 鲲鹏社区](https://www.hikunpeng.com/zh/compute/kunpeng920)（访问 2026-09-16）
- ⚠️ **重要观察**：官方规格页**完全没有提及任何虚拟化特性**（EL2 / Stage-2 / vGIC / SMMU 一项都没列）。
  这不等于"不支持" —— ARMv8-A 服务器 SoC 的虚拟化能力属架构标配、通常不作为卖点；
  但它意味着**官方材料无法正面证明任何虚拟化扩展的存在**，只能由架构版本（ARMv8.2 ⇒ 含 ARMv8.1 的 VHE，
  但不含 ARMv8.4 的 FEAT_NV2）来界定能力边界。

**证据 C''（弱，旁证，且含矛盾）—— 华为云社区个人博客：**
- 该博客同样自述"基于 ARM v8.2 指令集"，与官方规格一致。
  来源：[一文学会华为云鲲鹏服务器全栈架构](https://bbs.huaweicloud.com/blogs/475584)（2026-03-23）
- ⚠️ 但来源可靠性低：页面底部明示"本内容来自华为云开发者社区博主，**不代表华为云及华为云开发者社区的观点和立场**"，
  且同文给出多条无法核实或明显夸大的数据（"鲲鹏920 SPECint 930 分 vs Xeon 8380 780 分"、
  "三年 TCO 每台节约约 15 万元"、"容器镜像体积减少 40%"）。**只能当线索，不能当依据。**

**证据 D（矛盾点，重要）：**
- 同一篇社区博客在"虚拟化性能突破"一节声称"**嵌套虚拟化：支持 VM 内再运行 VM**，满足开发测试场景"。
- 这与证据 A/B 直接冲突：ARMv8.2 没有 FEAT_NV2，Linux 侧要到 6.16（2025）才支持 arm64 嵌套。
- **判断**：该说法要么指软件模拟（性能不可用）、要么指受限/特定版本，要么就是营销式表述。**不能采信为"鲲鹏 920 支持嵌套虚拟化"。**

### 待补证据（下一步，不影响主结论）

- 鲲鹏 950 的架构版本（是否 ARMv8.4+/ARMv9，从而首次具备 FEAT_NV2 硬件嵌套）—— **未证实**。
  若核实到 950 为 ARMv8.4+，则"鲲鹏平台自 950 起可能首次具备硬件嵌套能力"是可写的结论；
  在核实前只能写"未证实"。
- 社区侧对"鲲鹏上能否跑嵌套"的实际反馈：华为云论坛帖子
  [鲲鹏920支持 KVM嵌套虚拟化吗](https://bbs.huaweicloud.cn/forum/thread-0229132917577718008-1-1.html) 抓取为空（JS 渲染），待换方式核实。
- 鲲鹏是否提供 vSMMU / 设备直通 / 热迁移的官方支持矩阵，以及 openEuler StratoVirt 在鲲鹏上的成熟度 —— 由 04 号中间稿负责。

### 汇总稿中的表述纪律

必须写成："鲲鹏 920 是 ARMv8.2（**HiSilicon 官方规格页证实**），具备 EL2/Stage-2 与 VHE（ARMv8.1 起），
但**缺少 FEAT_NV2（ARMv8.4）这一硬件嵌套虚拟化扩展**；arm64 的 KVM 嵌套虚拟化直到 Linux 6.16（2025）
才落地且要求 FEAT_NV2。因此鲲鹏 920 上'VM 里跑 hypervisor'没有硬件加速路径。"
**不要**写成"鲲鹏支持嵌套虚拟化"，也不要凭架构版本推断后就当作已证事实。
**只写事实**：官方规格页未列出任何虚拟化特性，能力边界系由架构版本界定 —— 表述时把这一推理过程写出来，而不是直接断言。

---

## 验证项 2：软件虚拟化的性能损失到底多少

### 已验证（原始论文）

**Xen 2003（SOSP'03）原文自述**（摘要，可逐字引用）：

> "we allow operating systems such as Linux and Windows XP to be hosted simultaneously for a
> **negligible performance overhead — at most a few percent** compared with the unvirtualized case."

→ **半虚拟化（Xen 路线）的开销量级 = 几个百分点**。
来源：[Xen and the Art of Virtualization 全文](https://studyres.com/doc/8893844/xen-and-the-art-of-virtualization)（Barham et al., SOSP 2003）

**同一论文对"另一条路线"（VMware ESX 的二进制翻译）的描述**（原始表述，可引用）：

> "VMware's ESX Server dynamically rewrites portions of the hosted machine code to insert traps
> wherever VMM intervention might be required. This translation is applied to the **entire guest OS
> kernel** (with associated translation, execution, and caching costs) since all non-trapping
> privileged instructions must be caught and handled."

> "ESX Server implements shadow versions of system structures such as page tables and maintains
> consistency with the virtual tables by trapping every update attempt — **this approach has a high
> cost for update-intensive operations such as creating a new application process**."

→ 这是"软件虚拟化慢在哪"的**同期权威表述**：慢的不是纯计算，而是**特权指令拦截 + 影子页表同步**，
代价集中在**页表/地址空间更新密集**与**系统调用/陷阱密集**的负载上。

**对比结论（已验证方向）**：Xen 明确通过**不做影子页表**（guest 页表只读注册给 MMU + 批量 hypercall 校验）
来规避这部分开销，这正是它能把开销压到"几个百分点"的原因。

### 未完全验证（需 01 号中间稿以原文核实）

- **Adams & Agesen, "A Comparison of Software and Hardware Techniques for x86 Virtualization", ASPLOS 2006**
  （DOI `10.1145/1168918.1168860`）—— 论文存在性已确认，是"二进制翻译 vs 第一代硬件辅助"的**标准出处**；
  其知名结论方向是"**第一代 VT-x（无 EPT）并不自动更快**，BT 与硬件辅助各有强弱，取决于陷阱频率"。
  → **具体百分比数字未在本轮取得原文**，标记为「待核实」，不得凭记忆引用具体倍数。
- 我尝试但失败/不可用的来源（记录以免重复劳动）：
  - `https://www.vmware.com/pdf/asplos235_adams.pdf` → HTTP 404（Broadcom 收购后 vMware 站点 PDF 已下架）
  - `https://lms.su.edu.pk/download?...vmware-paravirtualization.pdf` → HTTP 403
  - `https://en.wikipedia.org/wiki/X86_virtualization` → fetch failed
  - `https://en.wikichip.org/wiki/hisilicon/microarchitectures/taishan_v110` → fetch failed
- **VMware 官方白皮书**《Understanding Full Virtualization, Paravirtualization, and Hardware Assist》
  —— 仍是"软件虚拟化 vs 硬件辅助"的最佳厂商侧出处，本轮未取到，待 01/05 稿处理。

### 汇总稿中的表述纪律

回答"软件虚拟化性能损失大概多少"时**必须分档**，不能给单一数字：

| 负载类型 | 软件虚拟化（无硬件辅助）开销量级 | 原因 |
|---|---|---|
| CPU 密集 / 计算密集 | 低（接近原生，二进制翻译的翻译块可缓存） | 大多数指令直接执行 |
| 特权指令 / syscall 密集 | 显著上升 | 陷阱-模拟或翻译开销 |
| 页表 / 地址空间更新密集 | **最高**（影子页表同步） | 每次更新都要 trap 并同步影子表 |
| I/O 密集（模拟设备） | 高 | 设备寄存器访问逐次陷入 |
| I/O 密集（半虚拟化驱动） | 低 | 共享内存环 + 批量提交，不陷入 |

并注明**数据来源与测试条件**，避免"损耗 5%~20%"式无依据断言。

---

## 验证项 3：仓库存量笔记的覆盖边界（避免重复或矛盾）

- `docs/tools-and-tips/memory-access-summary.md`：第 79 行只把"虚拟化对内存的影响"列为
  Drepper《What Every Programmer Should Know About Memory》第 4 章的一个话题，**正文并未展开**。
  → 该笔记**不覆盖**虚拟化内存开销（EPT/NPT vs shadow page table、TLB、大页），
  汇总稿需自行补齐，**无冲突风险，也无内容可复用**。
- `docs/rust-kunpeng/` 下三份沙箱笔记与虚拟化的映射关系，见 `07-existing-notes-map.md`。

---

## 验证项 4：ARM 嵌套虚拟化的代际划分与上游状态（修正 02 稿）

`02` 稿初版写"直到 ARMv8.4-NV 才提供 `HCR_EL2.NV` 与 `VNCR_EL2`"。核实后需修正为**两代划分**，但**不臆断两代各自对应哪个架构版本**：

- 上游 KVM 补丁系列的标题本身就反映了这段历史：
  - v10（2023-05）：`[PATCH v10 00/59] KVM: arm64: ARMv8.3/8.4 Nested Virtualization support`
  - v11（2023-11）：`[PATCH v11 00/43] KVM: arm64: Nested Virtualization support (FEAT_NV2 only)`
- v11 的 cover letter 由 **Marc Zyngier** 写明放弃原有 FEAT_NV 的理由（**原文**）：
  > "Drop support for the original FEAT_NV. **No existing hardware supports it without FEAT_NV2, and the architecture is deprecating the former entirely.** This results in fewer patches, and a slightly simpler model overall."
- 因此**可安全断言的事实**（4 条）：
  1. 该能力在架构上分 **FEAT_NV** 与 **FEAT_NV2** 两代；
  2. 上游 KVM **只支持 FEAT_NV2**，且**没有任何已存在硬件只实现 FEAT_NV 而不实现 FEAT_NV2**；
  3. 架构正在**废弃 FEAT_NV**；
  4. ⇒ **事实上的硬件嵌套门槛 = FEAT_NV2**。
- **不臆断**：FEAT_NV 名义上究竟属 ARMv8.3 还是 ARMv8.4 —— 未取得 Arm ARM 逐条定义
  （Arm developer 文档站抓取被重定向到 support.arm.com 而失败）。
  汇总稿写法应为"分 FEAT_NV / FEAT_NV2 两代（KVM 早期系列题为 ARMv8.3/8.4）"，
  **不要**写成"FEAT_NV 是 ARMv8.3"这类定点断言。
- 合入时间（**部分证实；我自己先前的"已核实 6.16"属过度断言，此处降级**）：
  - **已证实**：v11（2023-11）仍在评审阶段，Zyngier 当时的目标是"把本系列的一个前缀带进 6.8"
    → 说明**到 6.8 为止，完整支持尚未进入主线**。
  - **已证实**：**6.17（2025-07）的 KVM/arm64 合并说明把 "Nested support for FEAT_RAS and FEAT_DoubleFault2"
    列为新特性**（原文：*"Nested support for FEAT_RAS and FEAT_DoubleFault2, allowing the guest hypervisor to
    inject external aborts into an L2 VM"*）——这是在**既有嵌套支持之上追加能力**，
    说明**基础嵌套支持早于 6.17**。
  - **未逐条核实**：`Merge tag 'kvmarm-6.16'`（2025-05-26）。我尝试从 KernelNewbies 的 Linux 6.16 页、
    Elixir 源码交叉引用、以及 git 日志三处取证，**均未取得该 merge 的正文**；
    只能确认 `arch/arm64/kvm/nested.c` 在 v6.16 中**存在**，而该文件可能由更早的铺垫补丁引入
    （v11 cover letter 明确说 "NV trap forwarding, per-MMU VTCR" 等补丁**已经合并**）。
    → 因此**"基础支持在 6.16 合入"降级为待核实**。
  - **可安全写进定稿的表述**：上游 arm64 的嵌套虚拟化（FEAT_NV2）**直到 2025 年才进入主线** ——
    2023 年底仍在评审；2025-07 的 6.17 已在其之上追加 FEAT_RAS/DoubleFault2 的嵌套支持。
  - → 与 x86 的 KVM 嵌套（**Linux 3.1，2011**）相差约 **14 年**。
    这个量级（2025 vs 2011）**不依赖"具体是 6.16 还是 6.17"**，因此结论稳健。
- 对鲲鹏的含义不变且更强：鲲鹏 920 = **ARMv8.2**，连 FEAT_NV 都不具备 → **无任何硬件嵌套路径**。

---

## 验证项 5：产品性能指标必须回到厂商一手页（修正 02 稿）

`02` 稿初版把 Firecracker 写成"约 5 μVM/核/秒创建率"，**该数字查无实据，已修正**。
Firecracker 官网（一手）给出的官方指标是：

| 指标 | 官方值 |
|---|---|
| 启动时间 | boot in **<125 ms** |
| 创建速率 | **up to 150 microVMs per second per host** |
| 单实例内存开销 | **<5 MiB overhead per VM** |
| 设备模型 | **only 5 emulated devices**（virtio-net / virtio-block / virtio-vsock / serial console / 最小键盘控制器） |
| 硬件前提 | 64-bit Intel、AMD、Arm，需硬件虚拟化支持 |

顺带核实的两点：
- **与本仓库笔记一致**：`docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` 称 Firecracker 为"极简（3 virtio 设备）"——
  与官方"5 个模拟设备"**不矛盾**：官方把 serial console 与键盘控制器也计入模拟设备，virtio 设备恰为 3 个。
- **血缘印证**：Firecracker 源自 Chromium OS 的 crosvm，与 crosvm 共同构成 rust-vmm 社区 ——
  这正是 `01`/`06` 稿要讲的"Rust 化"趋势的一手证据。
- ⚠️ 注意：CubeSandbox 笔记里的"冷启动 <60ms、单实例 <5MB"与 Firecracker 官方指标高度接近，
  引用时**不要**把厂商宣称值当作第三方实测值。

**通用纪律**：凡产品性能指标（启动时间、内存开销、密度），定稿必须回到**厂商官方页**取数并注明"厂商宣称"，
与论文实测值分开标注、互不冒充。

---

## 验证项 6：Turtles（OSDI 2010）数字的可核实程度

`02` 稿的性能表全部引自 Turtles 项目。我独立复核了能拿到的部分。

**已核实（USENIX 官方摘要页，一手）** —— 论文自述的头部结论，原文：

> "Despite the lack of architectural support for nested virtualization in the x86 architecture,
> it can achieve performance that is **within 6-8% of single-level (non-nested) virtualization**
> for common workloads, through *multi-dimensional paging* for MMU virtualization and
> *multi-level device assignment* for I/O virtualization."

由此独立确认三点，均与 `02` 稿一致：
1. **"常见负载 6–8%"这个量级**；
2. 两个关键机制名：**multi-dimensional paging** / **multi-level device assignment**；
3. 论文处理的是**未经修改的 hypervisor**（KVM 与 VMware）——与 `02` 稿"用 VMware Server 作 L1"的用法一致。

**未能逐项复核**：表中 14.5% / 10.3% / 7.82% / 6.3% / 14.98% / 8.85% / 2.28%→5.17% /
13,000→2,000 / 837、469 Mb/s 等**逐项数字**来自论文正文。
USENIX 的正文与 slides 均为 `application/pdf`，本环境的 `web_fetch` **不支持 PDF 内容类型**，无法逐项比对。
→ **定稿纪律**：引用这些逐项数字时必须注明"依据 `02` 稿对论文正文的记录，未逐项复核"；
只有 **"常见负载 6–8%"** 这一条可作为**已独立核实**的数字使用。

**一处必须在定稿时点明的口径差异**：摘要说的是"**common workloads** 6–8%"，
而 `02` 稿把 kernbench 的 **14.5%** 也放在同一张表里。二者不矛盾
（kernbench 是全表最差的一档，不属于"常见负载"的平均），
但定稿**必须写明"6–8% 是常见负载的区间，重负载可到约 15%"**，
否则读者会以为论文只说 6–8%。

---

## 验证项 7：嵌套虚拟化的三处口径修正与新增量化（第二批补充）

来自多层分册的第二轮深挖（Intel nVMX 细节 + DPU/性能两条流）。三条是**修正**，其余是**新可引用量化**。

**修正 1（最重要，属常见误传）**："嵌套下 APICv 不可用"**只对 AMD AVIC 成立**。
Intel nVMX 的 L2 **可以**用 APICv（`prepare_vmcs02()` 从 vmcs12 复刻 VID / APIC-register virtualization /
virtualize-x2APIC / posted-interrupt 等控制位）；而 `APICV_INHIBIT_REASON_NESTED` 只被
`arch/x86/kvm/svm/avic.c` 引用。→ 已更正 `02` 稿 §2.2 与正式笔记 §七。

**修正 2**：Turtles 的 **6–8%** 是摘要中**带 DRW 优化的最佳值**；正文未优化值是
**14.5%（kernbench）/ 7.8%（SPECjbb）**。正式笔记已加"引用口径必须区分"的提示框。

**修正 3**：VMCS shadowing **没有独立 CPUID 位**（查 `IA32_VMX_PROCBASED_CTLS2` allowed-1 bit14 与
`IA32_VMX_MISC` bit29）；且**硬件不支持时 KVM 仍向 L1 宣告该能力并自行模拟**
（`nested_vmx_setup_secondary_ctls()` 注释原话："We can emulate VMCS shadowing, even if the hardware doesn't support it"）。
另有一个**独立口径**的数字：NEVE(SOSP'17) §8 在整机负载口径实测 **约 10%** 提升，
与 Turtles 的 84.6%（**退出链成本**口径）并列而不矛盾——量的是不同的东西，引用时须写明口径。

**新增可引用量化（均有出处）**：

| 来源 | 数字 |
|---|---|
| ATC'25 *HyperTurtle* | 每次 L2 exit **≥4 次 world switch**（非嵌套 2 次）；EPT fault 最多 **6 次**；EPT fault 延迟平均 **5.1×**、p99 **5.3×**；Kata 启动 **0.7 s → 1.5 s**（EPT 缺页占比 14% → **46%**）；Nested-VirtIO → Direct-Assignment 时延 −40%、吞吐 +44% |
| NEVE（SOSP'17，ARM） | ARMv8.3 trap-and-emulate：Hypercall **155×/113×**、Virtual IPI **>73×/59×**、Memcached **>40×**；改 NEVE 后微基准最高 5×、Memcached **<3×**。⚠️ **测试平台没有 NV 硬件**，论文自述数字仅用于相对比较 |
| Turtles 细节 | L2 一次 `cpuid` ≈ **58,000 cycles**（单层 ≈2,600、裸机 ≈100，≈单层 22×）；一次 PIO exit 平均引发 **31 次额外 exit**；kernbench 对**裸机** **+25.3%** |
| VMware KB 2009916 | **生产环境不支持嵌套 ESXi**，原文理由 "strict real-time constraints that cannot always be met in a virtualized environment" |
| Xen 官方 wiki | 嵌套的 stress test 与 performance test 均为 **"Not Tested"** → **Xen 从未公布嵌套性能数据**（可作负面证据） |
| KVM 源码 | nVMX 功能子集：**无 L3**、**L2 的 VMCS shadowing 为软件模拟**、**PML 恒模拟**、**无 `#VE`**、**VMFUNC 仅 EPTP switching 且 L1 不能用**、MMIO 快路径不可用 |
| 阿里云官方限制页 | **仅弹性裸金属服务器与超级计算集群支持二次虚拟化**，其他规格族不支持 |
| AWS 白皮书 | hypervisor 现为 "an **optional discrete component**"（裸金属实例的技术前提） |

**仍未证实**：ARMv8.4-NV 的**真实硬件**实测（NEVE 全部数字来自 ARMv8.0 上的 paravirt 模拟）；
Xen / Azure / GCP 的嵌套性能；Google Titanium 的数字；EPT A/D 位与 MBEC 的引入代际。

---

## 参考资料

| 来源 | URL | 访问日期 | 可靠性 |
|---|---|---|---|
| Barham et al., Xen and the Art of Virtualization (SOSP'03) | https://studyres.com/doc/8893844/xen-and-the-art-of-virtualization | 2026-09-16 | 原始论文（镜像站） |
| **HiSilicon 官方 · Kunpeng 920** | https://www.hisilicon.com/en/products/kunpeng/huawei-kunpeng/huawei-kunpeng-920 | 2026-09-16 | **官方一手（Architecture = ARMv8.2）** |
| **鲲鹏社区官方 · 鲲鹏920处理器** | https://www.hikunpeng.com/zh/compute/kunpeng920 | 2026-09-16 | **官方一手（Armv8.2 指令集 + 规格）** |
| KVM arm64 嵌套虚拟化补丁 v11（FEAT_NV2 only，含 Zyngier 放弃 FEAT_NV 原文理由） | http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-November/882814.html | 2026-09-16 | 上游邮件列表 |
| Firecracker 官网（<125ms / 150 microVM/s/host / <5 MiB / 5 个模拟设备） | https://firecracker-microvm.github.io/ | 2026-09-16 | **厂商一手** |
| USENIX 官方 · Turtles 论文摘要页（含 "within 6-8% ... for common workloads" 原文） | https://www.usenix.org/conference/osdi10/turtles-project-design-and-implementation-nested-virtualization | 2026-09-16 | **一手（摘要级）；正文 PDF 无法抓取** |
| KVM arm64 嵌套虚拟化补丁 v10（ARMv8.3/8.4） | http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-May/833892.html | 2026-09-16 | 上游邮件列表 |
| Merge tag 'kvmarm-6.16'（Linux 6.16） | http://git.armlinux.org.uk/cgit/linux.git/log/scripts?id=7f904ff6e58d398c4336f3c19c42b338324451f7&showmsg=1 | 2026-09-16 | 内核 git |
| 华为云社区博客《鲲鹏服务器全栈架构》 | https://bbs.huaweicloud.com/blogs/475584 | 2026-09-16 | ⚠️ 个人博主，含免责声明，仅作线索 |
| 华为云论坛《鲲鹏920支持 KVM嵌套虚拟化吗》 | https://bbs.huaweicloud.cn/forum/thread-0229132917577718008-1-1.html | 2026-09-16 | 抓取为空（JS 渲染），未取得内容 |
| Adams & Agesen, ASPLOS 2006 | https://dl.acm.org/doi/abs/10.1145/1168919.1168860 | 2026-09-16 | 论文存在性确认；原文待取 |
| ASPLOS 2006 论文的课程调查副本（备用路径） | http://cs.uni-salzburg.at/~ck/content/classes/SWS-Summer-2007/mwallner-survey.pdf | 2026-09-16 | 第三方调查；原文数字仍未取得 |
