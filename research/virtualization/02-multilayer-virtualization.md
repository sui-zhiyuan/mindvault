# 多层硬件虚拟化方案调研

> 中间调研稿 · 更新：2026-09-16
> 范围：Intel VMX / AMD SVM / ARM64 EL2 / 设备侧 IOMMU-SMMU / 云厂商绕开方案
> 说明：本文对未能从可靠来源核实的内容显式标注「**未能证实**」，不做推测性填充。

## 核心结论

1. **"多层"至少有三义，必须先辨析**：①**嵌套虚拟化**（L0 hypervisor / L1 guest hypervisor / L2 guest）；②**多级地址翻译**（GVA→GPA→HPA，设备侧再加一级 IOMMU/SMMU）；③**多级虚拟化栈**（KVM → Firecracker/Cloud Hypervisor → 容器 → 语言级沙箱）。三者正交、可叠加，混为一谈会直接导致选型错误。
2. **嵌套虚拟化的根本矛盾是"硬件资源只有一份"**：CPU 只有一套 VMCS/VMCB，内存只有二级硬件翻译（EPT/NPT/stage-2），而 L1 也需要一套来跑 L2。L0 必须做**结构合并**（VMCS12 → VMCS02 字段合并）或**影子**（shadow EPT / shadow page table）。性能代价与功能子集几乎全部由这一条推导出来。
3. **Intel 的关键硬件补丁是 VMCS shadowing（Haswell, 2013+）**：它把 L1 对 VMCS12 的 `VMREAD/VMWRITE` 从"必须 VM exit 到 L0"变成指向 shadow VMCS 的普通内存访存。Turtles 论文实测该优化使切换开销下降 **84.6%（13,000 → 2,000）**，是嵌套性能最重要的一次性收益。
4. **nested EPT/NPT 是性能分水岭**：没有它，L1 只能用影子页表，嵌套场景要把 3 级翻译压进 1 张 shadow page table；有它，L2 的两级翻译可以硬件直通，L0 只在"L1 的 EPT 页表本身被改"时介入。
5. **数量级结论（Turtles, OSDI 2010，KVM 同时作 L0/L1，x86 无嵌套硬件支持）**：相对单层虚拟化，kernbench 开销 **14.5%**、SPECjbb 得分下降 **7.82%**；把 `VMREAD/VMWRITE` 的开销扣掉后分别为 **10.3% / 6.3%**。同一实验中 L0 自身占用的 CPU 从单层的 **2.28%** 升到嵌套的 **5.17%**——"多一层"放大的是宿主开销，不是线性叠加。
6. **朴素软件模拟的代价是数量级的**：同一论文指出，更朴素的 MMU 虚拟化做法会让部分有用负载"至少慢 3 倍"（three-fold slowdown）。
7. **云厂商官方定位：嵌套是开发便利，不是生产性能方案**。AWS 文档把三层明确为 L0 = Nitro hypervisor、L1 = 客户实例、L2 = 实例内创建的 VM，L1 只支持 KVM 与 Hyper-V，并建议"对性能敏感或有严格延迟要求的客户改用裸金属实例"。
8. **ARM 的硬件嵌套支持来得很晚，而且更晚才进主线**：硬件侧直到 **ARMv8.4 的 FEAT_NV / FEAT_NV2** 才提供 `HCR_EL2.NV/NV2` 与 `VNCR_EL2`；在此之前 L0 只能靠 `HCR_EL2.TVM/TRVM` 把 L1 对虚拟内存控制寄存器的访问 trap 到 L0 做 trap-and-emulate。**主线 KVM/arm64 的嵌套支持只覆盖 FEAT_NV2**（补丁系列标题即 "Nested Virtualization support (FEAT_NV2 only)"，即不支持只有 FEAT_NV 的实现），并且**直到 2025 年才随 Linux 6.16 合入**——这比 x86 的 VMCS shadowing（2013）晚了约 12 年。
9. **设备侧还有第三级翻译**：IOMMU/SMMU 的 stage-1（guest 控制）+ stage-2（hypervisor 控制）在嵌套下需要再叠一层；Intel VT-d Scalable Mode 的文档直接称之为 "three-stage address translation"。AMD IOMMU 的 nested translation 支持到 2025–2026 仍在补丁评审中，尚未稳定。
10. **内存超分、dirty logging、大页是 EPT/NPT 路线的三个结构性痛点**：写保护页会累加 `->disallow_lpage` 从而阻止大页实例化；dirty logging 依赖 EPT 写保护，让每次写都产生 EPT violation；EPT 无法自然表示"不存在但可换出"的 GPA（要靠 balloon/swap 配合）。
11. **绕开方案（裸金属 + DPU）本质是"让客户自己的 hypervisor 当 L0"**：AWS Nitro、阿里云神龙 MOC、华为云擎天把网络/存储/安全卸载到卡上，宿主 hypervisor 极小化甚至不存在，从根上避免 exit 放大。
12. **本稿的选型建议**：能用裸金属/DPU 就不用嵌套；必须嵌套时优先 Intel VMCS shadowing + nested EPT、AMD nSVM + NPT（并接受功能子集）、ARM 优先 ARMv8.4-NV；多级栈（VM → 容器 → 语言沙箱）中的各层应尽量落在**不同的资源维度**上，避免在同一维度叠三层虚拟化。

---

## 一、"多层"的三种含义辨析

### 1.1 三种含义

