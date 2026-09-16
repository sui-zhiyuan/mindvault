# 硬件虚拟化依赖硬件的哪些能力 —— Intel x86 (VMX) 与 AMD x86 (SVM)

> 更新：2026-09-16
> 状态：**中间调研稿**（research/ 目录，非 docs/ 正式笔记）。代际与年份已尽量逐条核对，仍存疑处标「⚠ 存在分歧」或「未能证实」。
> 相关笔记：[CubeSandbox Rust 技术栈分析](../../docs/rust-kunpeng/cubesandbox-rust-analysis.md)（KVM/rust-vmm 视角）。
> 同一调研系列：`00-index.md`（索引）、`02-multilayer-virtualization.md`、`04-hw-capabilities-arm-kunpeng.md`、`05-vendors-and-products.md`、`08-verification-notes.md`（高风险项交叉验证）。

## 核心结论

1. **x86 硬件虚拟化的最小可用集合（2005–2006 起点）**：root/non-root 两种运行模式 + 一次 VM transition（VM entry/exit）+ 一块由硬件自动保存/恢复 vCPU 状态的区域（Intel VMCS / AMD VMCB）+ 一组「什么事件该陷入 hypervisor」的控制位（intercept / execution control）。**没有这四件套，x86 只能走二进制翻译或半虚拟化（paravirtualization）**。
2. **内存虚拟化是最大的性能分水岭**：Intel EPT（Nehalem，2008）与 AMD NPT（Barcelona / K10，2007）把二级地址翻译交给硬件（GPA→HPA），使 guest 页表直接由硬件走。缺失时只能用 **shadow page table**，hypervisor 必须为每个 guest 页表写入建影子页并同步，代价通常为 **数倍到 10 倍以上** 的页表密集负载开销（见 §3）。
3. **VPID（Nehalem，2008）与 ASID（SVM 原生）** 用标签区分不同 VM 的 TLB 项。缺失时每次 VM 切换必须 flush 整个 TLB，**客户机上下文切换 / 世界切换的固定成本可翻倍甚至更多**。
4. **中断虚拟化的演进链**：TPR shadow（Nehalem，2008）→ Virtual-APIC + APICv（Haswell，2013：APIC-register virtualization、virtual-interrupt delivery、posted interrupt）→ IPI virtualization（Sapphire Rapids，2023）。缺失时 guest 访问 APIC 页、写 TPR、EOI、发 IPI 全部要 VM-exit 由软件模拟。AMD 侧对应 AVIC（Zen 1 / 2017）与 x2AVIC（Zen 4 / 2022）。
5. **Unrestricted Guest（Intel，Westmere 2010）解决「关闭分页的 guest」**：没有它，运行在实模式或 CR0.PG=0 的 guest 无法被硬件直接承载，VMM 必须用「模拟 8086 + 分页陷阱」等特殊手段。AMD 侧该能力与 NPT 绑定（NPT 使能后 guest 关闭分页即可运行）。
6. **nested virtualization 的硬件辅助**：VMCS shadowing（Haswell，2013）与 VMFUNC/EPTP-switching（Haswell，2013）让 L1 hypervisor 访问自己的 VMCS、在多个 EPT 视图间零 exit 切换。缺失时 L0 必须对 L1 的每次 VMREAD/VMWRITE 做 VM-exit + 软件模拟 —— 嵌套两级的开销从「接近单级」恶化到「数量级放大」。
7. **设备侧必须靠 IOMMU 才能安全直通**：Intel VT-d 与 AMD-Vi 提供 DMA remapping + 中断重映射。**没有 IOMMU，把物理设备直通给 guest 就等于让 guest 拥有任意 DMA 能力，可直接改写宿主内存 —— 不是性能问题而是隔离被彻底击穿。**
8. **机密计算是独立分支**：Intel TDX（Sapphire Rapids，4th Gen Xeon，2023-01 正式 GA）把 guest 做成 TD，由 **TDX Module**（在 SEAM 模式运行的独立固件模块）负责 TD 生命周期与私有内存管理；AMD SEV（Zen 1，2016/2017）→ SEV-ES（Zen 1 后期 / 2017）→ SEV-SNP（Zen 3 / Milan，2021）用内存加密 + 反向映射表（RMP）提供认证与完整性。
9. **代际增量速查**：Nehalem 2008（EPT、VPID、TPR shadow、Virtualize APIC accesses、x2APIC 虚拟化）→ Westmere 2010（Unrestricted Guest）→ Haswell 2013（APICv、VMCS shadowing、VMFUNC、INVPCID exiting）→ Skylake 2015（PML、RDSEED exiting）→ Ice Lake 2019（Bus Lock Detection 等）→ Sapphire Rapids 2023（TDX、IPI virtualization、HLAT/VT-rp 在客户端 12 代起出现）。
10. **一个反直觉点**：硬件能力是**分层的、可独立缺失的**。例如独有 EPT 但没有 Unrestricted Guest（Nehalem）时，guest 一旦关闭分页就会触发 VM-exit；只有 EPT 没有 A/D 位时，dirty logging 必须由 VMM 自己写保护追踪。VMM 必须对每项能力写回退分支，这也是 KVM 里大量 `if (enable_ept) ... else ...` 的根源。

---

## Intel VMX

### 1. root / non-root 模式与 VMX transition

| 项 | 内容 |
|---|---|
| 首次引入 | **2005–2006**：Intel Pentium 4 后期（Prescott 6xx / Pentium D 9xx）与 Core 2 时代开始出现 VT-x；至 Nehalem（2008-11）成为服务器标配。⚠ 具体首发 SKU 各方记载不一。 |
| 硬件做了什么 | 引入 **VMX root operation** 与 **VMX non-root operation** 两种运行模式；执行 `VMXON` 进入 root，`VMLAUNCH`/`VMRESUME` 触发 **VM entry**，guest 中满足控制条件的事件触发 **VM exit** 回到 root。 |
| 缺失时回退 | 只能使用：① **二进制翻译（BT）**，如早期 VMware 的动态翻译；② **半虚拟化**，如 Xen 的 PV 模式，需修改 guest 内核；③ 纯模拟（QEMU TCG）。代价：BT 的翻译缓存维护与自修改代码探测开销大；PV 需改 guest，无法运行未修改的商业 OS。 |

### 2. VMCS 结构与 VMCS shadowing

| 项 | 内容 |
|---|---|
| VMCS | **VMCS（Virtual Machine Control Structure）** 是 VM entry/exit 时硬件自动读写的状态区：guest-state area、host-state area、VM-execution controls、VM-exit/VM-entry controls、VM-exit information fields。由 `VMPTRLD`/`VMREAD`/`VMWRITE`/`VMCLEAR` 访问，结构对软件不透明（仅 revision_id/abort 可见）。**引入：与 VMX 同期（2005–2006）。** |
| VMCS shadowing | 允许 guest（L1 hypervisor）直接执行 VMREAD/VMWRITE 访问「影子 VMCS」而不产生 VM-exit，由硬件按 **VMCS link pointer** 找到真实 VMCS。**首次引入：Haswell（2013）**，对应二级执行控制位 `VMX_SECONDARY_EXEC_SHADOW_VMCS`。 |
| 缺失时回退 | L0 必须对 L1 的每次 VMREAD/VMWRITE 做 VM-exit 并在软件中模拟（KVM 中即 `handle_vmread`/`handle_vmwrite` 路径）。代价：嵌套虚拟化下 VMCS 访问极频繁，**每次访问一次 VM-exit（约千级 cycle）**，嵌套负载可退化数倍。 |
| 核对来源 | QEMU `target/i386/cpu.c` v9.0.0：Haswell 模型首次在 `FEAT_VMX_SECONDARY_CTLS` 中加入 `VMX_SECONDARY_EXEC_SHADOW_VMCS`；Linux `asm/vmxfeatures.h` 位 `VMX_FEATURE_SHADOW_VMCS (2*32+14)`。 |

### 3. VM-exit 原因与信息字段

| 项 | 内容 |
|---|---|
| 内容 | 32 位 **VM-exit reason**（基本原因）+ **exit qualification**、**VM-exit interruption information**、**VM-exit instruction length/information**（如 EPT violation 的 GPA、INVEPT/INVVPID 类型、MSR 访问地址等）。**引入：与 VMX 同期。** |
| 作用 | 让 VMM 一次 exit 就能判定「为什么退出」并直接拿到所需上下文，避免读 guest 内存反推。 |
| 缺失时回退 | 无此硬件字段时，VMM 必须**解释导致退出的指令**（指令解码器），或依赖半虚拟化 hypercall 显式告知原因。代价：每次 exit 多一次指令解码，路径长、易错，且对自修改/多字节指令边界处理困难。 |
| 补充 | 后续增补了大量原因码与信息位：`INVPCID`/`INVVPID`/`INVEPT` 的 type、`Bus Lock`（Ice Lake+）、`Notify VM-exit`（Sapphire Rapids 一代）等。 |

