# CubeSandbox 源码构建与双平台对比测试手册

> 本手册用于在两台裸金属服务器（**x86_64** 与 **鲲鹏 950 / aarch64**）上，**从源码构建并部署** CubeSandbox，然后用**完全一致的方法**测出可以互相对比的数据。
>
> 适用平台：x86_64（AMD EPYC 9654 / 9755 级）、aarch64（鲲鹏 950）
> 验证状态：⚠️ **本手册未在实机验证过**。每条结论标注依据 —— ✅ 源码确认 ｜ 📄 官方文档 ｜ 🔬 上游 issue/PR ｜ ⚠️ 待验证
> 脱敏说明：全文不含真实 IP、口令、内网主机名；示例统一用 `192.168.x.x`、`<proxy-host:port>`。

---

# 第 1 篇　开始之前

> 本篇的目的是：即使你**从未听说过 CubeSandbox**，读完也能看懂后面每一步在做什么。

## 1.1 CubeSandbox 是什么，部署完会得到什么

CubeSandbox 是腾讯云开源的 **AI Agent 安全沙箱**服务（Apache 2.0）。它用 Rust 写的轻量虚拟机监控器（基于 KVM）为每次代码执行拉起一个 **MicroVM**，做到毫秒级冷启动、高密度共存，并对外提供**兼容 E2B 协议的 HTTP API**。

一次完整部署（单机 control 角色）跑起来后，你会得到：

```
        你的压测客户端 / Agent 代码
                    │  HTTP (E2B 协议)
                    ▼
        ┌───────────────────────┐
        │  cube-api  :3000      │  E2B 兼容 REST API，创建/销毁沙箱
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │  CubeMaster :8089     │  调度与编排：决定沙箱落在哪个节点
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │  Cubelet   (宿主机进程)│  节点 Agent：真正在本机拉起 MicroVM
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │ containerd-shim-cube  │  容器运行时 shim
        │ + cube-runtime        │
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │ hypervisor (Rust VMM) │  基于 /dev/kvm 启动 MicroVM
        └───────────┬───────────┘
                    ▼
              MicroVM + guest agent (cube-agent)

  支撑组件（Docker 容器）：
    cube-sandbox-mysql    :3306   元数据
    cube-sandbox-redis    :6379   缓存/注册
    cube-sandbox-minio    :9000   对象存储
    cube-proxy            :80/443 cube.app 域名路由与 TLS
    cube-proxy-coredns    DNS     域名解析
    cube-webui            :12088   管理界面
```

**每个组件一句话职责**：

| 组件 | 职责 | 语言 |
|---|---|---|
| `cube-api` | 对外唯一 API 入口，兼容 E2B SDK | Rust |
| `CubeMaster` | 调度器：把沙箱分配到节点，管模板 | Go |
| `Cubelet` | 节点 Agent：本机拉起/暂停/销毁 MicroVM | Go（通过 CGO 调 `cubecow`） |
| `containerd-shim-cube-rs` / `cube-runtime` | 容器运行时 shim 与 runtime | Rust |
| `hypervisor` | MicroVM 监控器（RustVMM + KVM） | Rust |
| `cube-agent` | guest 内 Agent，被 shim 注入 | Rust |
| `cubecow` | 基于 XFS `FICLONE` 的写时复制快照库 | Rust |
| `cube-proxy` + `coredns` | `cube.app` 域名路由、TLS（mkcert） | 容器 |
| MySQL / Redis / MinIO | 元数据 / 缓存 / 对象存储 | 容器 |

## 1.2 这次测试要回答什么、产出什么数据

| 测试项 | 回答的问题 | 主要指标 |
|---|---|---|
| **创建延迟** | 从请求到沙箱可用要多久？ | `create` 的 `avg / min / p95 / p99` |
| **并发扩展** | 并发拉高后吞吐与延迟如何变化？ | 100 并发下的 `wall`、吞吐、成功率 |
| **Pause / Resume** | 暂停与恢复有多快？ | `pause_*` / `resume_*` 的 `avg / min / p95 / max`、`per_*` |

**为什么要两台机器**：要回答的是「同一个 CubeSandbox，在 x86_64 与鲲鹏 950 上表现差多少」。所以两台机器上跑的东西必须尽量一致，测试方法必须完全相同 —— 这是**第 2 篇**存在的原因。

数据最终填进 `checking_points.xlsx`，取值方式见 **4.6**。

## 1.3 本手册的步骤写法（请先读这一节）

本文档**每一个步骤**都统一给出以下 7 项。任何时候你都能立刻知道「这一步在干嘛、它产出给谁用」：

| 项 | 说明 |
|---|---|
| **做什么** | 一句话说明这一步的目的 |
| **为什么做** | 不做会怎样（这一步存在的理由） |
| **前置条件** | 依赖前面哪一步的什么产出 |
| **命令** | 可直接复制执行 |
| **产出** | 这一步结束后多了什么（文件 / 服务 / 数据） |
| **产出给谁用** | 后面哪一步会消费它 ← **避免"很久之后才知道这步干嘛"** |
| **怎么验证 / 失败怎么办** | 可执行的判断依据 + 常见错误处置 |

编号规则：`3.2.1` = 第 3 篇第 2 章第 1 步。**部署**（第 3 篇）与**测试**（第 4 篇）是两件独立的事，见 3.0 与 4.0。

## 1.4 术语表

| 术语 | 全称 | 含义 |
|---|---|---|
| BM | Bare Metal | 裸金属物理机 |
| PVM | Pagetable-based Virtual Machine | 让普通云服务器获得 KVM 的方案，**仅 x86_64** |
| NPS | NUMA Nodes Per Socket | AMD 每插槽 NUMA 分区数（0/1/2/4） |
| SCCL | Super CPU Cluster | 鲲鹏的 NUMA 粒度单位 |
| UFS | Unified Fabric Switch | 鲲鹏互联调度特性，压测时应关闭 |
| LPI | Low Power Idle | ARM 深度空闲状态 |
| SMMU | System MMU | ARM 的 IOMMU |
| CoW | Copy-on-Write | 写时复制（XFS reflink / CubeCoW） |
| THP | Transparent Huge Page | 透明大页 |
| wall | wall-clock time | 整批端到端耗时 |
| per | per-operation | 均摊耗时（wall ÷ 操作数） |

---

# 第 2 篇　公平性基线

> 本篇是**两台机器动手部署之前**必须先做完的设置。不做完，后面测出的两组数据**没有可比性**。

## 2.1 为什么两台机器必须一致

因为我们要对比的是「平台差异」，不是「配置差异」。三原则：

| 原则 | 要求 |
|---|---|
| **同版本** | 两台机器从**同一个 tag** 构建 CubeSandbox |
| **同协议** | 同样的并发档位、同样的热身与轮次规则、同样的沙箱规格 |
| **同基线** | BIOS / 内核 / 大页 / NUMA 设置按本篇对齐并**记录** |

