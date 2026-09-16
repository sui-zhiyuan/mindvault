# 虚拟化主题调研 · 任务分解与索引

> 建立：2026-09-16 ｜ 状态：进行中
> 产出约定：中间稿在 `research/virtualization/`，汇总成稿在 `docs/virtualization/`（按 AGENTS.md 模板）

## 一、总目标

产出一份虚拟化主题的系统性调研，回答四个问题：

1. 虚拟化的发展过程；什么是硬件虚拟化；之前的软件虚拟化性能损失大概多少。
2. 多层硬件虚拟化有哪些方案？解决了什么问题。
3. 硬件虚拟化依赖硬件哪些能力（x86 Intel / x86 AMD / ARM / Kunpeng）。
4. 有哪些公司和产品（Hyper-V / VMware / VirtualBox / Azure / AWS / 阿里云 / 华为云 …）以及沙箱。

## 二、任务分解

### 阶段 0 · 准备
- 0.1 扫 `docs/` 存量笔记，确定复用点 ✅
- 0.2 确认输出路径与成稿规范 ✅
- 0.3 建立中间稿目录与任务清单 ✅

### 阶段 1 · 发展过程 + 硬件虚拟化定义 + 软件虚拟化开销（主题 A/B）→ `01-history-and-overhead.md`
- 1.1 演进时间线：CP/CMS(1960s) → Popek & Goldberg(1974) → 二进制翻译(VMware/Disco) → 半虚拟化(Xen 2003) → 硬件辅助(VT-x 2005 / SVM 2006) → KVM 入主线(2007) → 设备与 IO 虚拟化(virtio / SR-IOV / VT-d) → microVM(2018) → 机密计算(SEV/TDX)
- 1.2 概念辨析：模拟 vs 全虚拟化(二进制翻译) vs 半虚拟化 vs 硬件辅助虚拟化 vs OS 级虚拟化
- 1.3 硬件虚拟化的**可判据定义**（新增特权模式 + 陷入-模拟由硬件完成；Popek & Goldberg 三条件与敏感指令）
- 1.4 软件虚拟化性能损失的**分类量化**（CPU 密集 / 特权指令与 syscall 密集 / I/O 密集 / 内存 MMU），每个数字标注测试条件与出处

### 阶段 2 · 多层硬件虚拟化方案（主题 C）→ `02-multilayer-virtualization.md`
- 2.1 "多层"的三种含义辨析：嵌套虚拟化 / 多级地址翻译 / 多级虚拟化栈
- 2.2 嵌套虚拟化方案族：Intel VMCS shadowing、AMD nested NPT、ARM NV/NV2、纯软件模拟、云厂商裸金属+DPU
- 2.3 解决什么问题 + 代价与限制（每层嵌套的 exit 放大、功能子集）
- 2.4 内存虚拟化方案对比：shadow page table vs EPT/NPT vs 半虚拟化 MMU
- 2.5 对照表：方案 × 硬件前提 × 性能代价 × 适用场景

### 阶段 3 · 硬件能力依赖（主题 D）
- `03-hw-capabilities-x86.md`：Intel VMX 全能力清单（VMCS/EPT/VPID/APICv/Posted Interrupt/VMFUNC/HLAT/VT-d/SGX/TDX）、AMD SVM（VMCB/NPT/AVIC/VGIF/GMET/SEV 系），均按「能力 → 首次引入代际 → 作用 → 缺失时代价」组织
- `04-hw-capabilities-arm-kunpeng.md`：ARM EL0~EL3、Stage-2、VHE(8.1)、嵌套(8.4 NV/NV2/VNCR_EL2)、GICv2/v3/v4、SMMU、PMU/SPE/MTE/MPAM 虚拟化、RME(CCA)；**Kunpeng 920/950 的实际能力边界（含"是否支持硬件嵌套"这一高风险项）**、openEuler StratoVirt、BoostKit
- 3.5 能力矩阵（能力项 × Intel / AMD / ARM / Kunpeng）

### 阶段 4 · 厂商产品 + 沙箱（主题 E）
- `05-vendors-and-products.md`：类型学（Type-1/Type-2/KVM 型）；VMware（含 Broadcom 收购后变化）、Microsoft（Hyper-V/WHP/WSL2/VBS/Azure Boost）、Oracle VirtualBox、Xen 系、QEMU、Proxmox；AWS Nitro+Firecracker、Azure、阿里云神龙、华为云擎天、腾讯云 CubeSandbox、Google Cloud
- `06-sandboxes.md`：按隔离边界分层（硬件级机密计算 / 硬件辅助虚拟机级 / 用户态内核 / OS 级 / 应用与语言级 / 网络数据级）+ 沙箱 vs 虚拟化 对照 + AI Agent 沙箱
- `07-existing-notes-map.md`：仓库存量笔记（CubeSandbox / AgentENV / sandbox_research）与虚拟化的关系映射

### 阶段 5 · 汇总与交付
- 5.1 交叉对比总表（开销 / 隔离强度 / 启动时间 / 硬件依赖 / 典型产品）
- 5.2 趋势判断：DPU/IPU 卸载、机密计算、microVM 化、Rust VMM、ARM 服务器虚拟化
- 5.3 与存量笔记互链
- 5.4 按 AGENTS.md 模板成稿 + 登记 `docs/SUMMARY.md`

## 三、中间稿清单

| 文件 | 主题 | 状态 |
|---|---|---|
| `00-index.md` | 任务分解与索引（本文件） | ✅ |
| `01-history-and-overhead.md` | 发展过程、硬件虚拟化定义、软件虚拟化开销 | ✅ |
| `02-multilayer-virtualization.md` | 多层/嵌套虚拟化方案 | ✅ |
| `03-hw-capabilities-x86.md` | Intel / AMD 硬件能力 | ✅ |
| `04-hw-capabilities-arm-kunpeng.md` | ARM / 鲲鹏硬件能力 | ✅ |
| `05-vendors-and-products.md` | 厂商与产品 | ✅ |
| `06-sandboxes.md` | 沙箱全景 | ✅ |
| `07-existing-notes-map.md` | 存量笔记与虚拟化的关系 | ✅ |

## 四、方法

- **先分解后调研**：本文件即分解产物；六路调研互不依赖，可并行。
- **联网优先**：涉及产品现状、硬件代际、性能数字的部分，以官方文档 / 原始论文 / 内核文档为准，不依赖模型记忆。
- **数字必须有前提**：任何性能百分比都要写清测试条件（负载类型、是否半虚拟化、CPU 代际），拒绝"损耗 5%~20%"式无依据断言。
- **不确定就标注**：找不到可靠来源的结论显式写「未能证实」；架构版本推断必须标注「推断/待证实」。
- **风险项人工交叉验证**：① 鲲鹏是否支持硬件嵌套虚拟化；② 软件虚拟化的性能数字。这两项由主调研者单独复核，不只依赖单一路径。
- **不重复存量笔记**：`docs/rust-kunpeng/` 下已有 CubeSandbox / AgentENV / sandbox_research 分析，本次只做"与虚拟化关系"的映射与引用。

## 五、参考资料

- 仓库存量笔记：`docs/rust-kunpeng/sandbox_research.md`、`cubesandbox-rust-analysis.md`、`agentenv-cubesandbox-comparison.md`、`docs/tools-and-tips/memory-access-summary.md`
- 各分册参考资料见各自文件（访问日期 2026-09-16）