### 4. MSR bitmap / I/O bitmap / CR 访问掩码

| 项 | 内容 |
|---|---|
| MSR bitmap | 4KB 位图（`USE_MSR_BITMAPS`，基本执行控制），按「读/写」两位控制各 MSR 是否 exit。**引入：VMX 早期（Nehalem 前已存在，2005–2006 级）。** |
| I/O bitmap | 两个 4KB 位图覆盖 64K I/O 端口空间（`USE_IO_BITMAPS`，或 `UNCOND_IO_EXITING` 全陷入）。**引入：VMX 早期。** |
| CR 访问掩码 | `CR0/CR4 guest/host mask` + `CR0/CR4 read shadow`；`CR3-load/store exiting`、`CR8-load/store exiting`（CR8 即 TPR）。**CR0/CR4 掩码与 CR3 exiting：VMX 早期；CR8 exiting 与 TPR shadow 同期出现（2008 前后）。** ⚠ QEMU 的 Nehalem 模型已带 `CR8_LOAD/STORE_EXITING` 与 `TPR_SHADOW`，故 2008 年已可用。 |
| 作用 | 精确选择「哪些敏感指令/寄存器访问需要陷入」，把不需要陷入的路径留给硬件直通，是 VMX 性能调优的核心旋钮。 |
| 缺失时回退 | 只能对整类指令无条件陷入（例如所有 I/O 指令、所有 CR 写入、所有 MSR 访问）。代价：guest 启动、驱动初始化、频繁 CR3 切换（进程切换）等路径的 VM-exit 数量可上升 **1~2 个数量级**；I/O 密集负载尤其明显。 |

### 5. exception bitmap（异常位图）

| 项 | 内容 |
|---|---|
| 内容 | 32 位位图，`#DE`~`#AC` 每个异常向量一位；置位则该异常在 guest 中触发 VM-exit，否则由 guest 的 IDT 正常处理。另有 `page-fault error code mask/match` 可只对特定错误码组合陷入。 |
| 首次引入 | **与 VMX 同期（2005–2006）**；`page-fault error code mask/match` 同为早期 VMX 能力（QEMU 的 Nehalem 模型已具 `EXCEPTION_BITMAP`）。 |
| 作用 | 让 `#PF`（用于 MMIO 模拟/脏页追踪）、`#GP`、`#UD` 等少数异常陷入，其余异常（如 `#DE`、`#BP`、浮点异常）零开销直通 guest。 |
| 缺失时回退 | 要么所有异常都陷入（每次除零都 VM-exit），要么全不陷入（无法做 MMIO 与脏页追踪）。代价：全陷入时异常密集型负载（JIT、GC、浮点边界）开销显著；无法按错误码区分时，#PF 的处理必须在软件里再判断一次，且难以区分「guest 自己的缺页」与「需 VMM 介入的缺页」。 |

### 6. Unrestricted Guest（无限制客户机）

| 项 | 内容 |
|---|---|
| 首次引入 | **Westmere（2010）**（Intel 客户端 2010-01 首发；服务器对应 Westmere-EP 2010-03）。对应二级执行控制位 `VMX_SECONDARY_EXEC_UNRESTRICTED_GUEST`。 |
| 硬件做了什么 | 允许 guest 在 **CR0.PG = 0（未开启分页）** 或 **实模式** 下直接运行，且必须与 EPT 配合使用。 |
| 缺失时回退 | Nehalem 及更早硬件上，vCPU 一旦关闭分页（BIOS/引导阶段、DOS、某些 firmware 路径）就不能直接执行：VMM 需 ① 维持「分页始终打开 + 用 EPT 把线性地址等同物理地址」的技巧；② 或走 **8086 模式模拟器**（QEMU/KVM 对 `real mode` + `CR0.PG=0` 的 guest 会失败并报错，只能靠三重故障/模拟兜底）。③ 早期 VMware/Xen 用「VM86 + 陷阱」模拟。代价：引导阶段路径极慢（每条指令都可能 exit），且部分 guest 根本无法启动（KVM 在无 Unrestricted Guest 时对实模式 guest 支持受限）。 |
| 核对 | QEMU v9.0.0：`Westmere` 模型 `FEAT_VMX_SECONDARY_CTLS` 首次出现 `VMX_SECONDARY_EXEC_UNRESTRICTED_GUEST`（Nehalem 模型无）。 |

### 7. EPT（Extended Page Tables）

| 子项 | 首次引入 | 硬件做什么 / 缺失代价 |
|---|---|---|
| **EPT 本体**（GPA→HPA 二级翻译，`EPT_POINTER` 指向 EP4TA，4 级页表；`INVEPT` 失效） | **Nehalem（2008-11）** | 让 guest 自己的页表直接被硬件走，无需 VMM 维护影子页。缺失 → shadow page table（见 §3），代价通常 **数倍** 于 EPT，页表密集负载（fork/exec、数据库、编译）最高可达 **10× 以上**。 |
| **EPT violation**（`#VE` 前置的 VM-exit 原因 48，exit qualification 给出「读/写/执行 + GPA + 是否因 guest 线性地址翻译失败」） | **Nehalem（2008）** | 让 VMM 精确得知是「哪一页、哪种访问」出错 —— MMIO 模拟、写保护、缺页填充都依赖它。缺失时只能靠解释指令 + 反查 guest 页表猜测访问类型，慢且易错。 |
| **EPT 大页**（2MB / 1GB，`MSR_VMX_EPT_2MB/1GB`） | **Nehalem（2008）**（QEMU Nehalem 模型已含 `MSR_VMX_EPT_2MB`、`MSR_VMX_EPT_1GB`） | 用大页减少 EPT 层级与 TLB 压力。缺失时只能 4KB 映射：EPT 页表内存放大、TLB 命中率下降，**内存密集负载常见 10%~30% 回退**（量级估算，非精确基准）。 |
| **EPT A/D 位**（EPT 项中的 Accessed/Dirty 由硬件置位） | **Haswell 一代（2013）** ⚠ 存在分歧：QEMU 的 Haswell 模型**未**列 `MSR_VMX_EPT_AD_BITS`，而 Broadwell 及以后都列。Linux 侧 2017-04 才有 "MMU support for EPT accessed/dirty bits"、2018-08 才加入独立 CPU feature 位。**保守结论：Haswell 起的某些 SKU 具备，确切首发代际未能证实。** | 让硬件直接维护 A/D 位，VMM 无需为脏页追踪写保护 guest 页。缺失时脏页追踪（迁移、快照、`KVM_GET_DIRTY_LOG`）必须走写保护 + `#PF`（shadow 模式那套），迁移迭代轮次与 VM-exit 数量显著上升；替代品是 Haswell+ 的 **PML（Page Modification Logging，`PAGE_MOD_LOGGING`）**，把脏页地址批量写入环形缓冲，进一步减少 exit。 |

### 8. VPID（Virtual Processor ID）

| 项 | 内容 |
|---|---|
| 首次引入 | **Nehalem（2008）**，二级执行控制位 `VMX_SECONDARY_EXEC_ENABLE_VPID`（QEMU Nehalem 模型已有）。 |
| 硬件做了什么 | 给 TLB 项打上 16 位 VPID 标签，vCPU 在不同 VPID 下运行时 TLB 不必清空；`INVVPID` 支持按单个地址/单个上下文/全部上下文失效（含 `SINGLE_CONTEXT_NOGLOBALS`）。 |
| 缺失时回退 | 每次 VM entry/exit 必须 **flush 整个 TLB**（early VMX 硬件确实如此）。代价：客户机进程切换、vCPU 迁移、嵌套 entry/exit 后的 TLB 全冷启动，**每次切换多出数百到数千 cycle 的 TLB miss 惩罚**；syscall/上下文切换密集负载可退化 10%~50%，CPU 密集长驻负载影响较小。 |

### 9. Virtual-APIC / APICv（TPR shadow、EOI 加速、posted interrupt）

