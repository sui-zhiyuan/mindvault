# 汇总层：因果链、趋势判断与答案速览

> 建立：2026-09-16 ｜ 类型：中间稿（主调研者自撰的汇总层）
> 说明：本文件是**跨分册的合成**，不重复各分册的技术细节。分册（01~06）负责事实与数据，本文件负责把它们串成因果链。
> 标注：`【待回填】`表示该处需要等对应分册定稿后填入具体数字或产品名。

---

## 一、把四个问题串成一条因果链

用户问的四件事看似并列，实际是**一条有先后依赖的技术史**：硬件能力决定了 VMM 能怎么做，
VMM 的架构又决定了上层能长出什么产品形态。

```
① 问题：x86 天生不可虚拟化（敏感指令不陷入）
        ↓ 软件绕行（1999-2005）
② 方案 A：二进制翻译（VMware ESX）      方案 B：半虚拟化（Xen 2003）
   代价：翻译整个 guest 内核 + 影子页表      代价：改 guest 内核（但开销"几个百分点"）
        ↓ 硬件补课（2005-2008）
③ VT-x / SVM：CPU 陷入-模拟由硬件做 → 不再需要翻译
   EPT / NPT：二级地址翻译由硬件做 → 不再需要影子页表
        ↓ 开销降到个位数 %，虚拟化成为默认底座（2008-2015）
④ virtio → vhost → SR-IOV → VFIO 直通；APICv / posted interrupt / vGIC
        ↓ 两个副产品
⑤ 副产品一：**多层化**——云上要"VM 里跑 VM"（开发/CI/VDI/WSL2/Docker Desktop）
   → 嵌套虚拟化：Intel VMCS shadowing / AMD nested NPT / ARM FEAT_NV2
   → 或者绕开：裸金属实例 + DPU 卸载
⑥ 副产品二：**轻量化**——KVM 把 guest 内存做成 VMM 的用户态 mmap
   → 内存快照/克隆退化成"对进程做页级 CoW" → microVM（Firecracker 2018）→ Agent 沙箱产业
        ↓ 隔离需求再升级（2016-2025）
⑦ 机密计算：SEV → SEV-SNP / TDX / ARM CCA（RME）
        ↓ 反讽的终点
⑧ 硬件虚拟化足够快之后，云厂商把"虚拟化本身"卸载到专用芯片：
   AWS Nitro / 阿里云神龙 MOC / 华为云擎天 DPU —— 虚拟化开销趋近于"不占主 CPU"
```

**这条链的读法**：软件虚拟化的性能损失是**起点问题**（①→②），硬件虚拟化能力是**解法**（③→④），
而"多层虚拟化"和"沙箱"是**解法带来的两个分支**（⑤⑥），机密计算和 DPU 卸载是**当前的前沿**（⑦⑧）。
所以四问不是四个独立话题，而是**同一棵树的问题-解法-衍生关系**。

---

## 二、四个问题的答案速览

### 问题 1：发展过程 / 什么是硬件虚拟化 / 软件虚拟化损失多少

| 子问题 | 一句话答案 | 详细出处 |
|---|---|---|
| 发展过程 | 1960s IBM CP/CMS 起源 → 1990s x86 靠软件绕（二进制翻译 / 半虚拟化）→ 2005-2008 硬件补课（VT-x/SVM + EPT/NPT）→ 2010s 设备与中断虚拟化 → 2018+ microVM → 2020s 机密计算与 DPU 卸载 | 01 |
| 什么是硬件虚拟化 | **CPU 硬件新增的虚拟化模式**（Intel root/non-root + VMCS、AMD host/guest + VMCB、ARM EL2），使「陷入-模拟」由硬件完成而非软件翻译 —— 关键判据是**新增特权模式 + 硬件级陷入**，而不是"有没有虚拟机" | 01 |
| 软件虚拟化损失 | **不是一个数**：半虚拟化（Xen）自述"**至多几个百分点**"（原文 *at most a few percent*）；二进制翻译 + 影子页表的全虚拟化显著更高，代价集中在**页表/地址空间更新密集**与**特权指令密集**负载。`【待回填】`具体分档数字由 01 稿补 | 01 / 08 已验证部分 |

