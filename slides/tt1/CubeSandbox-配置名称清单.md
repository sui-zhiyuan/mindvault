# CubeSandbox 测试规格清单 —— 配置名称建议

> **用途**：测试规格清单的配置名称建议稿，供定稿用。配套文档见 [`CubeSandbox-交接与讨论结果.md`](CubeSandbox-交接与讨论结果.md)。
> **状态**：**待用户定稿**（增删、分组、以及区分「必须按规格配好」与「只需核对记录」）。
> **更新**：2026-09-29

标记含义：**★** = 现有清单里已有一项（建议拆细或改名）；**＋** = 建议新增。

---

## A. 平台与硬件

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| 服务器型号 | 整机散热 / 供电 / 拓扑不同，同芯片不同机器结果不同 | ＋ |
| CPU 型号（完整 SKU 串） | 「鲲鹏 950」有两个 SKU，核心数相差一倍 | ＋ |
| CPU 架构 | x86_64 / aarch64，决定所有后续对比 | ＋ |
| 插槽数 × 每插槽物理核数 | 决定总核数与内存通道数 | ＋ |
| 总物理核数 | 并发能力的上限 | ★ 核数 |
| SMT / 超线程（要求值 + 实测值） | 线程数可差一倍，直接改变并发甜点 | ★ 超线程 |
| 逻辑核数 | 调度器看到的并行度 | ＋ |
| NUMA 节点数 / 插槽（BIOS 侧） | 远端内存占比不同 | ★ NUMA |
| NUMA 拓扑实测（`numactl -H`） | 验证 BIOS 设置是否真的生效 | ＋ |
| 内存容量 | 决定可承载的沙箱密度 | ★ 内存规格 |
| 内存条数 × 容量 × 代次 × 速率 | 带宽与容量的基础 | ★ 内存规格 |
| 内存通道填充数 | 通道未插满会腰斩带宽 | ＋ |
| 内存 rank / 每通道条数 | 2DPC 会降频 | ＋ |
| ECC 开关 | 影响带宽与稳定性 | ＋ |
| 实测内存带宽（STREAM） | 把「带宽差异」从「CPU 差异」里分离出来 | ＋ |
| 数据盘型号与介质 | IO 延迟影响 pause / snapshot（全内存拷贝写盘） | ★ 数据集所在盘 |
| 数据盘容量 | 决定能否放下快照 / WAL | ＋ |
| 数据盘文件系统（XFS + reflink） | `/data/cubelet` 非 XFS 直接装不上；reflink 决定 CoW 行为 | ＋ |
| 数据盘挂载选项 | 影响 reflink 与写入行为 | ＋ |
| 网卡型号与速率 | 控制面与镜像拉取 | ＋ |
| 机箱进风温度 / 风扇策略 | 温度决定能否维持睿频，直接影响延迟 | ＋ |
| 电源配置 | 供电不足会触发降频 | ＋ |

## B. BIOS / 固件

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| BIOS 版本 | 同设置不同版本行为可能不同 | ＋ |
| BMC 版本 | 影响可设置项与风扇策略 | ＋ |
| Power Profile / 功耗策略 | 鲲鹏侧影响最大的一项 | ★ 功耗与性能 |
| TDP / PPT 上限 | 功耗墙决定全核频率 | ★ 功耗与性能 |
| Determinism / 确定性控制 | 决定频率是否可复现 | ★ 确定性控制 |
| Boost 策略（BoostFmax / Turbo Core） | 睿频开关 | ★ 频率与加速 |
| Uncore Turbo | 影响内存控制器与 L3 频率 | ＋ |
| Global C-state Control | 深度休眠引入唤醒延迟与抖动 | ★ 状态管理 |
| LPI | ARM 专属深度空闲，默认开 | ＋ |
| UFS | 鲲鹏专属，开启会扰动 NUMA 拓扑 | ＋ |
| Node Interleaving | 开启后 OS 只看到 1 个 NUMA 节点 | ＋ |
| SNC | 鲲鹏 930+ 的子 NUMA 划分 | ＋ |
| Memory Interleaving | 与 NPS 共同决定访问分布 | ＋ |
| 内存刷新速率（1x / 2x） | 刷新越慢带宽越高、稳定性越低 | ＋ |
| Hardware Prefetcher | 预取策略影响访存密集负载 | ＋ |
| PCIe ASPM | 省电状态引入延迟 | ＋ |
| 虚拟化扩展（SVM / SMMU） | 没有它跑不了 MicroVM | ★ 虚拟化 |
| IOMMU / AMD-Vi | 设备直通与 DMA 重映射 | ＋ |
| SR-IOV | 网卡虚拟化 | ＋ |
| BIOS 设置证据（Redfish JSON / 截图） | Power Profile 与 Prefetcher **无法从 OS 反推** | ＋ |

## C. OS / 内核

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| 发行版与版本 | 同内核不同发行版补丁不同 | ＋ |
| 内核版本 | 调度、内存管理、KVM 行为 | ★ 内核版本 |
| 内核 cmdline | 大页、隔离、缓解开关都在这里 | ＋ |
| 基础页大小 | 4K vs 64K 改变 TLB / THP / CoW 与快照脏页粒度 | ＋ |
| 透明大页策略 | 引入分配抖动 | ★ 大页配置 |
| 显式大页配置 | 预分配内存，影响可用内存与 TLB | ★ 大页配置 |
| NUMA balancing | 内核自动迁移会污染测量 | ★ NUMA配置 |
| CPU governor | 决定频率是否跟随负载 | ＋ |
| cpufreq 驱动（amd-pstate / acpi-cpufreq） | 驱动不同，调频行为不同 | ＋ |
| 漏洞缓解状态（mitigations） | 部分缓解有显著性能代价 | ＋ |
| IO 调度器 | 影响 pause / snapshot 的落盘 | ＋ |
| SELinux / 防火墙状态 | 影响网络路径 | ＋ |
| cgroup 版本与可用控制器 | 安装脚本只查 `cpu`，压测还可能需要 `cpuset` | ＋ |
| 中断亲和设置 | 中断集中在少数核会拖慢尾延迟 | ＋ |