⚠️ 上游官方性能报告（📄[基准测试报告](.dsh.local/scratch/CubeSandbox/docs/zh/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md)）**只有 x86_64 数据**（Intel Xeon 8255C），**不能当作鲲鹏 950 的基线**，也不能拿它和你的 x86 数据混合比较。

## 2.2 BIOS / 固件对照清单

| 目标 | x86_64（AMD） | 鲲鹏 950 |
|---|---|---|
| 消除频率不确定性 | Determinism Control = **Performance** | Power Profile = **Performance** |
| 放开功耗墙 | TDP/PPT = Manual，值按机型（9654→400 / 9755→500） | Turbo(Core) = **On**；Turbo(Uncore) = **On** |
| 关深度空闲 | Global C-state Control = Disabled（或限 C1） | C-State = **C0/C1**；**LPI = Off** |
| 干净的 NUMA | NPS = 1 或 4（**选定并记录**） | **Node Interleaving = Off**；**UFS = Off** |
| 睿频 | BoostFmaxEn = Manual；BoostFmax = Auto | Turbo(Core) = On |
| 虚拟化 | SVM = Enabled；SR-IOV = Enabled | SMMU = Enabled |
| 内存速率 | 额定满速 | DDR Speed = 额定满速 |
| PCIe 省电 | ASPM Disabled | PCIe ASPM = Off |
| 超线程 | 与鲲鹏侧**保持同策略** | 与 x86 侧**保持同策略** |

⚠️ **鲲鹏侧两个必关项**（🔬 社区资料，非华为官方）：**UFS** 默认开，会扰动 NUMA 拓扑；**LPI** 默认开，会引入唤醒延迟与尾抖动。

⚠️ **无法从 OS 反推的项**：`Power Profile` 与 `Hardware Prefetcher` 只能从 BIOS/Redfish 获取。请导出 **Redfish BIOS Attributes JSON 或拍 BIOS 截图**归档。

## 2.3 OS / 内核 / 大页 / NUMA 设置清单

| 项目 | 要求 | 查看命令 |
|---|---|---|
| 发行版 | 两边尽量同版本 | `cat /etc/os-release` |
| 内核 | 记录精确版本 | `uname -r` |
| 内核 cmdline | 归档 | `cat /proc/cmdline` |
| 大页 | 两边**显式设成同一组参数**并记录 | `grep -i huge /proc/meminfo` |
| 透明大页 | 两边同策略 | `cat /sys/kernel/mm/transparent_hugepage/enabled` |
| NUMA balancing | 两边一致（建议 `0`） | `cat /proc/sys/kernel/numa_balancing` |
| CPU governor | 两边一致（performance） | `cpupower frequency-info` |
| 基础页大小 | 记录（架构差异，见 2.5） | `getconf PAGESIZE` |
| 漏洞缓解状态 | 记录（影响性能） | `cat /sys/devices/system/cpu/vulnerabilities/*` |

⚠️ **CubeSandbox 的 `.env` 里没有任何大页/NUMA/cpuset 变量**（✅ 全文检索确认）。这些**全部是宿主机层设置**，靠内核 cmdline、`numactl`、`cpupower` 完成。

## 2.4 必须记录的证据

**每一步都用脚本自动收集，不要手工抄**：

```bash
python3 collect_baseline.py --out baseline-$(uname -m).json
```

该脚本收集：CPU/型号/SKU、SMT 状态、NUMA 拓扑、内存与通道速率、页面大小与 THP、大页、内核与 cmdline、governor、漏洞缓解、KVM/虚拟化能力、firmware/BIOS 信息、磁盘与文件系统、Docker 版本与镜像 digest。

> 完整字段说明见 **5.5**。**两台机器都必须在部署前跑一次，并归档两份 JSON。**

## 2.5 无法对齐的架构差异（如实记录，不假装相等）

