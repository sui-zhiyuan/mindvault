# 存量笔记映射：CubeSandbox / AgentENV / sandbox_research 与虚拟化的关系

> 建立：2026-09-16 ｜ 类型：中间稿（关系映射，不重复原笔记内容）
> 源笔记：`docs/rust-kunpeng/sandbox_research.md`、`docs/rust-kunpeng/cubesandbox-rust-analysis.md`、
> `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`、`docs/rust-kunpeng/rust-application-patterns.md`

## 一、结论先说

**CubeSandbox 与 AgentENV 不是虚拟化的替代品，而是虚拟化的消费者** —— 它们属于"用硬件辅助虚拟机做隔离"这一类沙箱（MicroVM 安全沙箱）。沙箱是**目的**（安全隔离 + 高密度），虚拟化是**手段**（隔离边界的实现方式）。

因此它们与本次调研四问的关系是：**同一个技术栈的根与枝**。

```
硬件虚拟化能力（T3/T4）→ 多层虚拟化栈（T2）→ 沙箱产品形态（T6）
                                        ↑
                              CubeSandbox / AgentENV 落在这里
```

## 二、在虚拟化栈中的位置

```
① 硬件            Intel VT-x+EPT / AMD-V+NPT / ARM EL2+Stage-2 / SMMU·VT-d / TDX·SEV-SNP
                  ↑ 硬件虚拟化能力（对应调研主题 D）
② 内核 hypervisor  KVM（kvm.ko + kvm-intel / kvm-arm64）     ← 两者共同的地基
                  ↑ 由内核提供的硬件虚拟化抽象（/dev/kvm ioctl）
③ 用户态 VMM       AgentENV: Firecracker（AWS，外部 binary，只 vendor API client）
                  CubeSandbox: CubeHypervisor（Cloud Hypervisor v28 fork，~30 crate，作为库链接）
                  ↑ virtio 设备模型、VFIO 直通、snapshot/restore、热迁移
④ guest 内核       microVM 内的独立 Linux 内核               ← 隔离边界所在
                  ↑
⑤ guest 内运行时   AgentENV: envd (Go, E2B 开源) / CubeSandbox: cube-agent (Rust, Kata agent fork)
                  + rustjail (OCI jail) + cgroups-rs         ← 又一层容器隔离
                  ↑
⑥ 业务             AI Agent 代码 / RL 训练环境
```

**关键观察**：两者都**不自己实现虚拟化**。真正做硬件虚拟化的是 ② 层的 KVM；它们的工作集中在 ③ 层 VMM 的封装取舍，以及 ③~⑥ 之间的编排、CoW、快照与生命周期管理。这正好是调研主题"多层"的**第三种含义（多级虚拟化栈）**的活样本。

## 三、用到的虚拟化机制映射

| 虚拟化机制 | 在两者中的具体体现 | 归属调研主题 |
|---|---|---|
| 硬件辅助 CPU 虚拟化 | KVM 运行 microVM，依赖 VT-x/AMD-V（x86）或 EL2（ARM） | T1 定义 / T3 / T4 |
| 二级地址翻译 EPT/NPT | guest 内存即 VMM 用户态 mmap（rust-vmm `vm-memory`）；脏页跟踪走 KVM dirty log（Intel PML / A/D 位） | T2 多级翻译 |
| 半虚拟化 virtio | Firecracker 仅 3 个 virtio 设备（极简）；CubeHypervisor 用 virtio-blk/net/fs/console；vsock 走 virtio-vsock | T1 开销（"为什么用 virtio 而不是模拟设备"） |
| 设备直通 VFIO | CubeHypervisor 保留 VFIO（`vfio-ioctls`），需要 IOMMU/VT-d/SMMU 才能安全直通 | T2 设备侧第三级翻译 / T3 / T4 |
| 快照 / restore | CubeSandbox 由 VMM 自带；AgentENV 自己实现内存快照 | **KVM 架构的直接红利**（见下） |
| 机密计算后端 | CubeHypervisor 支持 Intel TDX 与 Microsoft Hyper-V（`mshv-bindings`/`mshv-ioctls`）后端 | T3 能力清单 |
| ARM64 全栈 | CubeSandbox v0.5 已支持 ARM64（hypervisor / shim / agent / eBPF 网络） | **T4 鲲鹏场景的直接落点** |
| 半虚拟化优化 | KVM 的 Hyper-V enlightenments、pvclock，降低 guest 特权操作成本 | T1 "特权指令密集为何慢"的反面 |