| 含义 | 层次划分 | 隔离边界由谁定义 | 典型用途 | 它**不**涉及什么 |
|---|---|---|---|---|
| ① **嵌套虚拟化** | L0 hypervisor（裸机）/ L1 guest hypervisor / L2 guest | 硬件虚拟化扩展（Intel VMX、AMD SVM、ARM EL2）+ 上层 hypervisor 的 VMCS/VMCB 合并逻辑 | 云上跑 KVM、Docker Desktop、WSL2、CI 里跑模拟器/测试 hypervisor、VDI 教学实验 | 与容器、语言沙箱无关 |
| ② **多级地址翻译** | GVA→GPA（guest 页表）/ GPA→HPA（EPT/NPT/stage-2）/ 设备侧 IOMMU-SMMU stage-1 + stage-2 | MMU 与 IOMMU 的硬件页表 | 所有硬件虚拟化的内存与设备通路；PCIe ATS/PASID、SVA | 不需要存在第二层 hypervisor |
| ③ **多级虚拟化栈** | KVM → Firecracker/Cloud Hypervisor → 容器（namespace/cgroup）→ 语言级沙箱（seccomp/gVisor/WASM） | 各层软件策略，逐层收窄能力 | AI Agent 沙箱、FaaS、多租户 CI、浏览器渲染沙箱 | **通常只有一层硬件虚拟化**，其余是软件隔离 |

### 1.2 三条辨析要点

- **②是①的实现基础**。嵌套虚拟化之所以难，恰恰是因为"硬件只有 2 级翻译，而场景需要 3 级"。KVM 的 MMU 文档把这件事写得很直白：需要在支持 1 级（传统）或 2 级（TDP）翻译的硬件上，编码 1–3 级翻译；数量匹配时走 **direct mode**，否则走 **shadow mode**。
- **①是③的一种可能形态，但③通常不含①**。多级栈里除最底层的微 VM 之外，容器层、语言沙箱层都不产生 VM exit，因此成本量级完全不同。把"VM 里跑容器"叫作"两层虚拟化"是对的，但它和"VM 里跑 hypervisor"的代价完全不是一回事。
- **只有①会引入 "L0 exit 放大"**：一次 L2 的 VM exit 在嵌套下可能触发多次 L0↔L1 切换；②的代价是页表走查次数与 TLB 语义变化；③的代价主要是 syscall/系统调用路径的软件过滤开销。

### 1.3 叠加关系（示意）