| 差异 | 说明 | 处置 |
|---|---|---|
| **基础页大小** | x86 通常 4 KB；arm64 可能 4/16/**64 KB**。影响 TLB、THP、CoW 粒度与快照脏页粒度 | 记录；快照类指标按**实际脏页字节数**归一化 |
| **guest 内核不同** | 两平台是不同文件、不同 config | 记录双方 sha256 |
| **vGIC vs LAPIC** | 🔬 aarch64 快照恢复每沙箱约 1330 次 KVM ioctl（vGIC 逐字重放），x86 无此结构 | 把 restore 阶段单独报告 |
| **balloon 上报默认值不同** | 🔬 aarch64 默认关、x86_64 默认开 → 影响密度 | 两边显式设同值并记录 |
| **guest PMU** | ⚠️ 部分 ARM 主机不向 guest 暴露 PMUv3 | 不要用 guest 内硬件计数器做对比 |
| **ISA** | AVX-512 vs NEON/SVE | 涉及向量化的负载单独说明 |

## 2.6 红线清单（违反则数据作废）

| # | 红线 | 为什么致命 |
|---|---|---|
| R1 | 一边用 PVM、一边用原生 KVM | PVM **仅 x86_64**，是另一条 hypervisor 路径，比的不是 CPU |
| R2 | 两边 commit 不同 | 版本差异可能远大于平台差异 |
| R3 | SMT/线程数不一致且未记录 | 线程数可能差 2 倍 |
| R4 | NUMA 模式不一致且未记录 | 远端内存占比不同 |
| R5 | 工作负载镜像不是 Multi-Arch | 一边可能压根跑不起来 |
| R6 | 模板构建顺序两边不同 | 会命中 ARM64 专属 bug（见 4.0） |
| R7 | 拿官方 ~48ms 当 ARM 基线 | 那是 Intel x86_64 的数字 |
| R8 | 未验证 CubeVS eBPF 真的挂上 | 网络指标会静默走退化路径 |
| R9 | 拿 guest 内 PMU 指标对比 | ARM guest 可能没有 PMU |

---

# 第 3 篇　部署

> 本篇把源码变成一个**可用且已验证**的 CubeSandbox。做完即停，本篇不涉及任何测试。

## 3.0 部署总览

**部署侧流程**：

```
3.1 构建机准备 ──► 3.2 准备 guest 内核 ──► 3.3 源码构建发布包
                                                    │
                                                    ▼
                                        3.4 目标机安装 ──► 3.5 部署验收
```

**部署完成的判据**（全部满足才算部署结束）：

1. `systemctl list-units 'cube-sandbox-*'` 中 control target 为 active；
2. `curl http://127.0.0.1:<CUBE_API 端口>/health` 返回成功；
3. MySQL / Redis / MinIO / cube-proxy / coredns / webui 容器均为 running；
4. `3.5` 的证据文件已归档。

**部署侧产出清单**（逐项标明消费者）：

| 产出 | 由哪一步产生 | 被谁消费 |
|---|---|---|
| 机器级 BIOS/OS 基线证据 JSON | 2.4 | 结果报告（2 篇） |
| `vmlinux`（含 sha256） | 3.2 | 3.3 构建 |
| `release-manifest.json` + `VERSION.txt` | 3.3 | 3.4 安装校验、4.6 填表 |
| 运行中的服务与空闲端口 | 3.4 | 4.1～4.5 全部测试 |
| 安装后指纹文件 | 3.5 | 4.6 填表、问题排查 |

> **部署与测试的边界**：部署完成后，**第 4 篇不再回头修改部署**。测试阶段只做「建模板 → 压测 → 记录」。

## 3.1 构建机准备

> 构建机与目标机**可以是同一台**（📄 官方允许），也可以是两台。若目标机是共享服务器，建议分开。

### 3.1.1 安装系统依赖

- **做什么**：装齐构建所需的命令行工具。
- **为什么做**：构建脚本只用 `docker` + `make`，但它随后会**在宿主机上**执行 guest 镜像生成与打包，因此宿主机还需要一整套工具；缺任何一个都会在脚本中途直接报错退出。
- **前置条件**：有 root 或 sudo。
- **命令**：

```bash
# openEuler / RHEL 系
dnf install -y docker make git python3 tar pigz gzip sudo e2fsprogs util-linux which
# Debian / Ubuntu 系
apt-get update && apt-get install -y docker.io make git python3 tar pigz gzip sudo e2fsprogs util-linux
```

- **产出**：可用的 `docker` / `make` / `git` / `python3` / 打包工具。
- **产出给谁用**：`3.3` 构建发布包（`docker`、`make`、`python3`、`mkfs.ext4`、`tar`/`pigz`）；`3.1.2` 的镜像拉取。
- **怎么验证 / 失败怎么办**：

```bash
docker --version && make --version | head -1 && git --version && python3 --version
mkfs.ext4 -V 2>&1 | head -1     # 必须支持 -d，否则 guest 镜像生成失败
npm --version || echo "WARN: 无 npm，WebUI 无法构建（用 ONE_CLICK_WEB_DIST_DIR 绕过）"
```

> ⚠️ **最易踩的坑**：**WebUI 在宿主机上用 npm 构建**，**builder 容器里没有 Node**（✅ `build-release-bundle.sh:665-667`）。构建机必须自备 `npm`。

### 3.1.2 配置代理与网络

- **做什么**：设置代理与 Go 模块代理。
- **为什么做**：构建需要访问 Docker Hub、apt 源、go.dev、crates.io、npm 等多个外部源；内网环境不设代理会大面积拉取失败。**builder 容器内的下载也需要代理**。
- **前置条件**：3.1.1 完成；已知代理地址。
- **命令**：

```bash
export http_proxy="http://<proxy-host:port>"
export https_proxy="$http_proxy"
export no_proxy="localhost,127.0.0.1,192.168.0.0/16"
export GOPROXY="https://goproxy.cn,direct"     # 内网必需
```

- **产出**：可用的出网能力（`http_proxy` 等环境变量）。
- **产出给谁用**：3.1.3 拉镜像、3.2 下载内核、3.3 构建。
- **怎么验证 / 失败怎么办**：

```bash
curl -sI -x "$http_proxy" https://go.dev/dl/ | head -1
```
失败则确认代理是否允许 HTTPS CONNECT，以及 `no_proxy` 是否误排除目标。

### 3.1.3 预拉 builder 基座镜像（Docker Hub 不可达时）

- **做什么**：把 builder 镜像的基座 `ubuntu:20.04` 从可达镜像源拉到本地并**打上原名标签**。
- **为什么做**：✅ builder 镜像的 Dockerfile 第一行是 `FROM ubuntu:20.04`（**Docker Hub**）。Docker Hub 不可达时 `make builder-image` 会在第一步失败。打上原名标签后，Dockerfile **无需改动**即可命中本地镜像。
- **前置条件**：3.1.1（docker）、3.1.2（代理）；`skopeo` 可用。
- **命令**：

```bash
ARCH=$(uname -m); case "$ARCH" in x86_64) A=amd64;; aarch64) A=arm64;; esac
echo "arch=$ARCH -> $A"

# builder 基座
HTTPS_PROXY=$https_proxy skopeo copy --override-arch "$A" \
  "docker://<your-mirror>/ubuntu:20.04"  "docker-daemon:ubuntu:20.04"

# guest 镜像基座（3.3 会用到）
HTTPS_PROXY=$https_proxy skopeo copy --override-arch "$A" \
  "docker://<your-mirror>/tencentos/tencentos4-minimal:latest" \
  "docker-daemon:tencentos/tencentos4-minimal:latest"

docker images | grep -E 'ubuntu|tencentos'
```

- **产出**：本地镜像 `ubuntu:20.04` 与 `tencentos/tencentos4-minimal:latest`。
- **产出给谁用**：3.3 的 `make builder-image`（消费 `ubuntu:20.04`）与 guest 镜像构建（消费 `tencentos4-minimal`）。
- **怎么验证 / 失败怎么办**：

```bash
docker image inspect ubuntu:20.04 >/dev/null && echo "OK: ubuntu:20.04 present"
```
⚠️ 若拿不到 `ubuntu:20.04`，可改 `docker/Dockerfile.builder` 第一行指向可达镜像 —— 但这**偏离上游**，必须在报告中注明。

> ⚠️ 在 x86 构建机上为 aarch64 目标构建时，`--override-arch arm64` **必须显式给出**，否则会拉到错误架构。

### 3.1.4 获取源码并钉死版本

- **做什么**：拉取指定 tag 的源码，记录 commit；核对上游钉死的重资产版本。
- **为什么做**：**同 commit 是两台机器可比的前提**（2.1）。用漂移的 master HEAD 会让两组数据无法归因。
- **前置条件**：3.1.2 完成。
- **命令**：

```bash
CUBE_TAG=v0.7.2          # 建议 ≥ v0.7.2，理由见下
git clone --depth 1 --branch "$CUBE_TAG" \
  https://github.com/TencentCloud/CubeSandbox.git /path/to/CubeSandbox
cd /path/to/CubeSandbox

git rev-parse HEAD        | tee /tmp/cube-commit-$(uname -m).txt
git describe --tags --abbrev=0 | tee -a /tmp/cube-commit-$(uname -m).txt
git status --porcelain    # 期望为空
cat deploy/release-assets.yaml
```

- **产出**：指定 commit 的工作树；`/tmp/cube-commit-<arch>.txt`；`release-assets.yaml` 中的内核/镜像 tag。
- **产出给谁用**：3.2 用 `release-assets.yaml` 的 tag 下载内核；3.3 构建；4.6 填「测试版本」。
- **怎么验证 / 失败怎么办**：

```bash
git rev-parse --verify HEAD && echo "OK"
```
> ⚠️ **版本选择建议**：用 **≥ v0.7.2**，因为 🔬 该版本修复了 **ARM 上 pause 后 resume 失败**（`Could not restore GICv3ITS state`）与 cubebench 拓扑报告兼容性问题。更早版本在 aarch64 上 pause/resume 测试不可用。
>
> ✅ 上游 `.gitmodules` 是**空文件**，**不需要 `--recursive`**。

## 3.2 准备 guest 内核

### 3.2.1 放置 `vmlinux`

- **做什么**：把 guest 内核文件放到 `deploy/one-click/assets/kernel-artifacts/vmlinux`。
- **为什么做**：✅ `ensure_kernel_vmlinux` 在文件缺失时**直接 `exit 1`**；📄 官方「已知限制」也写明「缺少 `vmlinux`，构建会立即失败」。它是 guest MicroVM 的内核。
- **前置条件**：3.1.4（已 clone，能读到 `release-assets.yaml`）；3.1.2（能下载）。
- **命令（方式一：下载官方发布内核，推荐）**：

```bash
ARCH=$(uname -m); case "$ARCH" in x86_64) A=amd64;; aarch64) A=arm64;; esac
KERNEL_TAG=$(grep -E "^kernel_bm_${A}:" deploy/release-assets.yaml | awk '{print $2}')
echo "kernel tag = $KERNEL_TAG"

mkdir -p deploy/one-click/assets/kernel-artifacts
curl -L -x "$https_proxy" \
  -o deploy/one-click/assets/kernel-artifacts/vmlinux \
  "https://cnb.cool/CubeSandbox/CubeSandbox/-/releases/download/${KERNEL_TAG}/vmlinux-${A}"

sha256sum deploy/one-click/assets/kernel-artifacts/vmlinux \
  | tee /tmp/kernel-sha256-${A}.txt
```

**方式二（自行编译）**：

```bash
make kernel KERNEL_SRC=/path/to/linux
# 两平台 config 都在仓库里：
ls configs/kernel-oc9.*.config     # x86_64 / aarch64 各一份
```

- **产出**：`deploy/one-click/assets/kernel-artifacts/vmlinux` + 其 sha256。
- **产出给谁用**：3.3 构建（打进发布包）；4.6 填「镜像/内核版本」。
- **怎么验证 / 失败怎么办**：

```bash
ls -l deploy/one-click/assets/kernel-artifacts/vmlinux
```
> ⚠️ **不要**下载 `vmlinux-pvm-*`：PVM 仅 x86_64，且本手册要求**两台机器都用原生 KVM**（红线 R1）。

## 3.3 源码构建发布包

### 3.3.1 配置 `build.env`

- **做什么**：从模板复制构建配置。
- **为什么做**：`local` 模式才会**从源码编译**；若被改成其它值就会退回"用预编译二进制"，那就不满足本手册的目标。
- **前置条件**：3.1.4。
- **命令**：

```bash
cp deploy/one-click/build.env.example deploy/one-click/build.env
grep -E 'BUILD_MODE|BUILDER_IMAGE|KERNEL_VMLINUX' deploy/one-click/build.env
```

- **产出**：`deploy/one-click/build.env`。
- **产出给谁用**：3.3.2 构建。
- **怎么验证 / 失败怎么办**：确认 `ONE_CLICK_*_BUILD_MODE=local`。

### 3.3.2 构建 builder 镜像并构建发布包

- **做什么**：先构建 builder 容器镜像，再用它编译所有组件并打包。
- **为什么做**：CubeSandbox 是多语言工程（Rust + Go + C/eBPF + Web），官方用统一 builder 镜像保证工具链版本固定（Go 1.25.7、Rust 多工具链、clang-14 等）。这是"从源码"得以可复现的关键。
- **前置条件**：3.1.1～3.1.4、3.2.1、3.3.1。
- **命令**：

```bash
export http_proxy="http://<proxy-host:port>" https_proxy="$http_proxy"
export GOPROXY="https://goproxy.cn,direct"

make builder-image                       # 国内可加 MIRROR=cn 改写 apt/rustup/LLVM 源
./deploy/one-click/build-release-bundle-builder.sh 2>&1 | tee /tmp/cube-build-$(uname -m).log
```

- **产出**：`deploy/one-click/dist/cube-sandbox-one-click-<version>.tar.gz` 及解压目录。
- **产出给谁用**：3.4 目标机安装。
- **怎么验证 / 失败怎么办**：

```bash
ls -l deploy/one-click/dist/
BUNDLE=$(ls deploy/one-click/dist/cube-sandbox-one-click-*.tar.gz | head -1)
sha256sum "$BUNDLE" | tee /tmp/bundle-sha256-$(uname -m).txt
tar -tzf "$BUNDLE" | grep -E 'VERSION.txt|release-manifest.json|install.sh|env.example'
```

常见失败：拉 `ubuntu:20.04` 失败 → 回 3.1.3；WebUI 失败 → 装 npm；OOM → 降 `ONE_CLICK_BUILD_JOBS`。

### 3.3.3 归档构建元数据

- **做什么**：从发布包里取出 `VERSION.txt` 与 `release-manifest.json`。
- **为什么做**：这是「两台机器跑的到底是不是同一个东西」的**唯一硬证据**，也是 4.6 填表的直接来源。
- **前置条件**：3.3.2 成功。
- **命令**：

```bash
OUT=/tmp/cube-artifacts-$(uname -m); mkdir -p "$OUT"
tar -xzf "$BUNDLE" -C "$OUT" --strip-components=1 \
  --wildcards '*/VERSION.txt' '*/release-manifest.json' 2>/dev/null || true
