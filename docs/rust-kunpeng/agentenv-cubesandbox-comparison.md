# AgentENV vs CubeSandbox 架构对比

两者都是"**rust-vmm + KVM MicroVM 的 AI Agent 安全沙箱**"，但在 **VMM 选型、CoW 路径、存储、集群、生态集成** 上有本质差异。

## 一、总览对比

| 维度 | AgentENV | CubeSandbox |
|---|---|---|
| 出身 | 清华 MADSys + 月之暗面（Kimi K3） | 腾讯云 |
| VMM | **Firecracker**（AWS，第三方 binary，极简） | **CubeHypervisor**（Cloud Hypervisor fork，自研，功能全） |
| CoW 路径 | **内存 CoW**（overlaybd 内存快照 + fork） | **磁盘 CoW**（XFS FICLONE reflink） |
| 存储 | overlaybd 分层镜像 + ublk（按需加载） | CubeCoW（XFS reflink）+ QCOW2/VHDX |
| 集群 | **完整分布式**（Go gateway/scheduler + Rust P2P + OSS/S3） | **单节点为主**（CubeShim 接 containerd） |
| 生态集成 | 独立平台（E2B API） | **containerd 生态**（Shim v2 + Kata agent） |
| 指标 | boot/resume <50ms、pause <100ms、fork ≤16 子 | 冷启动 <60ms、单实例 <5MB |
| Rust 占比 | 主体 Rust + Go 控制面 | ~48% Rust + Go 交互面 |

## 二、相同点（Common）

1. **都基于 rust-vmm + KVM**：kvm-ioctls / vm-memory / virtio-queue 同一套底层库，与 Firecracker/crosvm 共享生态。
2. **都 E2B 兼容**：API 层（Axum）暴露 E2B 协议，替换 URL 即可迁移。
3. **Rust 是核心语言**：控制面（API/Orchestrator/VMM 管理/快照）都是 Rust。
4. **都有 snapshot/restore**：快照是核心原语（AgentENV 的 snapshot/fork，CubeSandbox 的 CubeCoW）。
5. **都面向 AI Agent 沙箱**：强隔离（独立内核）+ 高密度（microVM 轻量）的同一目标。

## 三、不同点（核心，分五维度）

### 1. VMM 选型

| | AgentENV | CubeSandbox |
|---|---|---|
| VMM | Firecracker（**外部 binary**，只 vendor API client） | CubeHypervisor（**Cloud Hypervisor fork，自研**，作为库链接） |
| 特性 | 极简（3 virtio 设备） | 功能全（virtio-blk/net/fs、VFIO、热迁移、TDX/Hyper-V） |
| 快照/restore | 依赖 AgentENV 自己实现（overlaybd 内存） | **VMM 自带** snapshot/restore |

### 2. CoW / 快照路径（最本质的差异）

| | AgentENV | CubeSandbox |
|---|---|---|
| 机制 | **内存 CoW**：脏页追踪 + `process_vm_readv` 拷脏页 → overlaybd 层 → `MAP_PRIVATE` 惰性 fault | **磁盘 CoW**：XFS `FICLONE` reflink，O(1) 元数据级克隆，零数据搬运 |
| 粒度 | 页级内存快照 | 块级磁盘快照 |
| 额外能力 | **fork**（运行中沙箱 fork 出 ≤16 个子沙箱，内存 CoW 继承） | 无 fork（只有快照/克隆） |
| 文件系统依赖 | 无（overlaybd 是块设备层） | **依赖 XFS**（FICLONE 是 XFS 特性） |

### 3. 存储

| | AgentENV | CubeSandbox |
|---|---|---|
| 镜像 | OCI → overlaybd 分层（registryfs_v2 **按需 HTTP 拉取**） | QCOW2/VHDX 本地磁盘 |
| 块设备 | ublk（用户态块设备 + io_uring） | CubeCoW（reflink 快照） |
| 远程分发 | ✅ OSS/S3 + P2P（iroh）跨节点 | ❌ 本地为主 |

### 4. 集群 / 生态

| | AgentENV | CubeSandbox |
|---|---|---|
| 集群 | **完整分布式控制面**：Go Gateway（路由）+ Scheduler（放置/发现）+ Rust P2P + OSS/S3 | **单节点**：CubeShim 实现 containerd Shim v2，接入容器运行时生态 |
| 扩展 | 多节点调度 | 靠容器编排（K8s/containerd） |
| guest agent | envd（E2B 开源，**Go**）+ Rust 客户端 | cube-agent（Kata Containers agent fork，**Rust**，tokio-vsock） |

### 5. 预热 / 快启动手段

- AgentENV：**warm-pool**（Firecracker 进程预热池）+ 内存快照 resume + OCI 按需加载。
- CubeSandbox：**CubeCoW reflink**（O(1) 磁盘克隆）+ VMM 自带 snapshot/restore。

## 四、架构图对比

```
AgentENV（分布式，内存 CoW）          CubeSandbox（单节点，磁盘 CoW）
───────────────                      ───────────────
Gateway(Go) ─ Scheduler(Go)          Cubelet(Go) ──CGO──► CubeCoW(Rust, reflink)
   │                                  │
   ▼                                  ▼
Node(Rust):                          CubeShim(Rust, containerd Shim v2)
 API ─ Orchestrator ─ Firecracker     ─ CubeHypervisor(Cloud Hypervisor fork)
        └─ snapshot/overlaybd+ublk    └─ cube-agent(Rust, guest)
        └─ P2P(iroh) ─ OSS/S3           （vsock 通信）
```

## 五、一句话总结

| | AgentENV | CubeSandbox |
|---|---|---|
| **本质** | "**分布式 + 内存 CoW + fork**"的 Agentic RL 环境平台 | "**单节点 + 磁盘 CoW + containerd 集成**"的容器式安全沙箱 |
| **差异化武器** | fork + 内存快照 + 跨节点分发（P2P/OSS） | XFS reflink O(1) 磁盘快照 + 容器生态兼容 |

**共同点**：rust-vmm + KVM MicroVM、E2B 兼容、Rust 控制面、snapshot/restore。
**核心差异**：AgentENV 走"**内存级 CoW（可 fork）+ 分布式**"，服务大规模 RL 训练；CubeSandbox 走"**磁盘级 CoW（reflink）+ containerd 单节点集成**"，服务容器化安全沙箱。二者代表了"microVM 快照"的两条技术路线——**内存快照 vs 磁盘 reflink**。