| 组合 | 是否常见 | 说明 |
|---|---|---|
| ① + ② | 必然叠加 | 有嵌套就一定有三级地址翻译的问题（除非 L1 不做内存虚拟化） |
| ① + ③ | 常见 | 云主机（L1）里跑 Kata/gVisor（L2 或同层进程隔离）；见本仓库 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` |
| ② + ③ | 常见 | 微 VM（Firecracker）里跑容器：只有一层硬件虚拟化，但内存通路仍是 GPA→HPA + 容器 overlay |
| ① + ② + ③ | 存在但少见 | 例如云上 VM（L1）里跑 microVM（L2）再跑容器；性能与功能子集问题会叠加 |

---

## 二、嵌套虚拟化的实现方案族

### 2.1 共同基本盘：vmcs01 / vmcs12 / vmcs02

KVM 官方文档《Nested VMX》给出的术语是理解所有实现的地基：

| 结构 | 含义 |
|---|---|
| **vmcs01** | L0 为 L1 建立的 VMCS |
| **vmcs12** | L1 为 L2 建立的 VMCS（`struct vmcs12`）；对 L1 **不透明**，只能通过 VMREAD/VMWRITE 访问，只有 `revision_id`、`abort` 两个字段对 L1 可见 |
| **vmcs02** | L0 实际用来运行 L2 的 VMCS；由 vmcs12 与 vmcs01 的字段**合并**而来 |

关键设计点：

- **ABI 是 Intel 规范本身**，不是 KVM 私有 ABI。文档明确说嵌套 VMX 的目标是"向 L1 提供一个标准且（最终）功能完整的 VMX 实现"，其 ABI 的正式定义是 Intel SDM（该文档 2018 年版本引用的是 **Vol 3B**；现行 SDM 中 VMX/EPT 相关章节在 **Vol 3C/3D**）。
- **字段合并是必需的**，不能简单照抄。示例：L1 想在 L2 中 trap 事件 EA，而 L0 不 trap L1 的 EA，则 L0 必须在 vmcs02 中为该事件设置 trap，事件才能被转发给 L1；L1 用于向 L2 注入事件的字段、处理器用于记录 exit 原因的字段也必须合并。
- **默认关闭**：Intel 需要给 `kvm-intel` 传 `nested=1`；QEMU 默认 CPU 模型 `qemu64` 不含 VMX，需显式 `-cpu host` 或 `-cpu qemu64,+vmx`。

### 2.2 Intel x86

| 机制 | 细节 | 硬件前提 | 性能代价 | 代表实现 |
|---|---|---|---|---|
| **VMCS shadowing** | L1 的 `VMREAD/VMWRITE` 不再 exit，而是读写由 `VMCS link pointer` 指出的 **shadow VMCS**；L0 把 vmcs12 的字段同步进 shadow VMCS | Haswell（2013）及以后；CPU 需报告对应 secondary-exec 能力位 | 消除嵌套中最贵的一类 exit。Turtles 实测去掉 VMCS 访问 trap 后切换开销 13,000 → 2,000（**-84.6%**）；论文另称在该优化下嵌套开销可降约 80% | KVM nVMX（`enable_shadow_vmcs`）、Hyper-V |
| **VMCS link pointer** | VMCS 的 64 位控制字段，指向 shadow VMCS；启用 shadowing 后 L1 的 VMCS 访问被硬件重定向 | 同 VMCS shadowing | 需要 L0 保证 link pointer 合法性；KVM 有专门校验路径 | KVM nVMX |
| **nested EPT（nEPT）** | L2 的两级翻译（GVA→GPA→HPA）在硬件上直通：硬件用 L1 的 EPT（EPT12/EPT02）继续走查。L0 只在 L1 的 EPT 页表自身被写入、或出现 MMIO/reserved bit/EPT misconfig 时介入 | Intel EPT（Nehalem 2008+） | TLB 未命中时二维走查次数翻倍；L0 需对 L1 的 EPT 页做写保护并在写入时 trap 更新 | KVM nVMX |
| **VM-entry/exit 嵌套放大** | L2 的一次 exit 可能需要在 L0 与 L1 之间来回多次：L0 先处理，再通过 VM-entry 送到 L1；MSR bitmap、CR 访问、I/O、EPT violation 路径都可能各自放大 | 无（通用现象） | "一次 L2 exit 触发多次 L0 exit"是嵌套开销的主要来源；Turtles 观测到 L0 占用 CPU 从 2.28%→5.17% | 所有实现 |
| **APICv 在嵌套下的限制** | APICv 依赖 virtual-APIC page 与 TPR threshold，嵌套时这些状态需要在 L0/L1 两侧一致；KVM 在嵌套场景对 APICv 的启用有额外约束 | APICv（Haswell+ 部分 SKU） | 不可用时中断投递退回较慢路径，中断密集负载受影响 | KVM nVMX |

> **待补**：VMCS shadowing 的具体能力位名称、nested EPT 的 shadow EPT 失效条件、KVM nVMX 明确不支持的 VMX 特性清单（posted interrupt / VMFUNC 等），需以 Intel SDM Vol 3C 与 KVM 源码/提交记录逐条核实后补入。当前稿对这部分标为**部分未证实**。

### 2.3 AMD x86

| 机制 | 细节 | 硬件前提 | 性能代价 | 代表实现 |
|---|---|---|---|---|
| **SVM 嵌套（nSVM）** | 与 nVMX 对偶：L1 用 `VMRUN`/`VMLOAD`/`VMSAVE` 管 L2；KVM 需要区分 VMCB01 / VMCB12 / VMCB02 | AMD-V；`kvm_amd` 的 nested 支持（默认可开） | 与 Intel 同构的 exit 放大问题 | KVM nSVM |
| **VMCB clean bits** | VMCB 中的 clean bits 让硬件跳过未修改字段的加载/保存。嵌套时 L0 会**背着 L1 改写 VMCB 字段**，因此必须正确地清除相应 clean bit，否则硬件会使用陈旧状态 | AMD APM Vol 2（VMCB / clean bits 章节） | clean bit 处理错误直接导致状态不一致；正确处理带来额外字段同步 | KVM nSVM |
| **intercept 的嵌套处理** | L1 为 L2 设置的 intercept 与 L0 自身的 intercept 需要合并成"有效 intercept 集合"；`VMRUN`/`VMLOAD`/`VMSAVE`/`STGI`/`CLGI` 等指令在 L1 中执行时被 L0 拦截并模拟 | AMD-V | 每类 intercept 都增加 L0 介入；合并逻辑是 nSVM 复杂度主体 | KVM nSVM |
| **nested NPT** | L1 的 NPT（由 `nested_cr3` 指定）被 L0 使用；L1 的 NPT 页表被写保护，写入时 trap 更新；NPT fault 需要在 L0/L1 间正确归属 | AMD NPT（Barcelona 2007+） | 与 nested EPT 同构：二维走查 + 页表写保护 | KVM nSVM |
| **AVIC 嵌套限制** | AVIC（Advanced Virtual Interrupt Controller）在嵌套场景下的可用性受限，KVM 对嵌套与 AVIC 的启用组合有约束 | AVIC（Zen 及以后，需固件/平台支持） | 不可用时中断投递退回软件路径 | KVM nSVM |

> **待补**：AMD nested 的最小 CPU/特性前提、AVIC 在嵌套下"禁用"的确切条件（内核代码与提交信息），当前稿标为**部分未证实**。

### 2.4 软件模拟路径（无硬件嵌套支持时）

| 路径 | 做法 | 代价 | 代表实现 |
|---|---|---|---|
| **#UD：L1 根本起不来** | 嵌套 VMX 默认关闭（需 `kvm-intel.nested=1`）；关闭或平台无能力时，L1 执行 `VMXON` 等指令触发 `#UD`。结果是 L1 hypervisor **无法启动**，而不是"变慢" | 功能不可用 | KVM（Intel 上默认关闭） |
| **Turtles 的软件多层方案** | 在硬件不支持嵌套的 x86 上，用**软件影子**维护多层 VMCS，并用 **multi-dimensional paging** 把 3 级翻译压进硬件 2 级；用 **multi-level device assignment** 处理 I/O 直通 | 论文报告最好情况为相对单层虚拟化 6–8% 开销；但更朴素的 MMU 虚拟化做法会让部分负载"至少慢 3 倍" | KVM 的 Turtles 分支（IBM，OSDI 2010） |
| **Xen nested HVM** | L1 配置 `nestedhvm=1` + `hap=1`；Xen 明言**不支持 L1 使用 shadow mode**（"性能非常差"），且 L2 的 EPT 只能建在 L1 的 EPT 之上 | Xen 4.4 时 Intel 上为 "tech preview"、AMD 上为 "experimental"，官方**不建议生产使用**；已知 L1 中用 populate-on-demand 或 guest paging 可能死锁 L0（可被 L1 管理员用于 DoS） | Xen（`nestedhvm`） |
| **VMware 早期做法** | 在宿主机上以二进制方式模拟 guest hypervisor 所需的指令；Turtles 论文以 VMware Server 作 L1 实测：kernbench 开销 **14.98%**、SPECjbb **8.85%** | 与 KVM 作 L1 同量级（论文称"结果类似"） | VMware Server / ESX（历史） |

### 2.5 ARM64：EL2 嵌套