cat "$OUT/VERSION.txt"
python3 -m json.tool "$OUT/release-manifest.json" | head -40
```

- **产出**：归档的 `VERSION.txt`、`release-manifest.json`。
- **产出给谁用**：3.4.1（安装时会校验 manifest）、4.6 填表。
- **怎么验证 / 失败怎么办**：确认 manifest 含 `components`、`guest_image`、`kernel` 三个键。

## 3.4 目标机安装

### 3.4.1 安装前检查

- **做什么**：逐项确认目标机满足安装的前置条件。
- **为什么做**：这些检查 install.sh 自己也会做，但**在安装前发现比在破坏性阶段中途失败便宜得多**；而且 install.sh **不检查端口占用**，必须手工排查。
- **前置条件**：拿到目标机 root。
- **命令**：

```bash
ls -la /dev/kvm                        # 必须存在（原生 KVM）
grep MemTotal /proc/meminfo            # 下限 7500000 KB
ldd --version | head -1                # glibc ≥ 2.31
grep bpf /proc/filesystems; df -T /sys/fs/bpf
stat -fc %T /sys/fs/cgroup             # v1 会被接受
lsblk -o NAME,SIZE,FSTYPE,MOUNTPOINT
df -hT / /data 2>/dev/null

# 端口占用（install.sh 不做这件事）
for p in 3000 8082 8089 3010 3306 6379 9000 9001 80 443 9090 9091 12088 8083; do
  ss -lntp "( sport = :$p )" | tail -n +2 | sed "s/^/port $p: /"