| 子项 | 首次引入 | 硬件做什么 | 缺失代价 |
|---|---|---|---|
| **TPR shadow**（`TPR_THRESHOLD` + `virtual_apic_page`） | **Nehalem 一代（2008）** | 把 guest 的 TPR（CR8）放在 `virtual-APIC page` 里，guest 读写 TPR 不产生 VM-exit（只有越过 TPR threshold 才退出） | 每次 TPR 写入（内核中断开关、`local_irq_save`）都 VM-exit；**中断密集负载开销成倍** |
| **Virtualize APIC accesses** | **Nehalem 一代（2008）**（QEMU Nehalem 已有 `VIRTUALIZE_APIC_ACCESSES`） | guest 访问 APIC MMIO 页被硬件重定向到 virtual-APIC page | 每次 APIC 寄存器读写（EOI、ISR、ICR、LVT）都 VM-exit |
| **Virtualize x2APIC mode** | **Nehalem 一代（2008）** | x2APIC 的 MSR 访问同样被重定向，不陷入 | x2APIC 平台下每次 APIC MSR 访问 exit |
| **APIC-register virtualization**（`APIC_REGISTER_VIRT`） | **Haswell（2013）** | 大部分 APIC 寄存器（IRR/ISR/TMR/LVT/ICR…）读写由硬件在 virtual-APIC page 上完成 | APIC 状态机仍要软件模拟，大量 VM-exit |
| **Virtual-interrupt delivery**（`VIRT_INTR_DELIVERY`） | **Haswell（2013）** | 硬件直接在 guest 中投递虚拟中断，实现 **EOI 加速**（write to EOI 不退出，硬件自动更新 ISR 并注入下一个中断） | 中断投递必须「VM-exit → VMM 判断 → 注入虚拟中断 → VMRESUME」，**每次中断 2 次 transition**；高 PPS/高 IOPS 负载吞吐断崖 |
| **Posted interrupt**（`PIN_BASED_POSTED_INTR`，需 Posted Interrupt Descriptor + `POSTED_INTR_VECTOR`） | **Haswell（2013）** ⚠ QEMU 从 IvyBridge 起标 `VMX_PIN_BASED_POSTED_INTR`；Haswell 及以后确定具备。**保守：Haswell（2013）。** | 物理中断可直接投递给正在运行的 vCPU，或写 **Posted Interrupt Descriptor** 唤醒被阻塞的 vCPU，**无需 host 唤醒 + IPI + VM entry** | 中断必须先唤醒 host 线程、再由 host IPI 唤醒 vCPU；**halted vCPU 的中断投递延迟增加，虚拟化密度高时明显** |
| **IPI virtualization** | **Sapphire Rapids（2022-2023）**（Linux 5.18，2022 年合入） | guest 之间的 IPI 由硬件直接投递，不产生 VM-exit | IPI 密集（多 vCPU 同步、spinlock、TLB shootdown）负载大量 exit |
| **PI 描述符**（Posted Interrupt Descriptor, PID） | 与 posted interrupt 同期 | 每 vCPU 一个 64B 结构，记录 PIR（pending interrupt request）与通知向量 | 无 posted interrupt 即无此结构 |

### 10. VMFUNC（VM Functions：EPTP switching / 多 EPTP）

| 项 | 内容 |
|---|---|
| 首次引入 | **Haswell（2013）**，二级执行控制 `VMX_SECONDARY_EXEC_ENABLE_VMFUNC` + `MSR_VMX_VMFUNC_EPT_SWITCHING`。 |
| 硬件做了什么 | guest 在 non-root 下执行 `VMFUNC` 指令（leaf 0 = **EPTP switching**），可在 **最多 512 个预登记的 EPTP** 之间零 VM-exit 切换，从而瞬时改变「GPA→HPA」的整个地址翻译视图。 |
| 缺失时回退 | 要换地址视图只能 VM-exit 后由 VMM 调 `VMPTRLD`/改 EPTP 再 entry。代价：**每次视图切换 2 次 VM transition（约数千 cycle）**；EPTP switching 是 unikernel/多视图（如内存内省、地址空间隔离）方案的核心前提，缺失则设计上直接不可行。 |

### 11. Mode-Based Execute Control（MBEC）

| 项 | 内容 |
|---|---|
| 首次引入 | **Kaby Lake（2017）** —— 与 CET 生态同批引入的 VMX 能力 ⚠ 具体首发代际在公开资料中表述不一（Intel 客户端 7 代 Core 起），**建议标注「未能完全证实」**。Linux 中对应 `SECONDARY_EXEC_MODE_BASED_EPT_EXEC`（`VMX_FEATURE_MODE_BASED_EPT_EXEC (2*32+22)`，`ept_mode_based_exec`）。 |
| 硬件做了什么 | 让 EPT 的执行权限**按特权级分离**：超级用户态与用户态各有独立的 execute 位，从而在硬件层面强制 **SMEP/SMAP 的虚拟化**（guest 内核不能执行/读写用户页，由硬件拦截）。 |
| 缺失时回退 | 只能靠 guest 自己的 SMEP/SMAP 位 + VMM 用 EPT 整体写保护/去执行权来近似，或者对 guest CR4 写入做陷入并在软件中模拟。代价：安全边界变弱（VMM 无法独立于 guest 状态强制 W^X 分离），且模拟 CR4 写入增加 exit。 |

### 12. HLAT（Hypervisor-Managed Linear Address Translation，Intel VT-rp）