| 机制 | 细节 | 硬件前提 | 性能代价 |
|---|---|---|---|
| **trap-and-emulate（ARMv8.4 之前）** | 对 v8.0 时代的 guest hypervisor，L0 设置 `HCR_EL2.TVM \| TRVM \| NV1`，把 L1 对 EL1 虚拟内存控制寄存器的访问 trap 到 L0 模拟；L1 的 `HCR_EL2` 虚拟状态需被尊重 | ARMv8.0 + 虚拟化扩展 | 每次寄存器访问都要 trap+模拟，是嵌套最贵的一类路径 |
| **VHE guest 的原生访问（ARMv8.1+）** | 若 L1 使用 VHE（virtual E2H=1），L1 对 EL1 VM 控制寄存器（`TTBR0_EL1`/`TTBR1_EL1`/`TCR_EL1`/`SCTLR_EL1` 等）的访问**本身就是 EL2 状态**，因此 L0 可以不 trap，允许原生访问，同时仍要遵守虚拟 `HCR_EL2` 状态 | ARMv8.1 VHE | 显著少于纯 trap 方案；内核里仍需针对 set/way cache 维护操作保留 `HCR_EL2.TVM`（存在尚未解决的冲突，代码注释明确写了 TODO） |
| **FEAT_NV / NV2（ARMv8.4 硬件嵌套）** | 通过 `HCR_EL2.NV`（FEAT_NV）与 `HCR_EL2.NV2`（FEAT_NV2）启用嵌套；`VNCR_EL2`（Virtual Nested Control Register）把虚拟 EL2 的一整组系统寄存器**重定向到内存**，从而让 L1 的 EL2 状态可被 L0 以内存方式访问/保存 | ARMv8.4 的 FEAT_NV（特性字段 `ID_AA64MMFR2_EL1.NV`）；**主线 KVM 要求 FEAT_NV2** | 预期显著优于 trap-and-emulate；**未找到公开的 ARMv8.4-NV vs trap 模拟的定量对比数据**（未能证实） |

补充说明：

- `HCR_EL2.NV` / `NV2` / `VNCR_EL2` 属于 ARMv8.4 的 **FEAT_NV / FEAT_NV2**；ARMv8.3 未引入 NV。**ARMv8.3 与嵌套的具体关系未能证实**，本稿不臆断。
- **主线状态（已核实）**：KVM/arm64 的嵌套虚拟化补丁系列长期评审（可见 v10/v11 系列），最终以 **"Nested Virtualization support (FEAT_NV2 only)"** 的形式于 **Linux 6.16（2025 年合并窗口）** 合入。含义有两点：①**只支持 FEAT_NV2**，仅有 FEAT_NV 的实现不在支持范围内；②相比 x86 的 VMCS shadowing（2013），ARM 的可用嵌套支持晚了约 12 年，因此 ARM 云平台上"嵌套可用"高度依赖内核版本与 CPU 代次，选型时必须按目标内核实测。已合入主线不等于生产级成熟，嵌套 stage-2 的 reverse map 等仍在后续版本继续演进。

### 2.6 绕开方案：裸金属实例 + DPU/SmartNIC 卸载

思路：**取消 L0 这一层，或把 L0 的 I/O 工作卸载到专用芯片**，让客户自己的 hypervisor 成为 L0。

| 方案 | 卸载了什么 | 与嵌套的关系 | 来源可信度 |
|---|---|---|---|
| **AWS Nitro System** | 网络（VPC）、存储（EBS/本地 NVMe）、安全等卸载到 Nitro Cards；宿主跑 Nitro hypervisor（基于 KVM） | AWS 在支持嵌套的实例上把 VT-x 透传给 L1；同时提供裸金属实例给性能敏感客户，官方明确建议后者 | AWS 官方文档（已核实） |
| **阿里云神龙（X-Dragon）+ MOC 卡** | 用 MOC 卡卸载 virtio-net/virtio-blk 等 I/O 与虚拟化开销，实现"裸金属 + 弹性" | 客户可直接跑自己的 hypervisor，成为 L0 | **部分未证实**：需以阿里云官方文档核实 MOC 的确切卸载边界与性能数字 |
| **华为云擎天（Qingtian）架构** | DPU/擎天卡卸载网络、存储、虚拟化，软硬协同 | 同上：让客户的 hypervisor 处于 L0 | **部分未证实**：需以华为云官方白皮书核实 |
| **通用类别：NVIDIA BlueField 等 DPU** | 把 virtio/OVS/存储协议栈从宿主 CPU 移到卡上的 ARM 核 | 减少宿主 hypervisor 的 exit 与中断负担 | **未证实具体数字** |

**为什么这能解决问题**：嵌套的性能损失主要来自"L2 的每次 exit 都要经过 L0 的放大"。裸金属/DPU 方案让客户的 KVM 直接面对硬件（无 L1 中间层），同时把 I/O 路径从"多个 exit + 中断注入"变为"卡上直接 DMA"，两头都消除了放大源。

> **未能证实**：AWS Nitro / 神龙 / 擎天 三者的"裸金属 vs 嵌套 VM"横向定量对比数据；本稿未找到可交叉验证的公开基准。

---

## 三、嵌套虚拟化解决了什么问题（以及它的代价）

### 3.1 场景：为什么必须要嵌套

| 场景 | 为什么需要嵌套 | 不嵌套的替代方案 |
|---|---|---|
| 云上跑 **KVM**（用户自带 hypervisor） | 客户想在云主机里管理自己的 VM、用熟悉的 hypervisor | 裸金属实例 |
| **Docker Desktop / WSL2** | 两者都需要宿主提供硬件虚拟化能力（WSL2 依赖 Hyper-V/WSL 的虚拟化层，Docker Desktop 在 Linux 上要 `/dev/kvm`） | 本地开发机；或云上裸金属 |
| **CI 里跑模拟器 / 测试 hypervisor** | Android 模拟器、QEMU 加速模式需要 KVM/HAXM/WHPX；hypervisor 开发者需要在 VM 里调试自己的 hypervisor | 裸金属 CI runner |
| **VDI / 教学实验** | 在给定的一个 VM 里让学生有 root 权限跑自己的 hypervisor | 每人一台物理机 |
| **云上 Android 模拟器** | 移动应用云测需要大量可加速的模拟器实例 | 裸金属实例池 |
| **机密计算 + 嵌套** | 在 CVM（TDX/SEV-SNP）内再跑 VM 或 enclave | **受限**：主流 CVM 技术对"guest 内再开 CVM"支持有限；实践上多用同层 enclave / 进程级隔离替代（见「未能证实」） |
| **Kata / gVisor 在 VM 内的分层部署** | 云主机本身就是 L1，客户在其内跑 Kata（每 Pod 一个 microVM）或 gVisor | 直接使用托管 K8s + 厂商提供的安全容器 |
| **软件定义的一切（SDx）跑在 VM 内** | 在 VM 里跑 Ceph/OVS/k8s 等"需要虚拟化能力"的基础设施 | 裸金属 |