done
```

- **产出**：一份"哪些前提已满足 / 哪些端口被占"的结论。
- **产出给谁用**：3.4.4 决定是否需要改 `.env` 端口。
- **怎么验证 / 失败怎么办**：见下表。

| 检查项 | 要求 | 失败含义 |
|---|---|---|
| `/dev/kvm` | 存在 | 不是物理机或 KVM 未开 |
| 内存 | ≥ 7500000 KB | 不够（`CUBE_MIN_MEMORY_KB` **只能调高**） |
| glibc | ≥ 2.31 | 系统过旧 |
| bpffs | `/sys/fs/bpf` 类型为 `bpf` | eBPF 无法 pin |
| cgroup | v2 需有 `cpu` 控制器（**v1 直接通过**） | 启动会失败 |
| `/data/cubelet` 所在 FS | 必须是 **XFS** | 见 3.4.2 |
| 端口 | 空闲 | 见 3.4.4 |

### 3.4.2 准备磁盘布局（bind mount）

- **做什么**：把 `/data` 指到 XFS 分区，把安装根指到空间充足的分区。
- **为什么做**：两个硬约束叠加 —— ① ✅ `/data/cubelet` **必须在 XFS 上**（install.sh 会检查，非 XFS 直接 die）；② ✅ `CUBE_SANDBOX_INSTALL_ROOT` 在 `lib/common.sh:9-12` 被**强制覆盖为 `/usr/local/services/cubetoolbox` 并设为 `readonly`** —— 写进 `.env` 会被**静默忽略**（不报错），上游**没有提供任何官方绕过方式**。
- **前置条件**：3.4.1 完成；有空间充足的 XFS 分区与大容量分区。
- **命令**：

```bash
# /data -> XFS 分区
mkdir -p /mnt/bss/cube-data /data
mount --bind /mnt/bss/cube-data /data

# 安装根 -> 空间充足的分区（绕过 readonly）
mkdir -p /home/cubetoolbox /usr/local/services/cubetoolbox
mount --bind /home/cubetoolbox /usr/local/services/cubetoolbox

# 持久化
echo "/mnt/bss/cube-data /data none bind 0 0" >> /etc/fstab
echo "/home/cubetoolbox /usr/local/services/cubetoolbox none bind 0 0" >> /etc/fstab
```

- **产出**：`/data` 落在 XFS 上；`/usr/local/services/cubetoolbox` 指向大分区。
- **产出给谁用**：3.4.5 `install.sh`（通过 XFS preflight 与安装根写入）。
- **怎么验证 / 失败怎么办**：

```bash
df -T /data && xfs_info /data | grep reflink     # 期望 Type=xfs, reflink=1
df -T /usr/local/services/cubetoolbox
```
> ⚠️ bind mount 绕过 `readonly` 是**运维手段，上游未支持也未测试**，请在报告中注明。
> ✅ **省空间**：`ONE_CLICK_ENABLE_S3LVOL` 默认 `0`，此时**不会**创建那个约 512 GiB 的 WAL 镜像。除非要测 S3 卷，否则保持 0。

### 3.4.3 预拉运行时镜像

- **做什么**：把安装/运行时需要的容器镜像预先拉到目标机。
- **为什么做**：✅ **发布包里不含任何镜像 tarball**（全仓库无 `docker save`），所有运行时镜像都在目标机上拉取。Docker Hub 不可达或网络受限时，安装阶段会卡住。
- **前置条件**：3.1.2 的代理思路；`skopeo` 可用。
- **命令**：

```bash
ARCH=$(uname -m); case "$ARCH" in x86_64) A=amd64;; aarch64) A=arm64;; esac
IMAGES=(
  "cube-sandbox-image.tencentcloudcr.com/opensource/coredns/coredns:1.14.2"
  "cube-sandbox-image.tencentcloudcr.com/opensource/mysql:8.0"
  "cube-sandbox-image.tencentcloudcr.com/opensource/redis:7-alpine"
  "cube-sandbox-image.tencentcloudcr.com/opensource/openresty:1.21.4.1-6-alpine-fat"
)
for img in "${IMAGES[@]}"; do
  echo ">>> $img"
  HTTPS_PROXY=$https_proxy skopeo copy --override-arch "$A" "docker://$img" "docker-daemon:$img"
done

# Docker Hub 的 compose 镜像：从可达源拉取后打回原名
HTTPS_PROXY=$https_proxy skopeo copy --override-arch "$A" \
  "docker://<your-mirror>/docker/compose:1.29.2" "docker-daemon:docker/compose:1.29.2"

docker images | grep -E 'coredns|mysql|redis|openresty|compose'
```

- **产出**：本地可用的运行时镜像。
- **产出给谁用**：3.4.5 `install.sh` 启动 MySQL/Redis/MinIO/WebUI/CoreDNS/cube-proxy 时消费。
- **怎么验证 / 失败怎么办**：`docker images` 应能看到上述镜像。缺哪个补哪个。
> ⚠️ **`docker/compose:1.29.2` 来自 Docker Hub**，是最容易被忽略的一个。

### 3.4.4 配置 `.env`

- **做什么**：解压发布包，生成并编辑 `.env`。
- **为什么做**：`.env` 决定端口、节点 IP、镜像区域、是否重启 Docker 等；装错会导致 unit 启动失败，或在共享服务器上影响他人容器。
- **前置条件**：3.3.3、3.4.1（已知端口占用）、3.4.2。
- **命令**：

```bash
tar -xzf cube-sandbox-one-click-<version>.tar.gz
cd cube-sandbox-one-click-<version>
cp env.example .env
```

按 3.4.1 的结果编辑 `.env`：

```bash
CUBE_SANDBOX_NODE_IP=192.168.x.x      # 主网卡不是 eth0 时必须显式指定
MIRROR=cn

# 若 3000 / 8082 被占用（示例）
CUBE_API_BIND=0.0.0.0:3001
CUBE_API_HEALTH_ADDR=127.0.0.1:3001   # 必须与 CUBE_API_BIND 端口一致
CUBE_PROXY_ADMIN_PORT=8083