### 问题 2：多层硬件虚拟化有哪些方案 / 解决什么问题

- **"多层"要先辨析三义**：①嵌套虚拟化（L0/L1/L2）、②多级地址翻译（GVA→GPA→HPA + 设备侧 IOMMU）、③多级虚拟化栈（KVM→VMM→容器）。
- **方案族**：Intel VMCS shadowing + nested EPT；AMD nested NPT + VMCB clean bits；ARM FEAT_NV2（ARMv8.4，HCR_EL2.NV/NV2 + VNCR_EL2）；纯软件模拟；以及**云厂商的绕开路线**（裸金属 + DPU）。
- **解决什么**：云上跑 KVM / Docker Desktop / WSL2、CI 里跑模拟器或测试 hypervisor、VDI 与教学实验、Kata/gVisor 的分层部署。
- **代价**：每加一层嵌套，VM-exit 会被放大，且暴露的功能是子集。`【待回填】`由 02 稿给出量级。
- **ARM 侧的硬约束（本次已验证）**：arm64 的 KVM 嵌套虚拟化**只支持 FEAT_NV2（ARMv8.4）**，且直到 **Linux 6.16（2025）**才合入主线 —— 比 x86 晚了十年以上。

### 问题 3：硬件虚拟化依赖哪些硬件能力

| 体系结构 | 核心能力清单 | 关键结论 | 出处 |
|---|---|---|---|
| Intel x86 | VMX/VMCS、root↔non-root 切换、EPT、VPID、Unrestricted Guest、APICv、Posted Interrupt、MSR/IO bitmap、VMFUNC、VMCS shadowing、HLAT、VT-d、SR-IOV、SGX/TDX | `【待回填】`代际与缺失代价由 03 稿补 | 03 |
| AMD x86 | SVM/VMCB、intercept 分类、ASID、NPT、AVIC、VGIF、GMET、SEV/SEV-ES/SEV-SNP | `【待回填】`由 03 稿补 | 03 |
| ARM | EL0~EL3、HCR_EL2、Stage-2（VTTBR_EL2）、VHE(8.1)、嵌套(8.4 FEAT_NV2)、GICv2/v3/v4 虚拟化、SMMU stage-2、PMU/SPE/MTE/MPAM 虚拟化、RME(CCA) | `【待回填】`由 04 稿补 | 04 |
| **Kunpeng** | **官方规格页证实鲲鹏 920 为 ARMv8.2** → 具备 EL2/Stage-2/VHE，**不具备 FEAT_NV2 硬件嵌套** | **已验证**：鲲鹏 920 上"VM 里跑 hypervisor"没有硬件加速路径；官方规格页**未列出任何虚拟化特性**（属架构标配，不作为卖点） | 04 / **08（已验证）** |

### 问题 4：公司与产品 / 沙箱

- **类型学**：Type-1（ESXi、Xen、Hyper-V）/ Type-2（VirtualBox、Workstation）/ **KVM 型**（内核模块 + QEMU 用户态）。
- **厂商**：VMware（ESXi/vSphere，Broadcom 收购后授权变化）、Microsoft（Hyper-V/WHP/WSL2/VBS/Azure）、Oracle VirtualBox、Xen 系（XCP-ng）、QEMU、Proxmox。`【待回填】`由 05 稿补全。
- **云厂商自研架构**：AWS Nitro + Firecracker、阿里云神龙（MOC）、华为云擎天（DPU + StratoVirt）、腾讯云 CubeSandbox、Google（KVM + gVisor + Confidential VM）。`【待回填】`由 05 稿补全。
- **沙箱**：按隔离边界分六层 —— 硬件级机密计算 / 硬件辅助虚拟机级（Firecracker、Cloud Hypervisor、CubeSandbox）/ 用户态内核（gVisor、Kata）/ OS 级（namespace+seccomp+Landlock）/ 应用与语言级（浏览器、WASM）/ 网络数据级。`【待回填】`由 06 稿补全对照表。