## D. 虚拟化与 guest

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| KVM 模块与 `/dev/kvm` | 原生 KVM 才能跑 | ＋ |
| 中断控制器（GICv3+ITS / x2APIC） | ARM 侧 vGIC 恢复有额外开销 | ＋ |
| 是否使用 PVM | PVM 仅 x86_64，用了就变成比 hypervisor 路径 | ＋ |
| guest 内核版本与 sha256 | 两平台 guest 内核是不同文件 | ＋ |
| guest 内核 config 差异 | 特性开关不同 | ＋ |
| guest 镜像版本与按架构 digest | tag 相同但两边 rootfs 可能不同 | ★ 镜像版本 |
| CubeVS eBPF 挂载状态 | 沉默失败会走退化网络路径 | ＋ |
| guest PMU 可用性 | ARM guest 可能没有硬件计数器 | ＋ |

## E. CubeSandbox 运行配置

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| 容器操作系统 | guest rootfs 内容 | ★ 容器操作系统 |
| 沙箱规格（vCPU / 内存） | 直接决定单沙箱成本 | ＋ |
| 容器 NUMA 绑定 | 绑定与否改变内存局部性 | ★ 容器NUMA绑定 |
| `tap_init_num` | 池子不够时高并发创建失败 | ＋ |
| balloon free-page 上报 | 两边默认值不同，密度数据不可比 | ＋ |
| 存储后端（cubecow / s3lvol） | 完全不同的 IO 路径 | ＋ |
| 是否启用 S3lvol | 会创建约 512 GiB WAL 镜像 | ＋ |
| 模板 ID | 不同模板不可比 | ＋ |
| 模板 probe 端口 | 没有 probe 时死容器也判 READY | ＋ |
| 模板可写层大小 | 影响建模板与快照耗时 | ＋ |
| 服务端口（API / proxy admin 等） | 端口冲突会让测试跑不起来 | ＋ |

## F. 版本与制品

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| 测试版本（CubeSandbox commit / tag） | 版本差异可能大于平台差异 | ★ 测试版本 |
| CubeSandbox repo 版本 | `release_version` | ★ repo 版本 |
| 各组件版本与 digest | cubemaster / cubelet / cube-api / shim / agent / runtime | ＋ |
| 内核 release tag | 与 `release-assets.yaml` 对齐 | ＋ |
| 发布包 sha256 | 两台机器是否同一个包 | ＋ |
| builder 镜像 digest | 编译工具链是否一致 | ＋ |
| 工具链版本（Go / Rust / clang） | 生成本机码的优化不同 | ＋ |
| docker / docker-compose 版本 | 决定 cgroup / 存储驱动行为 | ★ docker 版本 |
| docker cgroup driver / storage driver | 容器资源计账方式 | ＋ |
| 运行时镜像版本与 digest | coredns / mysql / redis / openresty / minio / proxy / LCM / egress / compose | ★ 镜像版本 |
| 测量工具版本（cube-bench commit / Python SDK） | 计时实现不同结果不同 | ＋ |

## G. 测试协议

| 配置名称 | 为何影响结果 | 状态 |
|---|---|---|
| create 并发档位 | 核心自变量 | ＋ |
| 每档迭代数 `-n` | 样本量 | ＋ |
| warmup 轮数 | 是否丢弃首轮 | ★ 是否启用warmup |
| 正式轮次与取平均方式 | 3 轮均值还是单轮 | ＋ |
| pause / resume 并发与轮数 | 与 create 不同档位 | ＋ |
| 模板构建顺序 | ARM 上顺序会造成 1.2–1.7 倍漂移 | ＋ |
| 每轮是否冷重启 | 同上，控制手段 | ＋ |
| 主机重启后已构建模板数 | issue #1803 的直接观测量 | ＋ |
| 延迟计时口径 | 到响应头 vs 到 body 读完 | ＋ |

---

## 统计

- **共 95 项**，其中现有清单已有 **20 项**（全部覆盖），建议新增 **75 项**。
- 分组：A 平台与硬件 22 项 ｜ B BIOS/固件 20 项 ｜ C OS/内核 14 项 ｜ D 虚拟化与 guest 8 项 ｜ E CubeSandbox 运行 11 项 ｜ F 版本与制品 11 项 ｜ G 测试协议 9 项。

## 三点说明

1. **B 组与 C 组有几项只能间接观测。** `Power Profile` 与 `Hardware Prefetcher` 无法从 OS 反推，所以单列了「BIOS 设置证据」；其余项都可以用 `collect_baseline.py` 采到。

2. **D 组的「guest 内核 config 差异」与「guest 镜像按架构 digest」不是可选项。** 两平台的 guest 内核是不同文件、不同 config，guest 镜像也是不同 tarball —— 不记这两项就无法论证两边跑的是等价环境。

3. **G 组本质是测试协议，不是环境配置。** 若希望规格清单只放环境类，可把 G 组拆到另一张表或另一列。

## 定稿时需要用户确认的两件事

1. **最终的分组与配置名称**（增删、改名、是否保留 G 组）。
2. **每项的性质**：属于「操作者必须按规格配好」，还是属于「操作者只需核对并记录」。

定稿后据此把手册第 2 篇（2.2 / 2.3）与 4.0 的表格对齐，并保持全文只以「从测试规格清单获取 / 查询 / 比较」的方式引用它。