ONE_CLICK_DEPLOY_ROLE=control
ONE_CLICK_ASSUME_YES=1
ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR=0   # 共享服务器必须为 0
CUBE_PVM_ENABLE=0                          # 原生 KVM（红线 R1）
ONE_CLICK_ENABLE_S3LVOL=0                  # 避免创建 ~512GiB 镜像
```

- **产出**：目标机上的 `.env`。
- **产出给谁用**：3.4.5 `install.sh`。
- **怎么验证 / 失败怎么办**：

```bash
grep -E 'CUBE_API_BIND|CUBE_API_HEALTH_ADDR|CUBE_PROXY_ADMIN_PORT|PVM|S3LVOL|TENCENT_DOCKER' .env
```
> ✅ **`ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR` 默认已是 0**。一旦设为 `1`，install.sh 会改写 `/etc/docker/daemon.json` 并 **`systemctl restart docker`**（✅ `install.sh:1370-1417`）—— 共享服务器上会停掉所有 `RestartPolicy=no` 的容器。**务必保持 0。**

### 3.4.5 运行安装

- **做什么**：执行 `install.sh`。
- **为什么做**：它完成解压组件、写配置、装 systemd unit、启动服务、健康检查的全过程。
- **前置条件**：3.4.2、3.4.3、3.4.4 全部完成。
- **命令**：

```bash
./install.sh --mode=install 2>&1 | tee /var/log/cube-install-$(uname -m).log
```

- **产出**：运行中的 CubeSandbox（systemd units + 容器 + 宿主机进程）。
- **产出给谁用**：3.5 验收；第 4 篇全部测试。
- **怎么验证 / 失败怎么办**：

```bash
journalctl -u cube-sandbox-cubelet -e --no-pager | tail -50
journalctl -u cube-sandbox-cube-proxy -e --no-pager | tail -50
```

| 失败现象 | 原因 | 处置 |
|---|---|---|
| `/data/cubelet` 非 XFS | 3.4.2 未做 | 回 3.4.2 |
| `port ... already in use` | 端口冲突 | 回 3.4.4 改端口 |
| `invalid release manifest` | manifest 结构不符 | 检查 3.3.3 |
| DNS preflight 失败 | 缺 resolvectl/NetworkManager | 装 dnsmasq 或设 `CUBE_PROXY_DNSMASQ_MODE=standalone` |

## 3.5 部署验收

- **做什么**：确认系统真的可用，并归档安装后指纹。
- **为什么做**：只有验收通过，第 4 篇的数据才有意义；指纹文件是事后归因问题的唯一依据。
- **前置条件**：3.4.5 完成。
- **命令**：

```bash
systemctl list-units 'cube-sandbox-*' --no-pager
ss -tlnp | grep -E ':(3001|8089|3010|8083|12088)'
curl -s http://127.0.0.1:3001/health || curl -s http://127.0.0.1:3010/health
docker ps --format '{{.Names}}\t{{.Status}}'
./smoke.sh

# 归档
python3 collect_baseline.py --out postinstall-$(uname -m).json
{ systemctl list-units 'cube-sandbox-*' --no-pager
  docker ps --format '{{.Names}}\t{{.Image}}\t{{.Status}}'
  docker images --digests
  cat /usr/local/services/cubetoolbox/VERSION.txt 2>/dev/null
  head -c 3000 /usr/local/services/cubetoolbox/release-manifest.json 2>/dev/null
} > /tmp/postinstall-$(uname -m).txt 2>&1
```

- **产出**：验收结论 + `postinstall-<arch>.json` + `/tmp/postinstall-<arch>.txt`。
- **产出给谁用**：4.6 填表；问题排查。
- **怎么验证 / 失败怎么办**：按 3.0 的「部署完成判据」逐条核对；未通过不要进入第 4 篇。

---

# 第 4 篇　测试

> 本篇产出可比数据。开始前请确认第 3 篇的「部署完成判据」已全部满足。

## 4.0 测试总览

**测试侧流程**：

```
4.1 建模板 ──► 4.2 冒烟 ──► 4.3 创建并发（100）──┐
                  │                              ├──► 4.6 结果记录与汇总
                  └──────► 4.4 Pause/Resume（10）┘
                            （4.5 可选：快照/密度）
```

**测试侧的固定参数**（两台机器必须完全一致）：

| 参数 | 值 |
|---|---|
| 创建并发档位 | **100** |
| 每档迭代数 `-n` | **c × 25** = 2500 |
| 热身 | **`-w 3`**（结果丢弃） |
| 正式轮次 | **3 轮**，取 `avg` 平均 |
| Pause/Resume 并发 | **10** |
| Pause/Resume 测量轮数 | `-n 5` |
| 沙箱规格 | 2 vCPU / 2 GiB |
| 模板镜像 | `sandbox-code:latest`（Multi-Arch，按 digest 钉死） |

**每轮开始前的复位清单**：

```bash
# 1) 清掉残留沙箱（create-only 会留下存活的沙箱）
cubemastercli box list 2>/dev/null || true

# 2) 轮次之间静默，避免上一轮残留影响下一轮
sleep 30
```

> ⚠️ **冷重启纪律（鲲鹏侧必做）**：🔬 上游 issue #1803 报告：在 aarch64 上，「物理机重启后创建的第一个模板总是最快的」，之后**同样镜像/配置**的模板延迟会劣化 **1.2×–1.7×**，吞吐降至 **0.64×**，且**在 x86_64 上无法复现**。所以鲲鹏侧**每轮正式测量前必须冷重启，并重建模板**，同时记录模板构建顺序。**绝不能**"x86 建一次、ARM 建一次"就对比。

## 4.1 建模板

- **做什么**：从 Multi-Arch 镜像制作一个沙箱模板，拿到 `template_id`。
- **为什么做**：CubeSandbox 创建沙箱必须指定模板；后续所有测试都用这一个模板，模板不一致数据就不可比。
- **前置条件**：第 3 篇验收通过；目标机能拉取 `sandbox-code:latest`。
- **命令**：

```bash
cubemastercli tpl create-from-image \
  --image cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/sandbox-code:latest \
  --writable-layer-size 1G \
  --expose-port 49999 --expose-port 49983 \
  --probe 49999

# 用上一步输出的 job_id 跟踪进度
cubemastercli tpl watch --job-id <job_id>

# 记录 template_id
cubemastercli tpl list | tee /tmp/tpl-$(uname -m)-$(date +%H%M).txt
```

- **产出**：`template_id`；模板列表文件。
- **产出给谁用**：4.2 / 4.3 / 4.4 全部测试（作为 `CUBE_TEMPLATE_ID`）；4.6 填表。
- **怎么验证 / 失败怎么办**：`cubemastercli tpl list` 中该模板状态为 `READY`。
> ⚠️ **必须带 `--probe 49999`**，否则**死容器也会被判为 READY**，后面的数据全是假的。

## 4.2 冒烟测试

- **做什么**：单并发创建 1 个沙箱，确认端到端可用。
- **为什么做**：如果单次创建都不成功，后面 100 并发的数据没有意义；这一步把"环境问题"和"性能问题"分开。
- **前置条件**：4.1 拿到 `template_id`。
- **命令**：

```bash
export E2B_API_URL=http://127.0.0.1:3001
export E2B_API_KEY=e2b_000000
export CUBE_TEMPLATE_ID=<模板ID>