### 3.2 代价与限制

**（1）exit 放大是结构性的**：Turtles 论文明确指出，L0 在单层 guest 场景只占 2.28% 的周期，在嵌套 guest 场景占 5.17%——"L0 的工作量增加了一倍以上"。原因是 L2 的每次 exit 都可能需要在 L0 与 L1 之间转发，而 VMCS 访问曾经是其中最大的一块（论文：切换成本 13,000 → 2,000 周期）。

**（2）功能子集**：所有实现都只暴露硬件 VMX/SVM 特性的一个子集。

- KVM 的 nested VMX 文档直言："并非 VMX 的所有特性当前都被完整支持"，目标是先从流行 hypervisor 实际用到的特性做起。
- Xen 明确不支持 L1 使用 shadow mode 承载 L2。
- ARM 的 NV 支持在主线 KVM 中经历了长期评审。

> 具体"不支持清单"（posted interrupt、VMFUNC、VMCS shadowing 暴露给 L1 等）**待以 KVM 源码/提交记录与 Intel SDM 逐条核实**，当前标为部分未证实。

**（3）性能下降幅度**：见第五节。核心数量级是——**在软件模拟路径下，相对单层虚拟化的 CPU 开销约 6%–15%，I/O 路径可达 25%–50% 的吞吐损失**（Turtles 的 netperf 数据，单核 + 1Gb 链路）。

**（4）运维复杂度**：L1 hypervisor 看到的 CPU 特性是 L0 过滤后的结果；L1/L2 的 live migration、快照、dirty logging 在多层下都要重新组合（Xen wiki 直接把 "L1/L2 save restore、live migration" 列为 **Not Tested**）。

**（5）安全边界变复杂**：Xen wiki 明确记录了"L1 管理员可以 DoS L0 hypervisor"的已知问题（L1 中启用 populate-on-demand 或 guest paging 承载 L2 可能导致 L0 死锁），并因此不建议生产使用。

### 3.3 机密计算 × 嵌套（部分证实）

"在 CVM 里再开一个 CVM"不是原理上不可能，但需要 **L0 额外虚拟化 CVM 专有的内存与页状态机制**，因此远比普通嵌套复杂：

- **AMD SEV-SNP 侧有公开证据**：2023 年的 RFC 补丁系列《Support nested SNP KVM guests on Hyper-V》明确目标是"在 Hyper-V 上启用 SNP-host 支持，从而可以启动嵌套的 SNP guest"，且"guest 或 QEMU 无需改动"。该系列揭示了 L0 必须补齐的东西：
  - **RMP（Reverse Map Table）**：Hyper-V 不提供，需要客户内核在启动时自行分配并维护一份 shadow rmp table；
  - **RMP 更新与页状态转换指令**：实现 **MSR 形式的 `rmpupdate`/`psmash`**，补丁说明明确指出这些指令"是面向虚拟化环境的"。
- **Intel TDX 侧**：**未能证实**其嵌套 TD 的官方支持状态与路线图。
- **结论**：机密计算与嵌套的组合仍是前沿领域，可用性取决于 L0 厂商是否专门虚拟化了这些机制（而非 x86 通用的 VMX/SVM 嵌套能力）。选型时不能假设"云上支持嵌套 ⇒ 云上支持嵌套的 CVM"。

---

## 四、内存虚拟化方案对比

### 4.1 共同前提：硬件级数与场景级数不匹配

KVM 的 x86 shadow MMU 文档把要处理的三种翻译列得很清楚：

| 场景 | 需要的翻译链 |
|---|---|
| guest 未开分页 | gpa → hpa |
| guest 开分页 | gva → gpa → hpa |
| guest 自己跑 guest（嵌套） | ngva → ngpa → gpa → hpa |

而硬件只支持 1 级（传统）或 2 级（TDP = Intel EPT / AMD NPT）。**数量匹配就用 direct mode，不匹配就用 shadow mode**——这是所有内存虚拟化方案的分类轴。

### 4.2 方案一：Shadow page table（软件）

**机制**：hypervisor 在 guest 页表之外另建一套 shadow page table，让硬件执行 GVA→HPA 的直接翻译。

- guest 的页表页被**写保护**；guest 写页表触发 fault，KVM 同步对应 spte 后放行。
- 关键优化是 **unsync page**：若某 guest 页表页从当前 `cr3` 可达，则 guest **有义务**在用该翻译前执行 `INVLPG`，KVM 借此**取消写保护**让 guest 自由修改，等 `INVLPG` 时再同步。这减少了"guest 连改多条 pte"导致的反复模拟。
- 代价：TLB flush 时需要重新同步所有可达的 unsync shadow page；维护 **rmap（反向映射）** 以便在 gfn→pfn 变化时撤销 spte；`cr0.wp=0` 的模拟在 SMEP/SMAP 下需要拆分权限（KVM 会把 `cr0.wp`、`!cr0.wp && cr4.smep`、`cr4.smap && !cr0.wp` 编入 shadow page 的 **role**，因此不同组合下同一页会有多份影子）。
- **退化保护**：`write_flooding_count` 统计某页表自上次真正被使用以来的模拟次数；若过于频繁，KVM 会**直接 unmap 该页**以避免持续模拟。

### 4.3 方案二：EPT / NPT / ARM stage-2（硬件二级翻译）

**机制**：硬件一次走查两级页表——GVA→GPA 由 guest 页表定义，GPA→HPA 由 EPT/NPT/stage-2 定义。