## 四、为什么"microVM 快照/CoW"是 KVM 架构的红利

这是把沙箱产品与硬件虚拟化能力连起来的关键一环：

- **KVM 把 guest 物理内存做成 VMM 进程的普通用户态映射**（`vm-memory` crate 封装的就是这个 mmap）。于是"给运行中的 VM 做内存快照 / 克隆"退化成"对一个进程的地址空间做页级 CoW"。
- AgentENV 的 **fork（≤16 子沙箱）** 正是这条路径：脏页追踪 → `process_vm_readv` 拷脏页 → overlaybd 层 → `MAP_PRIVATE` 惰性 fault。
- 换成 **Xen 的 dom0 模型**或 **Hyper-V 的微内核模型**，guest 内存不由 VMM 进程直接持有，这条路径不成立。**这是"hypervisor 架构选择"直接决定"上层的产品能力"的例子**——很有说明力。
- 对比之下，CubeSandbox 的 `XFS FICLONE` reflink 快照**不在虚拟化层**，而在文件系统层（块级、O(1) 元数据克隆），属于另一条技术路线。

> 一句话：**AgentENV 的 CoW 在虚拟化层（内存），CubeSandbox 的 CoW 在文件系统层（磁盘）。**

## 五、与相邻沙箱类别的边界

| 方案 | 隔离边界 | 是否共享宿主内核 | 是否用硬件虚拟化 | 定位 |
|---|---|---|---|---|
| 普通容器（runc） | namespace + cgroup + capabilities | ✅ 共享 | ❌ | 最轻，隔离最弱 |
| gVisor | 用户态内核拦截 syscall | ❌（但不靠硬件虚拟化，systrap 平台） | ❌（可选 KVM 平台） | 邻居，不同路线 |
| **CubeSandbox / AgentENV** | **独立 guest 内核** | ❌ | ✅ **KVM** | **本文对象** |
| VBS / 机密计算 | 硬件 TCB（TDX / SEV-SNP / CCA） | ❌ | ✅ + 硬件信任根 | 上探（CubeHypervisor 已留 TDX 后端） |

**代价交换很清楚**：容器启动约 10ms 量级、内存开销近零，但隔离弱；microVM 冷启动 <50~60ms、单实例 <5MB，换来独立内核与抗内核漏洞逃逸。

**因此这两份笔记的真正主题其实是"虚拟化开销工程"** —— AgentENV 的 fork/内存快照与 CubeSandbox 的 reflink，本质都是在**把虚拟化引入的那几十毫秒和几 MB 的开销再压回去**。

## 六、对本次调研的用法

| 汇总稿章节 | 用法 |
|---|---|
| 主题 C（多层虚拟化） | 作为"多级虚拟化栈"的实例；不重写，直接引用本仓库笔记并标注来源 |
| 主题 E（沙箱类型学） | 归入"硬件辅助虚拟机级"一档，与 gVisor、纯容器并列对比 |
| 主题 D（硬件能力） | 作为"能力 → 产品"的落点：EPT/NPT 与 guest 内存 mmap 语义 → 快照/fork；IOMMU → VFIO 直通；TDX → 机密计算后端 |
| Kunpeng 相关 | CubeSandbox v0.5 的 ARM64 全栈支持，是"rust-vmm 在 ARM 上生产可用"的正面证据 |

## 七、需要标记的不确定项

- 两份笔记中的性能指标（AgentENV boot/resume <50ms、pause <100ms、CubeSandbox 冷启动 <60ms、单实例 <5MB）来自**项目自述/原笔记**，本次未独立复测；汇总稿引用时须标注来源为项目宣称值。
- AgentENV 的 fork 上限（≤16 子沙箱）是否为产品硬限制还是当前实现约束，未证实。
- CubeSandbox 的 ARM64 支持成熟度（v0.5 全栈）需核实是否存在平台特定限制（尤其 vGIC / SMMU 直通相关）。

## 八、参考资料

- 本仓库笔记（访问日期 2026-09-16）：
  - `docs/rust-kunpeng/sandbox_research.md`
  - `docs/rust-kunpeng/cubesandbox-rust-analysis.md`（第 118–199 行 Hypervisor 层、第 379–381 行 ARM64 支持）
  - `docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`
  - `docs/rust-kunpeng/rust-application-patterns.md`（第 45、135、151 行）
- 外部：rust-vmm 组织 crate（`kvm-ioctls`、`vm-memory`、`virtio-queue`）、Cloud Hypervisor、Kata Containers、Firecracker 官方文档（详见 `06-sandboxes.md`）