cd examples/cube-bench && make          # 产出 ./bin/cube-bench
./bin/cube-bench -m create-only -n 1 -c 1 -w 0
```

- **产出**：一条成功的创建记录（成功率 100%）。
- **产出给谁用**：放行 4.3 / 4.4。
- **怎么验证 / 失败怎么办**：终端报告 `Success Rate 100.0%`。若失败，先查 cubelet 日志与模板状态，**不要**直接跳到并发测试。

## 4.3 创建延迟与并发扩展（100 并发）

- **做什么**：在 **100 并发**下压测沙箱创建，跑 3 轮。
- **为什么做**：这是本测试的核心指标 —— 高并发创建能力，也是两台机器对比的主战场。
- **前置条件**：4.2 通过。
- **命令**：

```bash
cd examples/cube-bench
export E2B_API_URL=http://127.0.0.1:3001 E2B_API_KEY=e2b_000000
export CUBE_TEMPLATE_ID=<模板ID>
mkdir -p /tmp/bench-$(uname -m)

for round in 1 2 3; do
  ./bin/cube-bench -m create-only -c 100 -n 2500 -w 3 \
    -o "/tmp/bench-$(uname -m)/create-c100-r${round}.json" || true
  sleep 30
done
```

- **产出**：3 个 JSON 报告（含 `create{avg,min,p95,p99,std}`、`summary{success_rate,throughput_qps}`，以及每个请求的 `raw`）。
- **产出给谁用**：4.6 汇总与填表。
- **怎么验证 / 失败怎么办**：

```bash
for f in /tmp/bench-$(uname -m)/create-c100-r*.json; do
  echo "== $f"; python3 -c "
import json,sys; d=json.load(open('$f'))
print(' success_rate', d['summary']['success_rate'])
print(' throughput   ', d['summary']['throughput_qps'])
print(' create       ', d['create'])
"
done
```

⚠️ **口径说明**（✅ 源码确认，写报告时必须写明）：
- `cube-bench` 的延迟是**到响应头为止**（`client.Do` 返回即停表），**不含**读 body 与解析，**没有 TTFB 指标**；
- `-c` 是**在途请求上限**，`-n` 是**总迭代数**；
- `-w` 的热身结果**完全不计入统计**，且热身是**串行**执行的；
- 任一迭代出错，进程退出码为 **1**（所以脚本里用了 `|| true`）。

## 4.4 Pause / Resume（10 并发）

- **做什么**：在 **10 并发**下测量暂停与恢复的延迟。
- **为什么做**：pause/resume 是 Agent 场景的关键能力（挂起空闲沙箱省资源）。
- **前置条件**：4.2 通过；已装 Python SDK。
- **命令**：

```bash
cd examples/snapshot-rollback-clone
pip install -r requirements.txt          # cubesandbox>=0.2.0

export CUBE_API_URL=http://127.0.0.1:3001
export CUBE_TEMPLATE_ID=<模板ID>

python3 bench_pause_resume_concurrency.py -c 10 -n 5
```

- **产出**：一行结果，含 `pause_avg/min/p95/max`、`per_pause`、`resume_avg/min/p95/max`、`per_resume`（单位 ms）。
- **产出给谁用**：4.6 汇总与填表。
- **怎么验证 / 失败怎么办**：

```bash
# 无报错且打印出一行含 12 列的表格即为成功
```

> ⚠️ **重要**：**`cube-bench` 无法测 pause/resume**。✅ 源码确认它内部只有两个 HTTP 调用（`POST /sandboxes`、`DELETE /sandboxes/{id}`），没有任何 pause/resume 代码。pause/resume **必须用官方 Python 脚本**。
>
> ⚠️ **必须 ≥ v0.7.2**：🔬 该版本才修好 ARM 上 `Could not restore GICv3ITS state` 的 pause→resume 失败。
>
> ⚠️ **解释数据时注意**：pause 目前是 **full-memory-copy**（把全部匿名页写盘），耗时与沙箱内存**线性相关**。报告里要写明测的是哪种模式。

## 4.5（可选）快照 / 回滚 / 克隆、密度测试

```bash
# 同一目录下的其它官方脚本，用法与 4.4 一致
python3 bench_snapshot_concurrency.py -c 10 -n 5
python3 bench_rollback_concurrency.py -c 10 -n 5
python3 bench_clone_concurrency.py    -c 10 -n 5
```

> ⚠️ 快照类指标受**页大小**影响（2.5）：64 KB 页下一次标记的脏数据是 4 KB 页的 16 倍。**按实际脏页字节数归一化**，不要只比墙钟。
> ⚠️ **密度数据默认不可比**：🔬 balloon free-page 上报在 aarch64 默认关、x86_64 默认开。做密度对比前必须把两边显式设成同值。

## 4.6 结果记录与汇总

- **做什么**：把 3 轮结果汇总，并把版本/环境信息填入 `checking_points.xlsx`。
- **为什么做**：单轮数字容易受偶发影响；3 轮 + 离散度才能判断数据是否稳定。
- **前置条件**：4.3、4.4 完成。
- **命令**：

```bash
python3 - <<'EOF'
import json, glob, statistics as st
for arch_dir in sorted(glob.glob('/tmp/bench-*')):
    avgs, p95s, rates, tps = [], [], [], []
    for f in sorted(glob.glob(f'{arch_dir}/create-c100-r*.json')):
        d = json.load(open(f))
        if d.get('create'):
            avgs.append(d['create']['avg']); p95s.append(d['create']['p95'])
        rates.append(d['summary']['success_rate']); tps.append(d['summary']['throughput_qps'])
    if avgs:
        print(arch_dir)
        print('  create avg  3轮均值 %.1f ms   轮间极差 %.1f ms' % (st.mean(avgs), max(avgs)-min(avgs)))
        print('  create p95  3轮均值 %.1f ms' % st.mean(p95s))
        print('  success_rate 最低 %.4f' % min(rates))
        print('  throughput  3轮均值 %.1f /s' % st.mean(tps))