- **TLB 语义变化**：TLB 项需要同时按地址空间标识与二级页表指针打标（Intel：VPID + EPTP；AMD：ASID + NPT；ARM：VMID + VTTBR）。结果是 **guest 写 `cr3` 不再需要 flush TLB**——这是相对 shadow page table 的最大语义收益。
- **代价**：TLB 未命中时页表走查次数显著增加（guest 各级 + EPT 各级），靠 **paging-structure cache**（PML4/PDPTE/PDE cache）缓解；第一代 EPT 缺少 A/D 位，KVM 在 shadow page 的 role 中记录 `ad_disabled`，并在软件里模拟访问/脏位。
- **失效场景**（相对 shadow 路线）：
  - **内存超分**：EPT 不能表示"不存在但可换出"的 GPA，超分要靠 balloon/swap 配合，EPT violation 会成为热点。
  - **dirty logging / live migration**：靠写保护 EPT 来 trap 每一次写，写密集负载会产生大量 EPT violation。
  - **大页**：KVM 要实例化大 spte 必须满足 4 个条件（宿主页是大页、guest pte 至少同等大小、可写大页不得与任何写保护页重叠、guest 页必须完全落在单个 memory slot 内）。每个写保护页都会使其 memory slot 的 `->disallow_lpage` 计数递增，从而**阻止大页实例化**——所以 dirty logging 与内存热插拔会直接削弱大页带来的 TLB 覆盖收益。
  - **MMIO 缓存失效**：MMIO 信息被缓存在 leaf spte 里，用 generation number 校验；generation 溢出时会 zap 所有页。

### 4.4 方案三：半虚拟化 MMU（pvMMU）

| 实现 | 机制 | 代价 | 现状 |
|---|---|---|---|
| **Xen pvMMU** | PV guest 默认**直接写自己的页表**（页表页被 pin 住，`MMUEXT_PIN_L1_TABLE`/`MMUEXT_NEW_BASEPTR` 等 hypercall 管理基址切换）；对需要额外保护或非 PV 语义的 guest 则退回 **shadow mode** | 每次页表更新需要 hypercall；pin/unpin 有额外簿记 | Xen 的经典路径，HVM guest 不经此路径 |
| **KVM paravirt MMU** | 早期通过 paravirt MMU ops 让 guest 批量提交 pte 更新，减少 shadow 同步；后来在 EPT/NPT 平台上意义大幅下降 | 依赖 guest 内核配合（需 pv 支持） | x86 上已被 EPT/NPT 取代；**保留的是 pv TLB flush 等轻量协作**（如 `KVM_FEATURE_PV_TLB_FLUSH`）、steal time 等 |
| **嵌套下的"多层 MMU"** | L0 要么 shadow L1 的 EPT（shadow EPT page，KVM 用 `role.ept_sp` 标记），要么（有 nEPT 硬件时）直接让硬件走 L1 的 EPT | 无 nEPT 时相当于"在 2 级硬件上编码 3 级翻译" | KVM 两种路径都有 |

**何时 pvMMU 是赢**：guest 内核可改（PV guest），页表更新可批量成 hypercall，且宿主没有 EPT/NPT。**何时是输**：guest 不可改（二进制 hypervisor/OS），或硬件已有二级翻译——此时 pvMMU 的 hypercall 纯属额外开销。

### 4.5 三方案对照

| 维度 | Shadow page table | EPT / NPT / stage-2 | pvMMU |
|---|---|---|---|
| 位置 | 软件（hypervisor 维护） | 硬件（MMU 走查第二级） | 软件 + guest 协作 |
| 翻译级数 | 压成 1 级（GVA→HPA） | 2 级 | 通常 1 级（PV 直接写页表） |
| guest 页表改动 | 必须写保护 + trap 同步 | guest 页表自由改，无需 trap | guest 自己写，hypercall 通知 |
| `cr3` 写入 | 需要切换 shadow root + flush | TLB 按 EPTP/ASID 打标，通常无需 flush | 需要 hypercall 换基址 |
| 大页支持 | 受 guest/host 页大小与写保护约束 | 受 `->disallow_lpage` 等约束 | 取决于 guest 页表 |
| 内存超分 | 相对友好（页表逐项可控） | 差（无法表示可换出的 GPA） | 友好 |
| dirty logging | 靠 guest 页表写保护 | 靠 EPT 写保护（每次写都 violation） | 靠 guest 页表写保护 |
| 主要失效场景 | guest 频繁切/写页表、`INVLPG` 风暴、32 位 guest 的 quadrant 拆分 | 超分、dirty logging、大页被写保护页打散、EPTP 频繁切换 | guest 不可改、页表更新极频繁、已有 EPT/NPT |
| 代表实现 | KVM 传统 MMU、Xen shadow mode、nested 无 nEPT 时 | Intel EPT、AMD NPT、ARM stage-2、nested EPT | Xen PV、KVM 早期 paravirt MMU |

### 4.6 失效场景速查

| 场景 | shadow PT | EPT/NPT | pvMMU |
|---|---|---|---|
| guest 页表频繁切换（进程密集） | 差：每次 `cr3` 写都要换 root + 同步 | 好：TLB 打标，无需 flush | 中：换基址要 hypercall |
| guest 频繁写页表 | 差：写保护 + 模拟，靠 unsync 与 `write_flooding_count` 兜底 | 好：不 trap | 中：hypercall 次数上升 |
| 大页（2M/1G） | 中：受写保护与 memory slot 对齐约束 | 中：`->disallow_lpage` 与 dirty logging 会打散大页 | 好 |
| 内存超分 | 好 | 差（需 balloon/swap 配合） | 好 |
| dirty logging（迁移/快照） | 中 | 差（每次写都 EPT violation） | 中 |
| 嵌套（L2） | 最差：3 级翻译压 1 级 | 好（nEPT/nNPT 直通） | 不适用 |

---

## 五、性能数据

### 5.1 已核实的定量数据（Turtles, OSDI 2010）