| 项 | 内容 |
|---|---|
| 首次引入 | **Intel 12 代 Core（Alder Lake，2021）起**在客户端平台出现；Intel Raptor Lake（13 代，2022）及以后的数据手册中明确列出 HLAT（与 VT 技术族并列）：[Intel Raptor Lake 数据手册 · Hypervisor-Managed Linear Address Translation](https://edc.intel.com/content/www/us/en/design/products/platforms/details/raptor-lake-s/13th-generation-core-processors-datasheet-volume-1-of-2/003/015/hypervisor-managed-linear-address-translation/)。**服务器首发 SKU 未能证实**；Intel 12 代数据手册「Supported Technologies」亦列出该技术。 |
| 硬件做了什么 | VT-rp（redirect protection）三件套之一：**HLAT** 让 hypervisor 提供一套「root page table」，guest 在 non-root 下的线性地址翻译可使用 HLAT paging 结构，从而把 guest 的可执行/可访问视图与 EPT 视图解耦（配合 PW/保护写、GPV/Guest Paging Verification，用于抵抗 remapping 攻击）。 |
| 缺失时回退 | 只能靠 EPT 权限位 + 影子页表组合近似。代价：无法在硬件层阻止「hypervisor 重映射页面劫持 guest 控制流」这类攻击（VT-rp 的核心目标），需要额外的软件方案（如受保护的内核完整性框架）。 |
| 补充 | 公开的演示性实现见 [Hello-VT-rp](https://github.com/tandasat/Hello-VT-rp)（作者 tandasat），其 README 说明该 hypervisor 可在 Dell Latitude 7330 上启用 HLAT、PW、GPV。 |

### 13. VT-d（IOMMU：DMA remapping、中断重映射）

| 项 | 内容 |
|---|---|
| 首次引入 | **Nehalem 平台（2008）**——VT-d 随 Nehalem 一代芯片组/CPU 引入；更早的 2005–2007 已有 VT-d 规范（Intel 2006 年发布 IOVM/VT-d 论文与规范），真正产品化随 Nehalem。⚠ 具体「首个支持 VT-d 的芯片组」记载不一。 |
| 硬件做了什么 | ① **DMA remapping**：设备发出的 DMA 地址（含 PCIe 请求者 ID）经 IOMMU 翻译并检查权限；② **中断重映射（Interrupt Remapping）**：MSI/MSI-X 与 IOAPIC 中断经中断重映射表校验并投递到指定 vCPU，防止伪造中断；③ 支持 **PCIe ATS/PRI、SVM（Shared Virtual Memory）** 等扩展。 |
| 缺失时回退 | 只能：① 完全 **设备模拟**（QEMU 模拟网卡/磁盘，virtio 半虚拟化是性能最优解）；② 依赖 VMM 的「DMA 陷阱」（如某些平台的 `INTEL_IOMMU` 关闭时 VMware 的 DMA remapping 模块）。**没有 IOMMU 就绝不能安全直通**：PCIe 设备可对任意物理地址做 DMA，guest 驱动即可改写宿主内核内存 —— 这是隔离性失效，不是性能问题。 |

### 14. SR-IOV 与 PCIe ACS

| 项 | 内容 |
|---|---|
| SR-IOV | 单物理功能（PF）虚拟出多个虚拟功能（VF），每个 VF 可独立分配给一个 guest，配合 VT-d/AMD-Vi 做每 VF 的地址翻译。**PCI-SIG 规范 2007 年发布；产品化随 Nehalem 平台（2008–2009）的 Intel 82576 等网卡。** |
| PCIe ACS（Access Control Services） | PCIe 拓扑中阻止设备间的「对等（peer-to-peer）」流量绕过 IOMMU，是「把 SR-IOV VF 安全分配给不同 guest」的前提。**ACS 能力随 PCIe 2.0/2.1 规范（2007–2008）引入。** |
| 缺失时回退 | 无 ACS 时，同一 PCIe switch 下的 P2P DMA 可能绕过 IOMMU 隔离，只能通过 `pci=noacs`/`disable_acs_redir` 之类的规避手段或干脆不直通该拓扑下的设备；无 SR-IOV 时只能整卡直通或走 virtio/vhost。代价：整卡直通牺牲共享密度；virtio 损失部分吞吐并消耗宿主 CPU。 |

### 15. SGX

| 项 | 内容 |
|---|---|
| 首次引入 | **Skylake 客户端（2015）**（SGX1）；SGX2（动态内存管理）随 **Kaby Lake / Skylake-SP（2017）**。 |
| 硬件做了什么 | 在用户态划出 **enclave** 受保护地址区间，内存由 **EPC（Enclave Page Cache）** 承载并以 MEE（内存加密引擎）加密，CPU 只在 enclave 上下文内可访问；远程认证（attestation）证明 enclave 身份。 |
| 与虚拟化的关系 | SGX 与 VMX 正交：enclave 不能在 guest 中直接使用（早期 EPC 不可虚拟化），VMM 只能选择「暴露 SGX 给专门 VM」或屏蔽；KVM 侧有 `SGX` 虚拟化的长期讨论但支持有限。 |
| 缺失时回退 | 只能用「机密 VM + 内存加密」路线（TDX/SEV）或进程级 TEE（如 Intel TDX 之前的 SGX SDK 应用）。代价：SGX 的 128MB 级 EPC 限制与频繁换页（EWB/ELDU）本身开销大，且无法承载整机工作负载。**注：Intel 已在 11–14 代部分消费级 SKU 上移除 SGX 支持，具体 SKU 列表未能逐一证实。** |

### 16. TDX（Trust Domain Extensions）与 TDX Module

| 项 | 内容 |
|---|---|
| 首次引入 | **Intel 4th Gen Xeon Scalable（Sapphire Rapids），2023-01-10 正式商用发布**；TDX 1.0 随该平台 GA。Intel 官方按 Xeon 代际列出 TDX readiness（见参考资料）。 |
| 硬件做了什么 | 引入 **SEAM（Secure Arbitration Mode）** 与 **SEAMCALL/SEAMRET/TDCALL** 指令：guest 变成 **TD（Trust Domain）**，其私有内存由 CPU 用 **TDX 密钥**加密（受 TME-MK 支持），host/VMM 不能读取或篡改；支持 TD 的 vCPU 状态保护与远程认证。 |
| **TDX Module 的定位** | TDX Module 是 **Intel 官方发布的、运行在 SEAM 模式下的独立固件模块**（由 CPU 以特殊方式加载、经 Intel 签名），**不是 VMM 的一部分**。它实现 TD 生命周期（TDH.MNG.*/TDH.VP.* 等接口）、私有内存管理、EPT/页表校验与 TD 退出处理。VMM 只通过 **TDCALL/SEAMCALL ABI** 与它交互 —— 因此 TDX 的信任根从「VMM 正确」上移到「CPU + TDX Module 正确」。 |
| 缺失时回退 | 无 TDX 时，机密 VM 只能靠 AMD SEV 系列或纯软件方案；Intel 平台上既无 SGX（整机级）也无 TDX 时，唯一选择是「加密内存 + 加固 VMM」，无法把 VMM 排除出信任边界。代价：无法抵御恶意/被攻陷的 hypervisor。 |

#### Intel VMX 能力代际速查

| 能力 | 引入代际 | 年份 | 依据（摘要） |
|---|---|---|---|
| VMX root/non-root、VM entry/exit、VMCS、bitmap、exception bitmap、VM-exit 信息字段 | Pentium 4 后期 / Core 2 | 2005–2006 | Intel SDM Vol 3C 附录 A；QEMU 最早 VMX 模型（`qemu64`）即含这些控制位 |
| EPT、VPID、TPR shadow、Virtualize APIC accesses、x2APIC 虚拟化、CR8/CR3 exiting | **Nehalem** | 2008 | QEMU v9.0.0 `Nehalem` 模型 |
| Unrestricted Guest | **Westmere** | 2010 | QEMU `Westmere` 模型新增 |
| APICv（APIC-register virtualization、virtual-interrupt delivery）、posted interrupt、VMCS shadowing、VMFUNC/EPTP switching、INVPCID exiting | **Haswell** | 2013 | QEMU `Haswell` 模型；⚠ posted interrupt QEMU 自 IvyBridge 标注，保守取 Haswell |
| EPT A/D 位 | Haswell / Broadwell ⚠ | 2013–2014 | QEMU 自 Broadwell 起标注；确切首发**未能证实** |
| PML（Page Modification Logging）、RDSEED exiting | **Skylake 一代** | 2015 | QEMU `Skylake-Server` 模型 |
| MBEC（Mode-Based Execute Control） | Kaby Lake ⚠ | 2017 | Linux `ept_mode_based_exec`；客户端 7 代起，**未完全证实** |
| Bus Lock Detection | Ice Lake 一代 | 2019 | Linux `VMX_FEATURE_BUS_LOCK_DETECTION`；QEMU 未在 IceLake-Server 列出，⚠ |
| HLAT / VT-rp | Alder Lake（客户端） | 2021 | Intel Raptor Lake 数据手册列出 HLAT |
| IPI virtualization、Notify VM-exit | Sapphire Rapids | 2022–2023 | Linux 5.18 KVM 合入描述；`VMX_FEATURE_NOTIFY_VM_EXITING` |
| TDX（SEAM + TDX Module） | Sapphire Rapids（4th Gen Xeon） | 2023 | Intel 官方 TDX readiness 页 + 发布报道 |
| VT-d（IOMMU） | Nehalem 平台 | 2008 | Intel VT-d 规范与 IOVM 论文（2006）→ Nehalem 产品化 |
| SGX | Skylake 客户端 | 2015 | SGX1；SGX2 随 Kaby Lake / Skylake-SP 2017 |

---

## AMD SVM

### 1. VMCB 结构与 intercept 分类

| 项 | 内容 |
|---|---|
| 首次引入 | **AMD SVM（Secure Virtual Machine）随 AMD-V 于 2006 年（Socket AM2 / Rev. F）推出**；`VMRUN`/`VMLOAD`/`VMSAVE`/`VMMCALL`/`STGI`/`CLGI`/`SKINIT`/`INVLPGA` 为其指令集。 |
| 硬件做了什么 | **VMCB（Virtual Machine Control Block）** 是 SVM 的「一块式」控制结构，物理上连续、4KB 对齐，分 **control area（1024 字节）** 与 **save area（744 字节，`struct vmcb_save_area`）**。Linux 侧有编译期断言：`EXPECTED_VMCB_SAVE_AREA_SIZE 744`、`EXPECTED_VMCB_CONTROL_AREA_SIZE 1024`（`arch/x86/include/asm/svm.h`）。`VMRUN` 以 VMCB 物理地址为参数，一条指令完成「保存 host 状态 + 载入 guest 状态 + 进入 guest」。 |
| intercept 分类（16 个 32 位 intercept word） | 由 Linux `svm.h` 枚举给出：**CR intercept**（CR0/CR3/CR4/CR8 读与写，独立位）、**DR intercept**（DR0–DR7 读写）、**exception intercept**（向量 0–31 各一位，`INTERCEPT_EXCEPTION_OFFSET = 64`）、**INTR/NMI/SMI/INIT/VINTR intercept**（word 3）、**instruction intercept**（word 4：`RDTSC`/`RDPMC`/`PUSHF`/`POPF`/`CPUID`/`RSM`/`IRET`/`INTn`/`INVD`/`PAUSE`/`HLT`/`INVLPG`/`INVLPGA`…）、**IO intercept**（`IOIO_PROT`，配 IOPM 位图）、**MSR intercept**（`MSR_PROT`，配 MSRPM 位图）。 |
| 与 Intel 的差异 | Intel 用 VMCS 的多个独立字段 + 独立 bitmap 指针（MSR/I/O 位图），AMD 用**固定偏移的 intercept 位** + IOPM/MSRPM 两个位图基址。功能等价，布局哲学不同：**VMCB 是内存中的固定结构，`VMSAVE`/`VMLOAD` 可把它与 guest 内存交换**，这使 AMD 的「VMCB 换出」比 Intel 的 VMCS 更直接。 |
| 缺失时回退 | 与 Intel 同理：无 intercept 机制就只能二进制翻译/半虚拟化。 |

### 2. host / guest 模式

| 项 | 内容 |
|---|---|
| 内容 | SVM 定义 **host mode**（`EFER.SVME=1`，可执行 `VMRUN`）与 **guest mode**；`STGI`/`CLGI` 控制 **GIF（Global Interrupt Flag）**，屏蔽/开放物理中断。**引入：AMD-V 首发（2006）。** |
| 缺失时回退 | 无 GIF 时，进入 guest 前必须手工关闭中断（`CLI`）并处理不可屏蔽中断的竞态，VM entry 窗口内的中断会破坏 guest 状态。代价：实现复杂、易出竞态 bug，且 host 中断延迟增加。 |

### 3. ASID（Address Space Identifier）

| 项 | 内容 |
|---|---|
| 首次引入 | **AMD-V 首发（2006）**。 |
| 硬件做了什么 | 每个 guest（或每套页表上下文）分配一个 ASID，TLB 项按 ASID 打标签；`VMCB.TLB_CONTROL` 支持 `TLB_CONTROL_FLUSH_ALL_ASID`、`FLUSH_ASID`、`FLUSH_ASID_LOCAL`（`svm.h`）—— 即**可选择只 flush 一个 ASID 而非整个 TLB**。 |
| 缺失时回退 | 每次 VM 切换 flush 整个 TLB。代价：与 Intel 无 VPID 时相同量级 —— **每次切换数百~数千 cycle 的 TLB 重填**；同时 ASID 还是 **SEV 内存加密密钥的关联键**（KVM 文档明确：SEV guest 的 ASID 必须落在 1..CPUID 0x8000001f[ecx] 范围内），因此无 ASID 也意味着 SEV 无法实现。 |

### 4. NPT（Nested Page Table）

| 项 | 内容 |
|---|---|
| 首次引入 | **AMD 家族 10h「Barcelona」（K10），2007 年发布**（KVM 侧 AMD nested paging 支持补丁出现在 2008-01，对象为 Family 16h 的 nested paging 实现）。AMD 的营销名为 **RVI（Rapid Virtualization Indexing）**。 |
| 硬件做了什么 | 与 Intel EPT 等价：guest 页表（NCR3）之上再走一层 **NPT（nested page table）**，把 GPA 翻译为 HPA；`VMCB` 中的 `NP_ENABLE`、`N_CR3`、`NPT_ERR` 等字段控制；NPF（Nested Page Fault）通过 `EXITINFO1/EXITINFO2` 报出访问类型与 GPA。 |
| 缺失时回退 | 只能用 **shadow page table**（KVM 的 `mmu.c` shadow MMU：`kvm_mmu_get_page`、写保护同步、`rmap` 反向映射、unsync page 优化）。代价见 §3，通常 **数倍**；KVM 文档 [The x86 kvm shadow mmu](https://www.kernel.org/doc/html/latest/virt/kvm/x86/mmu.html) 描述了维持影子页一致性所需的全部机制，本身就是复杂度的直接证据。 |
| 附注 | 与 Intel 一样，**NPT 使能后 guest 可以关闭分页**（CR0.PG=0 / 实模式）运行，因此 AMD 侧并不存在 Intel 那样的「独立 Unrestricted Guest 能力缺口」。 |

### 5. AVIC（Advanced Virtual Interrupt Controller）

| 项 | 内容 |
|---|---|
| 首次引入 | **Zen 1（2017）**；Linux KVM 的 AVIC 支持补丁最早可追溯到 2016-05（`svm: Introduce new AVIC VMCB registers` 等，2016-05-18），最终随硬件（Zen / EPYC Naples）落地。 |
| 硬件做了什么 | ① **物理 APIC 模式**：每个 vCPU 在 `AVIC_PHYSICAL_ID_TABLE` 中有表项，指向其 backing page 与 host 物理 APIC ID，guest 直接读写虚拟 APIC，硬件投递中断不 exit；② **逻辑 APIC 模式**：`AVIC_LOGICAL_ID_TABLE` 支持逻辑目标（cluster）IPI；③ IPI 可由硬件直接在 vCPU 间投递；④ 访问未加速寄存器时通过 `AVIC_UNACCEL_ACCESS` 报给 VMM。 |
| x2APIC 限制 | **x2AVIC（x2APIC virtualization）在 Zen 4 / EPYC 9004（Genoa，2022）才引入**，Linux 支持随 **Linux 6.0（2022-10）** 合入（Phoronix 报道）。硬件限制见于 `svm.h`：传统 AVIC 的物理 APIC ID 上限 `AVIC_MAX_PHYSICAL_ID = 0xFE`（255），**x2AVIC 提升到 `X2AVIC_MAX_PHYSICAL_ID = 0x1FF`（512）**；且开启 AVIC 时早期 KVM 明确 **不向 guest 暴露 x2APIC**（2016 年补丁 "Do not expose x2APIC when enable AVIC"）。 |
| 缺失时回退 | 中断全部走 VM-exit 软件模拟（KVM 的 APIC 访问模拟 + `kvm_lapic` 注入）。代价：与 Intel 无 APICv 时相同量级 —— **每次 APIC 访问、每次 EOI、每次 IPI 各一次 VM-exit**，中断密集负载吞吐显著下降。 |

### 6. VGIF（Virtual GIF）

| 项 | 内容 |
|---|---|
| 首次引入 | Linux 侧 2017-08-23 补丁（`KVM: SVM: Add Virtual GIF feature definition` / `Enable Virtual GIF feature`，随 Linux 4.14）。硬件世代对应 **Zen 2（2019）/ 更早的 Zen 修订** ⚠ 具体首发 SKU 未能证实（AMD 未在公开资料中明确标注，通常记为 Zen 2 起）。 |
| 硬件做了什么 | 在 VMCB 中提供 **虚拟 GIF（V_GIF）**：guest 内的 `STGI`/`CLGI` 直接修改虚拟 GIF（`V_GIF_MASK`、`V_GIF_ENABLE_MASK`，`svm.h`），**不再产生 VM-exit**，只有在 GIF 为 0 时才陷入。 |
| 缺失时回退 | 每次 guest 执行 `STGI`/`CLGI` 都要 VM-exit 并由 VMM 模拟。代价：guest 内核在每次进入/退出临界区时成对执行 STGI/CLGI，**频繁中断开关的内核（尤其 Windows guest、实时负载）开销明显**，每次 exit 约千级 cycle。 |

### 7. VMSAVE / VMLOAD

| 项 | 内容 |
|---|---|
| 内容 | `VMLOAD`/`VMSAVE` 在 guest 与 host 之间搬运一组额外状态（FS/GS/TR/LDTR 基址与 limit、sysenter/syscall MSR 等），**由硬件完成而非软件逐条读写**。**引入：AMD-V 首发（2006）。** 后续增加 **Virtual VMLOAD/VMSAVE**（2017-07 Linux 补丁 `KVM: SVM: Enable Virtual VMLOAD VMSAVE feature`），让 guest 内使用这两条指令不 exit。 |
| 缺失时回退 | VMM 必须手工逐字段保存/恢复这些寄存器，**每次上下文切换数十条 MSR/寄存器读写**；或者在 guest 内 trap 这两条指令并模拟。代价：切换成本线性增加，且极易漏字段导致状态污染。 |
| 对照 Intel | Intel 的对应机制是 VMCS 的 host-state / guest-state area 自动加载 + `VM-entry/VM-exit MSR load/store lists`；功能等价，AMD 额外提供可直接调用的 `VMSAVE`/`VMLOAD` 指令做「VMCB 换出」。 |

### 8. GMET（Guest Mode Execute Trap）

| 项 | 内容 |
|---|---|
| 首次引入 | **AMD「未能证实」确切首发世代。** KVM 侧的 `GMET` 位定义出现在 2026-05（`KVM: SVM: add GMET bit definitions`，以及 `kvm-mbec` 分支合并），同期 QEMU 提交「add new AMD EPYC models for GMET enablement」（2026-03/04）。**公开资料中 GMET 与 SEV-SNP / Zen 4+ 常被并列提及，但本次检索未能找到 AMD 官方 APM 中的引入代际说明。** |
| 硬件做了什么（按现有资料） | 类比 Intel MBEC：在 guest 页表/NPT 中允许**按特权级区分执行权限**，即 guest 内核态与用户态使用不同的 execute 判定，从而在硬件层强制 SMEP/SMAP 语义（`PFEC.I/D` 相关）。 |
| 缺失时回退 | 与 Intel MBEC 相同：只能靠 guest 自身 SMEP/SMAP + VMM 的权限近似，安全边界较弱。 |
| 结论 | **标注为「未能证实世代」，需后续以 AMD APM Volume 2 / PPR 核对。** |

### 9. SEV / SEV-ES / SEV-SNP

| 子项 | 首次引入 | 硬件做了什么 | 缺失代价 |
|---|---|---|---|
| **SEV**（Secure Encrypted Virtualization） | **Zen 1（2017，EPYC Naples）**（Linux KVM 文档与 `cpuid 0x8000001f` 接口同期出现） | guest 内存用**每 VM 独立密钥**透明加密；ASID 与密钥绑定（KVM 文档：SEV guest 的 ASID 必须在 1..CPUID 0x8000001f[ecx]）；VMCB 偏移 0x90 bit1 使能 SEV | 无 SEV 时 hypervisor 可任意读取 guest 全部内存 —— 机密性完全依赖 VMM 可信 |
| **SEV-ES**（Encrypted State） | **Zen 1 后期（2017 宣布，2018 产品化）** ⚠ 具体首个 SKU 未能证实；常与 EPYC 7002（Rome）关联 | 除内存外，**vCPU 寄存器状态也在 VM-exit 时加密**（VMSA，`struct sev_es_save_area`），hypervisor 无法读取 guest 寄存器；需要 **#VC（VMM Communication Exception）** 处理，guest 需改造（Linux 有 `#VC` handler） | 无 SEV-ES 时 VM-exit 会把明文寄存器交给 hypervisor，可推断 guest 密钥/明文 |
| **SEV-SNP**（Secure Nested Paging） | **Zen 3（EPYC 7003 Milan，2021-03）** | 引入 **RMP（Reverse Map Table）** 做**嵌套页表级的完整性认证**：每个物理页记录归属与权限，阻止 hypervisor 重映射/重放攻击；提供 **VMPL（VM Privilege Level）** 支持 guest 内分层；认证报告（attestation）覆盖更完整 | 无 SNP 时，hypervisor 虽读不到明文，但可**篡改 GPA→HPA 映射或重放旧页面**（完整性攻击）。Linux 侧 RMP 支持补丁 2022-04 起（`x86/sev` 系列），随 v5.19 合入 |

### 10. AMD-V 在 Zen 各代的增量

| 世代 | 年份 | 虚拟化相关增量 |
|---|---|---|
| **K8 / AMD-V 首发** | 2006 | SVM 基础：VMCB、host/guest 模式、intercept 分类、ASID、VMSAVE/VMLOAD、TLB_CONTROL |
| **K10 / Barcelona** | 2007 | **NPT（RVI）**，二级地址翻译硬件化 |
| **Zen 1（Naples）** | 2017 | **AVIC**（物理/逻辑 APIC 模式）、**SEV**（内存加密） |
| **Zen 1 后期 / Zen 2（Rome）** | 2018–2019 | **VGIF**（虚拟 GIF）、**Virtual VMLOAD/VMSAVE**、**SEV-ES** 产品化 |
| **Zen 3（Milan）** | 2021 | **SEV-SNP + RMP + VMPL** |
| **Zen 4（Genoa）** | 2022 | **x2AVIC**（x2APIC 虚拟化，Linux 6.0 支持）、更大的 vCPU/APIC ID 空间（至 511） |
| **更新（GMET 等）** | 未能证实 | GMET 相关 KVM/QEMU 补丁出现在 2026 年；**世代归属未能证实** |

---

## 回退路径与代价

> 本节回答「某项硬件能力缺失时，VMM 怎么办、代价多大」。代价量级为公开资料与 KVM 实现结构给出的**工程估计**，不是精确基准数字；精确数字高度依赖负载类型。

### 3.1 没有 EPT / NPT → shadow page table

| 维度 | 说明 |
|---|---|
| 机制 | VMM 拦截 guest 对 CR3 的写入与 guest 页表的修改，为每个 guest 页表组合生成一份**影子页表（shadow page table）**（直接 GPA→HPA 的单级结构），并让硬件使用影子页。guest 写自己的页表时需通过写保护 + `#PF` 回到 VMM 同步（KVM `mmu.c` 的 synchronized / unsynchronized page 与 rmap 反向映射机制）。 |
| 代价 | ① **每次 CR3 写入一次 VM-exit**（进程切换 → guest 内进程切换全部变成 VMM 事件）；② 页表写入触发 `#PF` 并做软件同步，**页表密集型负载（fork/exec、编译、数据库 buffer 管理）常见 3×～10× 以上**；③ 影子页表本身消耗大量宿主内存；④ 实现复杂度极高，是 KVM 最复杂的子系统之一。 |
| 现状 | 现代 x86 服务器/客户端（2008+ Intel、2007+ AMD）几乎都有 EPT/NPT，shadow 路径主要保留给「无嵌套分页的旧硬件」与**嵌套虚拟化时 L2 的某些组合**（L0 可能对 L1 的 EPT 再做一层 shadow）。 |

### 3.2 没有 VPID / ASID → 每次切换 flush TLB

| 维度 | 说明 |
|---|---|
| 机制 | VM entry/exit 时执行全量 TLB 失效（Intel 无 `VPID` 时硬件自动 flush；AMD 无 ASID 时只能 `INVLPGA`/全刷）。 |
| 代价 | 每次转换后 TLB 全冷，**数百~数千 cycle 的重填惩罚**；对以下负载影响最大：guest 内进程/线程切换频繁、syscall 密集、嵌套虚拟化（exit 更频繁）。经验量级：**10%~50% 吞吐下降**，长驻 CPU 密集负载影响接近 0。 |
| 附加价值 | AMD 的 ASID 同时是 **SEV 密钥关联键**，因此缺 ASID 不只是性能问题。 |

### 3.3 没有 APICv / AVIC → 中断靠 VM-exit 模拟

| 维度 | 说明 |
|---|---|
| 机制 | VMM 拦截 guest 对 APIC MMIO/x2APIC MSR 的访问，在软件中维护 IRR/ISR/TMR/LVT/ICR 状态；中断投递 = host 收到中断 → 唤醒/通知 vCPU → 注入（VM entry 时通过 VM-entry interruption-information field）→ guest 处理 → EOI 写操作再次 exit。 |
| 代价 | 每次 APIC 访问、每次 EOI、每次 IPI 各一次 VM-exit；**中断速率高的负载（网络转发、NVMe IOPS、多 vCPU 同步）吞吐下降可达数十个百分点**；posted interrupt 缺失还额外增加「唤醒 halted vCPU」的延迟（host 线程唤醒 + IPI）。 |
| 半虚拟化替代 | `PV EOI`（KVM_FEATURE_PV_EOI）、`PV IPI`（KVM_FEATURE_PV_SEND_IPI）、`PV sched yield`、`steal time` 等半虚拟化接口正是为了在没有 APICv 时降低这部分开销 —— 但它们要求 guest 配合（Linux 支持，Windows 需驱动）。 |

### 3.4 没有 Unrestricted Guest → 纯实模式 / 分页关闭 guest 需特殊处理

| 维度 | 说明 |
|---|---|
| 场景 | guest BIOS 自检阶段、DOS、某些引导加载器、`CR0.PG=0` 的早期内核入口。 |
| 回退 | ① 维持 guest 处于「分页开启」并让 EPT/shadow 把线性地址等同物理地址（需要 VMM 精巧的同步）；② **8086/实模式模拟器**（逐指令模拟，几乎所有内存访问都 exit）；③ 直接拒绝该配置。 |
| 代价 | 引导阶段耗时从「秒级」恶化到「分钟级」甚至不可用；早期 KVM 在关闭嵌套分页下对实模式 guest 支持受限，QEMU 会以「三重故障 / KVM 不支持」告终。现代硬件（Westmere 2010+）已无此问题。 |
| AMD 对照 | NPT 使能后 guest 关闭分页即可运行，故 AMD 侧无独立「Unrestricted Guest」位；**但无 NPT 的老 AMD 硬件上问题与 Intel 相同**。 |

### 3.5 其他能力的缺失代价一览

| 能力缺失 | 回退 | 代价量级 |
|---|---|---|
| MSR/I/O bitmap 缺失 | 全量陷入或不做 MSR 虚拟化 | guest 启动、驱动初始化 VM-exit 数上升 1~2 个数量级 |
| VMCS shadowing 缺失 | 每次 VMREAD/VMWRITE 一次 exit + 软件模拟 | 嵌套虚拟化下降数倍 |
| VMFUNC/EPTP switching 缺失 | VM-exit 改 EPTP 再 entry | 每次视图切换两次 transition（数千 cycle），部分方案不可行 |
| PML / EPT A/D 缺失 | 写保护 + `#PF` 追踪脏页 | 迁移/快照迭代轮数增加，脏页密集负载明显变慢 |
| IOMMU（VT-d / AMD-Vi）缺失 | 设备模拟 / virtio；或放弃直通 | **不是性能问题：直通即隔离失效** |
| ACS 缺失 | 不直通该拓扑下的设备 / ACS 重定向规避 | 无法安全分配同 switch 下多个 VF |
| TDX / SEV 缺失 | 加固 VMM + 内存加密（如 TME/TSME） | VMM 留在信任边界内，恶意 hypervisor 可读取 guest |

---

## 对照表

### A. 核心内存与执行能力

| 能力项 | Intel | AMD | 首次引入代际 | 作用 | 缺失代价 |
|---|---|---|---|---|---|
| 两模式 + transition | VMX root/non-root，`VMXON`/`VMLAUNCH`/`VMRESUME` | host/guest mode，`VMRUN` | 2005–2006（Intel）/ 2006（AMD） | vCPU 与 VMM 在同一物理核上分时运行 | 只能二进制翻译或半虚拟化 |
| 状态区 | VMCS（不透明，`VMREAD`/`VMWRITE`） | VMCB（4KB，control 1024B + save 744B） | 同期 | VM entry/exit 自动保存恢复 vCPU 状态 | 软件逐字段保存恢复，慢且易漏 |
| 二级地址翻译 | **EPT**（Nehalem 2008） | **NPT/RVI**（Barcelona 2007） | 上述 | GPA→HPA 硬件翻译，guest 页表直接用 | shadow page table，**3×~10×** |
| 地址翻译失效报告 | EPT violation（exit qualification 含 GPA/访问类型） | NPF（`EXITINFO1/2`） | 同期 | MMIO 模拟、写保护、缺页填充 | 需解释指令反推，慢且易错 |
| TLB 标签 | **VPID**（Nehalem 2008），`INVVPID` | **ASID**（2006），`TLB_CONTROL` 可单 ASID 刷 | 上述 | 切换不刷全 TLB | 每次切换 flush 全 TLB，10%~50% 回退 |
| 大页 | EPT 2MB / 1GB（Nehalem 2008） | NPT 2MB 大页（Barcelona 2007 起） | 上述 | 减少页表层数/TLB 压力 | 4KB only：页表内存放大、TLB miss 增加 |
| 关闭分页的 guest | **Unrestricted Guest**（Westmere 2010） | 随 NPT 提供（2007） | 上述 | 实模式/`CR0.PG=0` guest 直接运行 | 引导阶段需 8086 模拟，分钟级或不可用 |
| 特权级分离执行 | **MBEC**（Kaby Lake 2017 ⚠） | **GMET**（未能证实） | 上述 | 硬件强制 SMEP/SMAP 语义 | 安全边界弱化 |
| hypervisor 页表（反重映射） | **HLAT**（Alder Lake 2021 客户端）作为 VT-rp 一部分 | 无直接对应（SNP RMP 目标不同） | 上述 | 抵抗 hypervisor 重映射攻击 | 控制流劫持防护缺失 |

### B. 陷入控制与中断虚拟化

| 能力项 | Intel | AMD | 首次引入代际 | 作用 | 缺失代价 |
|---|---|---|---|---|---|
| MSR 陷入 | MSR bitmap（4KB） | MSRPM 位图 | Intel 早期 VMX / AMD 2006 | 按 MSR 选择是否 exit | 全量陷入，启动/驱动路径 exit 数大增 |
| I/O 陷入 | I/O bitmap A/B（64K 端口） | IOPM 位图 | 同期 | 端口访问按需陷入 | 全量陷入或无法虚拟化 I/O |
| CR/DR 访问控制 | CR0/CR4 mask + read shadow、CR3/CR8 exiting | CR0/CR3/CR4/CR8、DR0–DR7 intercept 位 | 同期 | 精确控制敏感寄存器访问 | guest 进程切换全部 exit |
| 异常陷入 | exception bitmap + PF error code mask/match | exception intercept（向量 0–31） | 同期 | 只陷入 MMIO/脏页相关异常 | 全陷入或无法追踪 MMIO |
| TPR/EOI 加速 | TPR shadow（Nehalem 2008）+ virtual-interrupt delivery（Haswell 2013） | AVIC（Zen 1 2017） | 上述 | TPR 写与 EOI 不 exit | 每次中断 2 次 transition，吞吐断崖 |
| APIC 访问虚拟化 | Virtualize APIC accesses + APIC-register virtualization（Nehalem 2008 / Haswell 2013） | AVIC backing page（Zen 1 2017） | 上述 | guest 直接读写虚拟 APIC | 每次 APIC 访问 exit |
| IPI / 中断投递 | posted interrupt + PID（Haswell 2013）、IPI virtualization（Sapphire Rapids 2023） | AVIC IPI（物理/逻辑模式，Zen 1 2017）、x2AVIC（Zen 4 2022） | 上述 | 中断直达运行中/阻塞的 vCPU | 唤醒延迟增加，多 vCPU 同步慢 |
| 中断开关虚拟化 | —（Intel 无独立位，靠 GIF 等价物 + interrupt-window exiting） | **VGIF**（Zen 2 2019 ⚠） | 上述 | `STGI`/`CLGI` 不 exit | 每次临界区进出两次 exit |

### C. 嵌套与扩展

| 能力项 | Intel | AMD | 首次引入代际 | 作用 | 缺失代价 |
|---|---|---|---|---|---|
| 嵌套 VMCS 访问 | VMCS shadowing（Haswell 2013） | VMCB 影子由 L0 软件管理（无对应硬件位） | Haswell 2013 | L1 直接用 VMREAD/VMWRITE | 每次访问一次 exit + 软件模拟 |
| 多地址视图切换 | VMFUNC + EPTP switching（Haswell 2013，最多 512 EPTP） | 无直接对应（可软件换 NCR3） | Haswell 2013 | 零 exit 切换 GPA→HPA 视图 | 每次切换 2 次 transition |
| 脏页日志 | PML（Skylake 2015）、EPT A/D（Haswell/Broadwell ⚠） | NPT 无 A/D 等价（软件写保护） | 上述 | 硬件记录被修改页 | 迁移/快照效率低 |
| 设备 DMA 隔离 | **VT-d**（Nehalem 平台 2008） | **AMD-Vi**（Barcelona 平台 2007） | 上述 | DMA remapping + 中断重映射 | **无 IOMMU 不能安全直通** |
| 设备虚拟功能 | SR-IOV（PCI-SIG 2007，产品化 2008–2009）+ ACS（PCIe 2.x 2007–2008） | 同左（PCIe 标准，非厂商特有） | 上述 | 单卡多 VF 分给多 guest | 只能整卡直通或 virtio |
| 进程级 TEE | SGX（Skylake 客户端 2015；SGX2 Kaby Lake 2017） | SEV/SEV-ES/SEV-SNP（Zen 1 2017 / Zen 1 后期 2017-18 / Zen 3 2021） | 上述 | enclave / 加密 VM | 无 TEE 只能信任 hypervisor |
| 整机级机密 VM | **TDX**（Sapphire Rapids 2023，含 TDX Module） | SEV-SNP（Milan 2021） | 上述 | 把 hypervisor 移出信任边界 | VMM 可读/篡改 guest |

---

## 未决问题

1. **VPID 的确切首发 SKU**：QEMU 的 `Nehalem` 模型含 VPID，但 Intel 是否在 Nehalem 全系还是仅部分 SKU 提供，未逐 SKU 核对。
2. **EPT A/D 位的首发代际**：QEMU 自 Broadwell 起标注，Haswell 未标；Linux 到 2017–2018 才补齐软件支持。需查 Intel SDM 历史版本的 `IA32_VMX_EPT_VPID_CAP` 说明。
3. **posted interrupt 的首发代际**：QEMU 自 IvyBridge 起标 `VMX_PIN_BASED_POSTED_INTR`，而 APICv 通常记作 Haswell；两者是否同期存在分歧。
4. **MBEC 的客户端/服务器首发版本**：Kaby Lake（客户端 7 代）与 Xeon Scalable（Skylake-SP）是否同期支持，未证实。
5. **Bus Lock Detection 首发代际**：Linux 有 `VMX_FEATURE_BUS_LOCK_DETECTION`，但 QEMU `Icelake-Server` 模型未列出，存在分歧。
6. **HLAT/VT-rp 在服务器平台的首发**：目前只有客户端 12 代起的官方依据（Intel 12/13 代数据手册）；Xeon 侧的起始代际未证实。
7. **GMET 的引入世代与完整语义**：KVM/QEMU 补丁 2026 年才出现，AMD 官方 APM 引入代际未找到，**需以 AMD APM Volume 2 / PPR 补证**。
8. **VGIF 的硬件首发世代**：Linux 支持 2017-08，硬件世代常记为 Zen 2，未获 AMD 官方确认。
9. **SEV-ES 的产品首发 SKU 与年份**：公开资料在 2017 宣布 / 2018 产品化 / EPYC 7002 之间表述不一。
10. **TDX Module 的版本与 TDH/TDV 接口细节**：本稿只说明其定位（SEAM 模式下的 Intel 官方固件模块），接口清单未展开。
11. **VT-d 首个产品化芯片组**：Nehalem 平台为通常说法，但具体芯片组型号未核对。
12. **各回退路径的精确性能数字**：本稿给出的是量级估计，缺少同硬件对照基准（需引用具体论文/基准报告）。

---

## 参考资料

> 访问日期：**2026-09-16**

### 官方规范与手册

- Intel® 64 and IA-32 Architectures Software Developer's Manual, Volume 3C（VMX 与 VMX 能力 MSR、附录 A 的 VMX 能力编码位定义）— 本稿通过 Linux/QEMU 源码中的对应位定义间接核对，SDM 原文需 Intel 站点登录下载。
- Intel® Architecture Instruction Set Extensions Programming Reference（含 TDX、VT-rp 等扩展）— <https://www.intel.cn/content/dam/develop/external/us/en/documents/architecture-instruction-set-extensions-programming-reference-737410.pdf>
- AMD64 Architecture Programmer's Manual Volume 2: System Programming（SVM / VMCB / intercept 定义，第 15 章）— <https://www.amd.com/system/files/TechDocs/24593.pdf>（本次访问返回 HTTP/2 错误，未取得正文，故 AMD 侧细节以 Linux `svm.h` 与 KVM 文档为准）
- AMD SEV-SNP 规范（RMP / VMPL / attestation）— 经 KVM 文档引用核对

### Intel 官方产品页

- Intel Raptor Lake（13 代 Core / Xeon E 2400 / 6300）数据手册 Vol 1：Hypervisor-Managed Linear Address Translation — <https://edc.intel.com/content/www/us/en/design/products/platforms/details/raptor-lake-s/13th-generation-core-processors-datasheet-volume-1-of-2/003/015/hypervisor-managed-linear-address-translation/>
- 同上：Intel APIC Virtualization Technology (Intel APICv) — <https://edc.intel.com/content/www/cn/zh/design/products/platforms/details/raptor-lake-s/13th-generation-core-processors-datasheet-volume-1-of-2/010/014/intel-apic-virtualization-technology-intel-apicv/>
- Intel：哪些 Xeon 支持 Intel TDX（按代际列出） — <https://www.intel.com/content/www/us/en/support/articles/000091103/processors/intel-xeon-processors.html>
- Intel：Intel TDX Feature Readiness by Intel Xeon Processor Generation — <https://www.intel.com/content/www/us/en/support/articles/000099708/processors/intel-xeon-processors.html>
- 12th Gen Intel Core (Alder Lake) 数据手册 Vol 1：Supported Technologies（含 VT-rp 相关项） — <https://edc.intel.com/content/www/th/th/design/ipla/software-development-platforms/client/platforms/alder-lake-desktop/12th-generation-intel-core-processors-datasheet-volume-1-of-2/001/supported-technologies/>
- Intel Technology Journal 2006 Vol 10 Iss 3：IOVM / VT-d 论文 — <https://www.thailand.intel.com/content/dam/www/public/us/en/documents/research/2006-vol10-iss-3-intel-technology-journal.pdf>

### Linux 内核与 KVM 文档

- KVM for x86 systems（总览） — <https://www.kernel.org/doc/html/latest/virt/kvm/x86/index.html>
- The x86 kvm shadow mmu（shadow page table 机制与代价） — <https://www.kernel.org/doc/html/latest/virt/kvm/x86/mmu.html>
- Nested VMX（嵌套虚拟化、Turtles Project、vmcs12 结构） — <https://www.kernel.org/doc/html/latest/virt/kvm/x86/nested-vmx.html>
- Secure Encrypted Virtualization (SEV)（SEV / SEV-ES / SEV-SNP 的 KVM 接口） — <https://www.kernel.org/doc/html/latest/virt/kvm/x86/amd-memory-encryption.html>
- KVM CPUID bits（PV EOI / PV IPI 等半虚拟化能力位） — <https://www.kernel.org/doc/html/v6.6/virt/kvm/x86/cpuid.html>
- The Turtles Project: Design and Implementation of Nested Virtualization, OSDI 2010 — <https://www.usenix.org/events/osdi10/tech/full_papers/Ben-Yehuda.pdf>
- Linux `arch/x86/include/asm/vmx.h`（v6.6）— <https://raw.githubusercontent.com/torvalds/linux/v6.6/arch/x86/include/asm/vmx.h>
- Linux `arch/x86/include/asm/vmxfeatures.h`（v6.6）— <https://raw.githubusercontent.com/torvalds/linux/v6.6/arch/x86/include/asm/vmxfeatures.h>
- Linux `arch/x86/include/asm/svm.h`（v6.6，VMCB 布局 / intercept 枚举 / AVIC / VGIF） — <https://raw.githubusercontent.com/torvalds/linux/v6.6/arch/x86/include/asm/svm.h>
- Linux KVM 提交检索（GitHub Commits API）：`x2AVIC`（2022-06-24）、`Virtual GIF`（2017-08-23）、`AVIC`（2016-05-18）、`EPT accessed/dirty bits`（2017-04-07）、`GMET bit definitions`（2026-05-10）

### QEMU 源码（代际能力的交叉核对基准）

- QEMU v9.0.0 `target/i386/cpu.c`（各 CPU 型号的 `FEAT_VMX_*` 定义，用于判定 Nehalem/Westmere/SandyBridge/IvyBridge/Haswell/Broadwell/Skylake/Icelake/SapphireRapids 的能力差异） — <https://raw.githubusercontent.com/qemu/qemu/v9.0.0/target/i386/cpu.c>
- QEMU `target/i386/cpu.h`（VMX feature/capability 位常量）

### 邮件列表 / 第三方分析

- KVM 邮件列表：`[PATCH v13 0/3] x86, apicv: Add APIC virtualization support` — <https://www.mail-archive.com/search?l=kvm@vger.kernel.org&q=subject:%22%5C%5BPATCH+v13+0%5C%2F3%5C%5D+x86%2C+apicv%5C%3A+Add+APIC+virtualization+support%22>
- LKML：`[PATCH 0/2] KVM: SVM: Virtual GIF`（2017） — <https://www.openwall.com/lists/kernel-hardening/> 与 <https://lists.openwall.net/linux-kernel/2017/08/16/608>
- LKML：`[RFCv2 PATCH 00/12] Introducing AMD x2APIC Virtualization (x2AVIC) support` — <https://lkml.iu.edu/hypermail/linux/kernel/2203.1/00900.html>
- LKML：`[PATCH v6 00/17] Introducing AMD x2AVIC and hybrid-AVIC modes` — <https://lkml.iu.edu/hypermail/linux/kernel/2206.3/05465.html>
- Phoronix：Linux 6.0 KVM Brings Intel IPI Virtualization, AMD x2AVIC — <https://www.phoronix.com/news/Linux-6.0-KVM>
- Phoronix：Linux 5.18 KVM Prepares For Intel IPI Virtualization, Larger AMD VMs — <https://www.phoronix.com/news/Linux-5.18-KVM>
- Phoronix：AMD Sends Out New Linux Code For SEV-SNP With EPYC 7003 Series — <https://www.phoronix.com/news/AMD-SEV-SNP-Linux-RFC>
- Phoronix：Intel Launches 4th Gen Xeon Scalable (Sapphire Rapids) — <https://www.phoronix.com/review/intel-xeon-sapphire-rapids-max/3>
- The Next Platform：The Rest Of The World Can Finally Get Sapphire Rapids Xeon SPs（2023-01-10 GA） — <https://www.nextplatform.com/compute/2023/01/10/the-rest-of-the-world-can-finally-get-sapphire-rapids-xeon-sps/1641850>
- SecurityWeek：Intel Adds TDX to Confidential Computing Portfolio With Launch of 4th Gen Xeon Processors — <https://www.securityweek.com/intel-adds-tdx-to-confidential-computing-portfolio-launch-of-4th-gen-xeon-processors/>
- AMD Xen 邮件列表（2007）：AMD IOMMU / AMD-Vi 早期讨论 — <https://lists.xenproject.org/archives/html/xen-users/2007-02/msg01011.html>
- KVM 邮件列表（2008）：`SVM: Add Support for Nested Paging in AMD Fam16 CPUs`（NPT 支持补丁） — <https://sourceforge.net/p/kvm/mailman/kvm-devel/thread/20080126100605.GD21476%408bytes.org/>
- tandasat：Intel VT-rp - Part 1. remapping attack and HLAT（2023-07-05，含 Alder Lake 上启用 HLAT 的实测） — <https://tandasat.github.io/blog/2023/07/05/intel-vt-rp-part-1.html>（经 Wayback Machine 检索）
- GitHub：tandasat/Hello-VT-rp（HLAT / PW / GPV 演示 hypervisor） — <https://github.com/tandasat/Hello-VT-rp>
- QEMU 补丁列表：`target/i386: add new AMD EPYC models for GMET enablement` — <https://lists.nongnu.org/archive/html/qemu-devel/2026-03/msg08090.html>
- Mi et al., USENIX Security 2020（EPTP switching 特性分析） — <https://www.usenix.org/system/files/sec20-mi.pdf>