EOF
```

- **产出**：一张双平台对照表 + 填入 xlsx 的各字段。
- **产出给谁用**：最终报告。
- **怎么验证 / 失败怎么办**：

| xlsx 行 | 取值来源 |
|---|---|
| 测试版本（commit/tag） | `/tmp/cube-commit-<arch>.txt` |
| CubeSandbox repo 版本 | `release-manifest.json` 的 `release_version` |
| docker / docker-compose 版本 | `docker --version`、`docker-compose --version` |
| 镜像版本 | `release-manifest.json` 各组件 version/digest；`docker images --digests` |
| guest 镜像 / 内核 | `release-manifest.json` 的 `guest_image`、`kernel` |
| 内核版本 / 大页 / NUMA / 核数 | `baseline-<arch>.json` 与 `postinstall-<arch>.json` |

> ⚠️ **报比值时必须附 2 篇的硬件/固件对照表**，否则比值没有意义。
> ⚠️ 若轮间极差很大（例如 > 20%），**先怀疑环境不稳定**（温度/降频/残留沙箱），不要直接当结论。

## 4.7 多轮执行的检查清单

再次执行测试时，按顺序确认：

- [ ] 清掉上一轮残留沙箱
- [ ] 轮间静默 ≥ 30 s 已执行
- [ ] 鲲鹏侧：**冷重启 + 重建模板**（#1803）
- [ ] 模板 `template_id` 已更新，并记录构建顺序
- [ ] 并发档位 / `-n` / `-w 3` 与另一台机器**完全一致**
- [ ] 本轮 JSON 已落盘并归档

---

# 第 5 篇　附录

## 5.1 双平台差异速查表

| 动作 | x86_64 | aarch64（鲲鹏 950） |
|---|---|---|
| 架构标识 | `uname -m` = `x86_64`，资产后缀 `amd64` | `uname -m` = `aarch64`，资产后缀 `arm64` |
| guest 内核资产 | `vmlinux-amd64` | `vmlinux-arm64` |
| guest 镜像资产 | `cube-guest-image-amd64.tar.gz` | `cube-guest-image-arm64.tar.gz` |
| 内核 config | `configs/kernel-oc9.x86_64.config` | `configs/kernel-oc9.aarch64.config` |
| PVM | 可用（本手册要求不用） | ❌ **不支持**，必须原生 KVM 裸金属 |
| 热迁移 | 支持 | ❌ 仅 x86_64 |
| KVM 模块 | `kvm_amd` | `kvm` + GICv3 |
| 虚拟化扩展 | SVM / AMD-Vi | SMMU |
| 中断控制器 | x2APIC / IOAPIC | GICv3/GICv4 + ITS |
| 基础页大小 | 4 KB | ⚠️ 4/16/**64 KB**（需实测） |
| 深度空闲 | Global C-state Control | **LPI = Off** |
| 互联相关 | — | **UFS = Off** |
| guest PMU | 通常可用 | ⚠️ 可能不暴露 PMUv3 |
| skopeo 架构覆盖 | `--override-arch amd64` | `--override-arch arm64` |
| 一键脚本 | 自动发现 | ⚠️ `online-install.sh` **不自动发现** ARM64（这是本手册走源码构建的原因之一） |

## 5.2 版本钉死清单（执行时逐项填写）

| 项目 | x86_64 | aarch64 | 来源 |
|---|---|---|---|
| CubeSandbox tag | | | `git describe` |
| commit | | | `git rev-parse HEAD` |
| `kernel-release-*` tag | | | `release-assets.yaml` |
| vmlinux sha256 | | | `sha256sum` |
| `guest-image-*` tag | | | `release-assets.yaml` |
| guest image digest | | | `release-manifest.json` |
| bundle sha256 | | | `sha256sum` |
| 工作负载镜像 digest | | | `docker manifest inspect` |
| builder 镜像 digest | | | `docker images --digests` |

## 5.3 完整命令清单

按 `3.1 → 3.2 → 3.3 → 3.4 → 3.5` 部署，再按 `4.1 → 4.2 → 4.3 → 4.4 → 4.6` 测试。各步命令见对应小节，均已给出可复制代码块。

## 5.4 参考资料

**上游源码（本机 clone：`.dsh.local/scratch/CubeSandbox`，commit `80614ab`）**
- [本地构建部署](.dsh.local/scratch/CubeSandbox/docs/zh/guide/self-build-deploy.md) — 从源码构建部署的权威依据
- [裸金属部署](.dsh.local/scratch/CubeSandbox/docs/zh/guide/bare-metal-deploy.md) — ARM64 限制与手工程序
- [下载与 Release](.dsh.local/scratch/CubeSandbox/docs/zh/guide/downloads.md) — 资产命名与版本 pin
- [release-assets.yaml](.dsh.local/scratch/CubeSandbox/deploy/release-assets.yaml) — 内核/镜像 tag 钉死
- [env.example](.dsh.local/scratch/CubeSandbox/deploy/one-click/env.example) — 全部目标机变量
- [build.env.example](.dsh.local/scratch/CubeSandbox/deploy/one-click/build.env.example) — 构建期变量
- [common.sh](.dsh.local/scratch/CubeSandbox/deploy/one-click/lib/common.sh) — readonly 安装根（`:9-12`）
- [cube-bench](.dsh.local/scratch/CubeSandbox/examples/cube-bench/README.md) — 并发创建基准
- [bench_pause_resume_concurrency.py](.dsh.local/scratch/CubeSandbox/examples/snapshot-rollback-clone/bench_pause_resume_concurrency.py) — pause/resume 基准
- [官方性能基线报告](.dsh.local/scratch/CubeSandbox/docs/zh/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md) — **仅 x86_64**

**上游 issue / PR**
- [#1803](https://github.com/TencentCloud/CubeSandbox/issues/1803) — ARM64 模板顺序导致延迟劣化 1.2–1.7×
- [#1865](https://github.com/TencentCloud/CubeSandbox/pull/1865) — aarch64 快照恢复 ioctl 风暴
- [#1826](https://github.com/TencentCloud/CubeSandbox/pull/1826) — balloon 上报 aarch64 默认关
- [#1658](https://github.com/TencentCloud/CubeSandbox/pull/1658) — ARM pause→resume 修复（v0.7.2）
- [#1611](https://github.com/TencentCloud/CubeSandbox/issues/1611) — aarch64 96 核 cube-bench 吞吐异常

**平台资料**
- [Huawei Connect 2025 keynote](https://www.huawei.com/en/news/2025/9/hc-xu-keynote-speech) — 鲲鹏 950 两个 SKU（96C/192T、192C/384T）
- [KPBot BIOS 优化 playbook（openEuler 社区）](https://gitcode.com/openeuler/KPBot) — 鲲鹏 BIOS 选项矩阵
- [华为云 ECS 规格文档](https://support.huaweicloud.com/drawer-ecs/ecs_parameter_0309.html) — 鲲鹏灵动核默认关闭

## 5.5 `collect_baseline.py` 说明

| 参数 | 说明 |
|---|---|
| `--out FILE` | 输出 JSON 路径（默认 `baseline-<arch>.json`） |
| `--pretty` | 美化输出 |
| `--quiet` | 只输出 JSON，不打印进度 |

收集内容对应第 2 篇：CPU/SKU/SMT、NUMA(`numactl -H`)、内存与速率、页面大小/THP/大页、内核与 cmdline、governor、漏洞缓解、KVM 能力、firmware/BIOS、磁盘与文件系统、Docker 版本与镜像 digest、CubeSandbox 安装版本（若已装）。

脚本**只读**系统信息，不修改任何配置。