**测试条件**：IBM 研究，Intel x86（无硬件嵌套支持时代），**KVM 同时作 L0 和 L1**，使用 multi-dimensional paging；kernbench/SPECjbb 的 CPU 列为占用率；netperf 使用单核、1Gb 链路，对比改造前后。

| 工作负载 | 配置 | 对比基线 | 开销 | 来源 |
|---|---|---|---|---|
| kernbench（编译） | KVM L0 / KVM L1 / L2 guest | 单层 guest | **+14.5%** | Turtles OSDI'10 |
| kernbench，扣除 VMREAD/VMWRITE 开销 | 同上 | 单层 guest | **+10.3%** | 同上 |
| SPECjbb | 同上 | 单层 guest | 得分 **-7.82%** | 同上 |
| SPECjbb，扣除 VMREAD/VMWRITE 开销 | 同上 | 单层 guest | 得分 **-6.3%** | 同上 |
| kernbench | **VMware Server 作 L1** | 单层 guest | **+14.98%** | 同上 |
| SPECjbb | VMware Server 作 L1 | 单层 guest | 得分 **-8.85%** | 同上 |
| L0 自身 CPU 占用 | kernbench，单层 vs 嵌套 | 单层 guest | **2.28% → 5.17%** | 同上 |
| 切换开销（VMCS 访问优化） | 去掉 VMREAD/VMWRITE trap | 优化前 | **13,000 → 2,000（-84.6%）** | 同上 |
| netperf 64B 消息 | L2 + multi-level device assignment | 裸机 900 Mb/s | **837 Mb/s（约 -7%）** | 同上 |
| netperf 64B 消息 | virtio-on-direct | 裸机 900 Mb/s | **469 Mb/s（-50%）** | 同上 |
| netperf 吞吐 | 单层 guest 全模拟网卡 | 裸机 940 Mb/s @20% CPU | 仅约 **25% 线速**，且 CPU 打满 | 同上 |
| netperf 吞吐 | virtio 一路到 L2 | 裸机 | 约 **75% 线速**，CPU 打满 | 同上 |
| netperf 吞吐 | multi-level device assignment | 裸机 | **线速 @60% CPU** | 同上 |
| 朴素 MMU 虚拟化 | 非 multi-dimensional paging 的做法 | 合理做法 | 部分负载**至少慢 3 倍** | 同上 |

**结论**：在**同代硬件、纯 CPU 负载**下，嵌套相对单层的开销是**个位数到十几个百分点**；但在**I/O 与中断密集**场景，代价可以到**几十个百分点**，且对实现方式（设备直通的层数）极其敏感。

### 5.2 其他定性结论（已核实）

- **AWS**：官方建议性能敏感或有严格延迟要求的客户"评估裸金属实例"，而不是用嵌套。AWS 未在该页给出嵌套开销的具体数字。
- **Xen**：官方 wiki 称对"许多常见场景"应当可靠且低开销，但同时把 AMD 标为 experimental、Intel 标为 tech preview，并因 L1→L0 DoS 风险**不建议生产使用**。

### 5.3 未能证实

- **现代硬件（Haswell 之后、AMD Zen、ARMv8.4-NV）上的嵌套开销百分比**：本稿未找到可交叉验证的、注明测试条件的公开数据。
- **ARMv8.4-NV vs trap-and-emulate 的定量对比**：未能证实。
- **"一次 L2 exit 触发 2–3 次 L0 exit" 这类具体倍数**：未能证实（Turtles 只给出 L0 CPU 占比 2.28%→5.17% 的观测）。
- **云厂商裸金属 vs 嵌套 VM 的性能对比数字**：未能证实。

---

## 六、对照表（方案 × 硬件前提 × 性能代价 × 适用场景）

| 方案 | 硬件前提 | 性能代价 | 适用场景 | 备注 |
|---|---|---|---|---|
| Intel nVMX + VMCS shadowing | Haswell(2013)+，`nested=1` | 消除 VMCS 访问 exit（-84.6% 切换成本）；仍有 entry/exit 放大 | 云上跑 KVM/Hyper-V、CI、Docker Desktop/WSL2 | 当前 x86 云上最成熟的路线 |
| Intel nVMX（无 shadowing） | Nehalem(2008)+ VMX/EPT | VMCS 访问每次 exit，嵌套开销显著更高 | 老平台 | 应尽量避免 |
| AMD nSVM + nested NPT | AMD-V + NPT | 与 Intel 同构的放大；clean bit 处理带来字段同步成本 | 同上（AMD 平台） | AVIC 在嵌套下的可用性受限 |
| 软件模拟（Turtles 式） | 无特殊要求 | CPU 负载 6–15%；朴素做法慢 3 倍 | 研究/兼容性验证 | 不是生产选项 |
| Xen nestedhvm | EPT + `hap=1` + `nestedhvm=1` | 官方称常见场景低开销，但不建议生产 | 实验、测试 | 存在 L1→L0 DoS 已知问题 |
| ARM trap-and-emulate | ARMv8.0 + 虚拟化扩展 | 每次 EL2/EL1 寄存器访问 trap | 存量 ARM 平台 | 代价高 |
| ARM VHE guest 原生访问 | ARMv8.1 VHE | 显著低于纯 trap | ARMv8.1+ 云平台 | cache 维护指令仍有 trap 冲突 |
| ARMv8.4 FEAT_NV2（`VNCR_EL2`） | ARMv8.4 + **FEAT_NV2** | 预期显著优于 trap（**无公开定量数据**） | 新 ARM 服务器 | 主线 KVM 只支持 NV2，2025 年随 Linux 6.16 合入 |
| 裸金属实例 | 无 hypervisor 或极薄 hypervisor | 无 exit 放大 | 性能敏感、自定义 hypervisor、CVM 需求 | AWS 官方推荐路径 |
| 裸金属 + DPU/SmartNIC | Nitro / MOC / 擎天等卸载卡 | I/O 与中断不占宿主 CPU | 云网络/存储密集、K8s 数据面 | 各厂商卸载边界需按官方文档核实 |
| 微 VM（Firecracker/Cloud Hypervisor） | KVM + 一层硬件虚拟化 | 约 5 μVM/核/秒创建率；无需嵌套 | AI Agent 沙箱、FaaS、多租户 CI | 见 `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md` |
| 容器 + 语言沙箱 | 无（纯软件） | namespace/cgroup/seccomp 的 syscall 路径开销 | 同租户内应用隔离 | 与硬件虚拟化正交，不产生 VM exit |