---

## 三、趋势判断（五条）

1. **虚拟化的成本中心已经从 CPU 转移到"多出来的那一层"**。
   一旦 EPT/NPT 把 CPU 与内存虚拟化压到个位数开销，剩下的开销就集中在 I/O 路径与中断投递上；
   于是 virtio → vhost → SR-IOV → VFIO 直通 → DPU 全卸载，本质是**不断把 VMM 的工作挪出走**。
   → 判断：**未来"虚拟化开销"的讨论将主要等于"数据面卸载程度"的讨论**。

2. **"轻量虚拟化"是虚拟化能力成熟后的反哺，而不是对虚拟化的否定**。
   microVM（Firecracker/CubeSandbox）之所以能秒起、能快照、能 fork，恰恰因为 KVM 把 guest 内存做成了
   普通用户态映射，使得快照退化为对进程做页级 CoW（详见 `07-existing-notes-map.md`）。
   → 判断：**容器式体验 + 虚拟机级隔离**会成为 Agent/Serverless 场景的默认形态。

3. **ARM 服务器虚拟化的功能差距正在关闭，但时间差要以"十年"计**。
   x86 的嵌套虚拟化在 2013 年（Haswell VMCS shadowing）就已可用；arm64 的 KVM 嵌套直到 **Linux 6.16（2025）**
   才落地且要求 **ARMv8.4 FEAT_NV2**。鲲鹏 920（ARMv8.2）因此**不在硬件嵌套的支持范围内**。
   → 判断：**在 ARM/鲲鹏上规划"嵌套"类场景（云上跑 Docker Desktop、CI 跑 hypervisor、VDI）时必须先确认芯片代次**，
   不能按 x86 的默认假设推进。

4. **隔离等级的竞争从"共享内核 vs 独立内核"转向"谁在 TCB 里"**。
   虚拟机解决了内核共享问题，但 hypervisor 与宿主机内核仍在 TCB 内；
   SEV-SNP / TDX / ARM CCA(RME) 把宿主机也移出信任边界。
   → 判断：**机密计算会成为多租户云与 Agent 执行环境的差异化卖点**，且它会**反过来强化**虚拟化（而不是替代它）。

5. **虚拟化技术栈正在 Rust 化，且与 ARM 服务器化同向发生**。
   rust-vmm crate 生态 + Firecracker / Cloud Hypervisor / crosvm / **StratoVirt（openEuler）** / CubeSandbox
   共享同一套底层库；CubeSandbox v0.5 已实现 ARM64 全栈。
   → 判断：**这是鲲鹏生态"参与上游"成本最低的切入点之一**（为 rust-vmm 的 ARM64 后端做鲲鹏特定优化，
   比从零造 VMM 划算得多）。

---

## 四、待回填清单（分册定稿后填）

- [ ] `01`：软件虚拟化开销的**分档数字表**（CPU 密集 / 特权指令密集 / I/O 密集 / MMU），含 Adams & Agesen ASPLOS 2006 的原文数字（`08` 已标为"待取得"）。
- [ ] `02`：嵌套虚拟化的**性能开销量级**与各方案的功能子集清单。
- [ ] `03`：Intel / AMD 各项能力的**首次引入代际**与缺失时代价。
- [ ] `04`：ARM 各能力对应的**架构版本**；鲲鹏 950 的架构版本（决定其是否首次具备 FEAT_NV2）。
- [ ] `05`：Broadcom 收购后 VMware 产品线与授权的**最新时间线**；各云厂商卸载架构的现状。
- [ ] `06`：沙箱六类的**启动延迟 / 内存开销 / 逃逸风险**对照数字。

## 五、参考资料

- 分册：`01-history-and-overhead.md` ~ `06-sandboxes.md`
- 已验证结论：`08-verification-notes.md`
- 存量笔记映射：`07-existing-notes-map.md`
- 任务分解：`00-index.md`