---

## 未决问题

1. **KVM nVMX / nSVM 的"不支持特性清单"需要逐条落到源码与 SDM 章节**：posted interrupt、VMFUNC、VMCS shadowing 是否暴露给 L1、L2 的 A/D 位、mode-based execution control 等。当前稿只有定性描述。
2. **现代硬件上嵌套开销的可靠数字缺失**：需要一条可在自有环境复现的基准（L0/L1/L2 三层，分别测 CPU、内存带宽、4K/顺序磁盘、网络 PPS、中断密集），否则"嵌套开销 X%"的说法无法引用。
3. **ARMv8.4-NV 的实际收益**：`VNCR_EL2` 把虚拟 EL2 寄存器重定向到内存后，哪些负载受益最大？与 trap-and-emulate 的差值是多少？
4. **AVIC / APICv 在嵌套下到底何时可用**：需要内核代码与提交记录的确切条件，以及不可用时对中断延迟的量化影响。
5. **机密计算与嵌套的组合边界**：TDX / SEV-SNP 是否允许 guest hypervisor 为其 L2 提供同等保护？现状与路线图需以 Intel/AMD 官方文档核实。
6. **设备侧第三级翻译的实际落地程度**：Intel VT-d Scalable Mode 的三级翻译、AMD IOMMU nested translation（HWPT-based vs vIOMMU-based 两条路线）、ARM SMMUv3 nested stage 的主线状态。
7. **dirty logging × 大页 × 嵌套三者的相互削弱**：`->disallow_lpage` 机制在多层场景下的累积效应缺少实测。

---

## 参考资料

访问日期均为 **2026-09-16**。

1. Linux 内核 KVM《Nested VMX》文档 — https://www.kernel.org/doc/Documentation/virtual/kvm/nested-vmx.txt
2. Linux 内核 KVM《The x86 kvm shadow mmu》文档 — https://www.kernel.org/doc/Documentation/virtual/kvm/mmu.txt
3. Muli Ben-Yehuda et al., *The Turtles Project: Design and Implementation of Nested Virtualization*, OSDI 2010 — https://www.usenix.org/legacy/events/osdi10/tech/full_papers/Ben-Yehuda.pdf
4. AWS 文档《Use nested virtualization to run hypervisors in Amazon EC2 instances》 — https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/amazon-ec2-nested-virtualization.html
5. Xen Wiki《Nested Virtualization in Xen》 — https://wiki.xenproject.org/wiki/Nested_Virtualization_in_Xen
6. KVM arm64 NV 补丁（`KVM: arm64: nv: Configure HCR_EL2 for nested virtualization`, v10 系列，2023） — http://lists.openwrt.org/pipermail/linux-arm-kernel/2023-May/833943.html
7. KVM arm64 嵌套补丁系列（`[PATCH v11 00/43] KVM: arm64: Nested Virtualization support (FEAT_NV2 only)`，标题即证明只支持 NV2） — https://yhbt.net/lore/linux-arm-kernel/67082409-f432-44b6-bf40-1af9b4b7b569@os.amperecomputing.com/
8. KVM/arm64 更新随 **Linux 6.16** 合并窗口合入的证据（kvmarm-6.16 merge tag / KVM pull request） — http://git.armlinux.org.uk/cgit/linux.git/log/?id=7f904ff6e58d398c4336f3c19c42b338324451f7&showmsg=1
9. Arm 官方寄存器文档《VNCR_EL2, Virtual Nested Control Register, EL2》 — https://developer.arm.com/documentation/ddi0595/2021-03/AArch64-Registers/VNCR-EL2--Virtual-Nested-Control-Register
10. Arm SMMUv3 架构规范 — https://documentation-service.arm.com/static/5f901081f86e16515cdc0919
11. AMD Architecture Programmer's Manual Volume 2（VMCB / NPT / clean bits） — https://www.amd.com/content/dam/amd/en/documents/processor-tech-docs/programmer-references/24593.pdf
12. Linux 补丁《iommu/amd: Introduce Nested Translation support》（AMD IOMMU 嵌套翻译） — https://patchew.org/linux/20250820113009.5233-1-suravee.suthikulpanit@amd.com/
13. Linux 补丁《SMMUv3 Nested Stage Setup (IOMMU part)》 — http://yhbt.net/lore/kvmarm/20211209154046.GQ6385@nvidia.com/T/
14. Intel® 64 and IA-32 Architectures Software Developer's Manual（VMX / EPT / VMCS shadowing） — https://www.intel.com/content/www/us/en/developer/articles/technical/intel-sdm.html
15. Intel VT-d Scalable Mode "three-stage address translation" 相关公开文献 — https://www.freepatentsonline.com/y2022/0350714.html
16. Firecracker 设计文档（KVM、jailer、seccomp、cgroup） — https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md
17. LKML RFC 补丁系列《Support nested SNP KVM guests on Hyper-V》（RMP table、MSR 形式 rmpupdate/psmash） — https://lkml.iu.edu/hypermail/linux/kernel/2301.2/09448.html
18. 本仓库既有笔记 — `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`、`docs/rust-kunpeng/cubesandbox-rust-analysis.md`

> **引用注意**：第 14、15 项为规范/专利索引页，本稿引用的是其描述的概念，未能逐页核对具体条款号；第 10、11 项为大部头 PDF，本稿只对其中嵌套/页表相关章节做了定位性引用；第 6–8 项为邮件列表/内核补丁归档。凡未逐条核对的，正文已相应标注「未能证实」或「待补」。
