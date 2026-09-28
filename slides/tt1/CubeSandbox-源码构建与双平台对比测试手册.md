# CubeSandbox 源码构建与双平台对比测试手册

在两台裸金属服务器（x86_64 与鲲鹏 950）上从源码构建并部署 CubeSandbox，然后用完全一致的方法测出可以互相对比的数据。

适用平台：x86_64（AMD EPYC 9654 / 9755 级）、aarch64（鲲鹏 950）

---

# 第 1 篇　开始之前

## 1.1 CubeSandbox 是什么，部署完会得到什么

CubeSandbox 是腾讯云开源的 AI Agent 安全沙箱服务（Apache 2.0）。它用 Rust 编写的轻量虚拟机监控器（基于 KVM）为每次代码执行拉起一个 MicroVM，做到毫秒级冷启动、高密度共存，并对外提供兼容 E2B 协议的 HTTP API。

单机部署完成后的组件关系：

```
        压测客户端 / Agent 代码
                    │  HTTP（E2B 协议）
                    ▼
        ┌───────────────────────┐
        │  cube-api  :3000      │  E2B 兼容 REST API，创建 / 销毁沙箱
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │  CubeMaster :8089     │  调度与编排：决定沙箱落在哪个节点
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │  Cubelet              │  节点 Agent：在本机拉起 MicroVM
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │ containerd-shim-cube  │  容器运行时 shim
        │ + cube-runtime        │
        └───────────┬───────────┘
                    ▼
        ┌───────────────────────┐
        │ hypervisor（Rust VMM）│  基于 /dev/kvm 启动 MicroVM
        └───────────┬───────────┘
                    ▼
              MicroVM + cube-agent

  支撑组件（Docker 容器）：
    cube-sandbox-mysql    :3306    元数据
    cube-sandbox-redis    :6379    缓存与注册
    cube-sandbox-minio    :9000    对象存储
    cube-proxy            :80/443  cube.app 域名路由与 TLS
    cube-proxy-coredns     DNS     域名解析
    cube-webui            :12088   管理界面
```

各组件职责：

| 组件 | 职责 | 语言 |
|---|---|---|
| `cube-api` | 对外唯一 API 入口，兼容 E2B SDK | Rust |
| `CubeMaster` | 调度器：分配沙箱到节点，管理模板 | Go |
| `Cubelet` | 节点 Agent：本机拉起 / 暂停 / 销毁 MicroVM | Go（CGO 调 `cubecow`） |
| `containerd-shim-cube-rs` / `cube-runtime` | 容器运行时 shim 与 runtime | Rust |
| `hypervisor` | MicroVM 监控器（RustVMM + KVM） | Rust |
| `cube-agent` | guest 内 Agent，由 shim 注入 | Rust |
| `cubecow` | 基于 XFS `FICLONE` 的写时复制快照库 | Rust |
| `cube-proxy` + `coredns` | `cube.app` 域名路由、TLS（mkcert） | 容器 |
| MySQL / Redis / MinIO | 元数据 / 缓存 / 对象存储 | 容器 |

## 1.2 这次测试要回答什么、产出什么数据

| 测试项 | 回答的问题 | 主要指标 |
|---|---|---|
| 创建延迟 | 从请求到沙箱可用要多久 | `create` 的 `avg / min / p95 / p99` |
| 并发扩展 | 并发拉高后吞吐与延迟如何变化 | 100 并发下的 wall、吞吐、成功率 |
| Pause / Resume | 暂停与恢复有多快 | `pause_*` / `resume_*` 的 `avg / min / p95 / max`、`per_*` |

两台机器上运行的 CubeSandbox 版本、沙箱规格、测试方法必须一致，否则测出的是配置差异而不是平台差异。第 2 篇列出必须对齐和必须记录的项。

`checking_points.xlsx` 是本次测试的输入表，用于登记两台机器各自的版本与环境关键信息；性能结果单独归档，两者的取值方式见 4.6。

## 1.3 术语表

| 术语 | 全称 | 含义 |
|---|---|---|
| BM | Bare Metal | 裸金属物理机 |
| PVM | Pagetable-based Virtual Machine | 让普通云服务器获得 KVM 的方案，仅 x86_64 |
| NPS | NUMA Nodes Per Socket | AMD 每插槽 NUMA 分区数（0/1/2/4） |
| SCCL | Super CPU Cluster | 鲲鹏的 NUMA 粒度单位 |
| UFS | Unified Fabric Switch | 鲲鹏互联调度特性 |
| LPI | Low Power Idle | ARM 深度空闲状态 |
| SMMU | System MMU | ARM 的 IOMMU |
| CoW | Copy-on-Write | 写时复制（XFS reflink / CubeCoW） |
| THP | Transparent Huge Page | 透明大页 |
| wall | wall-clock time | 整批端到端耗时 |
| per | per-operation | 均摊耗时（wall ÷ 操作数） |

---

# 第 2 篇　公平性基线

## 2.1 为什么两台机器必须一致

对比的目标是「平台差异」。如果两边的 CubeSandbox 版本、BIOS 策略、内核参数、沙箱规格不同，测出的差异会混入大量配置差异，无法归因。因此遵循三条原则：

| 原则 | 要求 |
|---|---|
| 同版本 | 两台机器从同一个 tag 构建 CubeSandbox |
| 同协议 | 同样的并发档位、同样的热身与轮次规则、同样的沙箱规格 |
| 同基线 | BIOS / 内核 / 大页 / NUMA 设置按本篇对齐并记录 |

上游官方性能报告只有 x86_64 数据（Intel Xeon 8255C），不能当作鲲鹏 950 的基线，也不能与本次实测混合比较。

## 2.2 BIOS / 固件对照清单

| 目标 | x86_64（AMD） | 鲲鹏 950 |
|---|---|---|
| 消除频率不确定性 | Determinism Control = Performance | Power Profile = Performance |
| 放开功耗墙 | TDP/PPT = Manual，值按机型（9654→400 / 9755→500） | Turbo(Core) = On；Turbo(Uncore) = On |
| 关深度空闲 | Global C-state Control = Disabled（或限 C1） | C-State = C0/C1；LPI = Off |
| 干净的 NUMA | NPS = 1 或 4（选定并记录） | Node Interleaving = Off；UFS = Off |
| 睿频 | BoostFmaxEn = Manual；BoostFmax = Auto | Turbo(Core) = On |
| 虚拟化 | SVM = Enabled；SR-IOV = Enabled | SMMU = Enabled |
| 内存速率 | 额定满速 | DDR Speed = 额定满速 |
| PCIe 省电 | ASPM Disabled | PCIe ASPM = Off |
| 超线程 | 与鲲鹏侧保持同策略 | 与 x86 侧保持同策略 |

鲲鹏侧的 Power Profile 与 Hardware Prefetcher **无法从操作系统读取**，必须导出 Redfish BIOS Attributes JSON 或拍摄 BIOS 截图归档。SMT 变更需要冷重启并重新规划 CPU 亲和性。

## 2.3 OS / 内核 / 大页 / NUMA 设置清单

| 项目 | 要求 | 查看命令 |
|---|---|---|
| 发行版 | 两边同版本 | `cat /etc/os-release` |
| 内核 | 记录精确版本 | `uname -r` |
| 内核 cmdline | 归档 | `cat /proc/cmdline` |
| 大页 | 两边显式设成同一组参数并记录 | `grep -i huge /proc/meminfo` |
| 透明大页 | 两边同策略 | `cat /sys/kernel/mm/transparent_hugepage/enabled` |
| NUMA balancing | 两边一致（建议 0） | `cat /proc/sys/kernel/numa_balancing` |
| CPU governor | 两边一致（performance） | `cpupower frequency-info` |
| 基础页大小 | 记录 | `getconf PAGESIZE` |
| 漏洞缓解状态 | 记录 | `cat /sys/devices/system/cpu/vulnerabilities/*` |

CubeSandbox 的 `.env` 中没有任何大页、NUMA 或 cpuset 变量，这些全部是宿主机层设置，通过内核 cmdline、`numactl`、`cpupower` 完成。

## 2.4 必须记录的证据

两台机器在部署前各执行一次：

```bash
python3 collect_baseline.py --out baseline-$(uname -m).json
```

收集 CPU 型号与 SKU、SMT 状态、NUMA 拓扑、内存与通道速率、页面大小与 THP、大页、内核与 cmdline、governor、漏洞缓解、KVM 与虚拟化能力、firmware/BIOS、磁盘与文件系统、Docker 版本与镜像 digest。字段说明见 5.3。

## 2.5 无法对齐的架构差异

以下差异由架构决定，无法消除，只能如实记录并在解释数据时说明。

| 差异 | 说明 | 处理方式 |
|---|---|---|
| 基础页大小 | x86 通常 4 KB；arm64 可能 4/16/64 KB。影响 TLB、THP、CoW 粒度与快照脏页粒度 | 记录；快照类指标按实际脏页字节数归一化 |
| guest 内核 | 两平台是不同文件、不同 config | 记录双方 sha256 |
| 中断控制器恢复成本 | aarch64 恢复快照时按字重放 vGIC 寄存器，x86 无此结构 | 把 restore 阶段单独报告 |
| balloon 上报默认值 | aarch64 与 x86_64 默认值不同，影响密度 | 两边显式设成同值并记录 |
| guest PMU | 部分 ARM 主机不向 guest 暴露 PMUv3 | 不用 guest 内硬件计数器做对比 |
| ISA | AVX-512 与 NEON/SVE 不同 | 涉及向量化的负载单独说明 |

## 2.6 红线清单

出现以下任一情况，对比数据无效。

| # | 红线 | 原因 |
|---|---|---|
| R1 | 一边用 PVM、一边用原生 KVM | PVM 仅 x86_64，是另一条 hypervisor 路径，比的不是 CPU |
| R2 | 两边 commit 不同 | 版本差异可能远大于平台差异 |
| R3 | SMT / 线程数不一致且未记录 | 线程数可能相差一倍 |
| R4 | NUMA 模式不一致且未记录 | 远端内存占比不同 |
| R5 | 工作负载镜像不是 Multi-Arch | 一边可能无法运行，或运行的是不同 rootfs |
| R6 | 模板构建顺序两边不同 | aarch64 上模板顺序本身会造成延迟漂移 |
| R7 | 拿官方约 48 ms 当 ARM 基线 | 那是 Intel x86_64 的数字 |
| R8 | CubeVS eBPF 的挂载状态未经检查 | 网络指标会静默走退化路径 |
| R9 | 拿 guest 内 PMU 指标对比 | ARM guest 可能没有 PMU |

---

# 第 3 篇　部署

## 3.0 部署总览

```
3.1 构建机准备 ──► 3.2 准备 guest 内核 ──► 3.3 源码构建发布包
                                                    │
                                                    ▼
                                        3.4 目标机安装 ──► 3.5 部署验收
```

部署完成判据（全部满足才算完成）：

1. `systemctl list-units 'cube-sandbox-*'` 中 control target 为 active；
2. `curl http://127.0.0.1:<CUBE_API 端口>/health` 返回成功；
3. MySQL、Redis、MinIO、cube-proxy、coredns、webui 容器均为 running；
4. 3.5 的证据文件已归档。

部署产出的下述内容将被后续步骤消费：

| 产出 | 由哪一步产生 | 被谁使用 |
|---|---|---|
| 基线证据 JSON | 2.4 | 结果报告 |
| `vmlinux` 及其 sha256 | 3.2 | 3.3 构建 |
| `release-manifest.json`、`VERSION.txt` | 3.3 | 3.4 安装校验、4.6 登记 |
| 运行中的服务与端口 | 3.4 | 4.1～4.5 全部测试 |
| 安装后指纹文件 | 3.5 | 4.6 登记、排查问题 |

## 3.1 构建机准备

构建机与目标机可以是同一台，也可以分开。目标机为共享服务器时建议分开。

### 3.1.1 配置代理

构建过程需要访问 Docker Hub、apt 源、go.dev、crates.io、npm 等外部源，后面的步骤都依赖网络，因此先配置代理。由于不同工具读取的环境变量名不同（`apt` / `dnf` 读小写 `http_proxy`，`curl` / `skopeo` 读大写 `HTTPS_PROXY`），这里把大小写一次性都设置好，后续命令直接使用，不再逐个加前缀。

```bash
export HTTPS_PROXY="http://<proxy-host:port>"
export HTTP_PROXY="$HTTPS_PROXY"
export https_proxy="$HTTPS_PROXY"
export http_proxy="$HTTPS_PROXY"
export NO_PROXY="localhost,127.0.0.1,192.168.0.0/16"
export no_proxy="$NO_PROXY"
export GOPROXY="https://goproxy.cn,direct"
```

验证：

```bash
curl -sI https://go.dev/dl/ | head -1
```

应输出 `HTTP/2 200` 或 `HTTP/1.1 200` 之类的一行状态行。若长时间无输出或返回 403，检查代理是否允许 HTTPS CONNECT，以及 `NO_PROXY` 是否误排除了目标。

### 3.1.2 安装系统依赖

构建脚本自身只用 `docker` 和 `make`，但它随后会在宿主机上生成 guest 镜像并打包，因此宿主机还需要 `git`、`python3`、`e2fsprogs`（`mkfs.ext4`）、`tar`、`pigz`、`sudo`。其中 `npm` 用于构建 WebUI，而 WebUI 不在 builder 容器内编译，宿主机缺少 `npm` 会导致构建在最后阶段失败；`skopeo` 用于在拉不到 Docker Hub 时从其它镜像源搬运镜像。

```bash
# openEuler / RHEL 系
dnf install -y docker make git python3 tar pigz gzip sudo e2fsprogs util-linux which skopeo

# Debian / Ubuntu 系
apt-get update && apt-get install -y docker.io make git python3 tar pigz gzip sudo e2fsprogs util-linux which skopeo
```

验证：

```bash
docker --version && make --version | head -1 && git --version && python3 --version && mkfs.ext4 -V 2>&1 | head -1
```

应依次打印 docker、make、git、python3 的版本号，最后一行是 mke2fs 的版本。若某个命令报 `command not found`，说明该包未装上，需要按发行版补装；若 `mkfs.ext4` 版本过旧不支持 `-d`，guest 镜像生成会失败。

### 3.1.3 预拉全部镜像

构建与安装需要的镜像在这里一次性拉完，避免后续重复走网络。其中 builder 基座 `ubuntu:20.04` 与 guest 镜像基座 `tencentos/tencentos4-minimal` 供 3.3 编译使用，两者的 Dockerfile 直接引用这两个原名，因此拉取时必须打回同名标签；`mysql`、`redis`、`minio` 由 support 组件启动，供 CubeMaster 存元数据、缓存与对象存储；`coredns` 由 cube-proxy 启动，负责 `cube.app` 域名解析；`openresty` 是 WebUI 的 nginx 基座；`docker/compose:1.29.2` 是各 compose 包装脚本实际调用的容器，它来自 Docker Hub，最容易被漏掉。构建机与目标机为同一台时本步骤只做一次，分成两台时两台各需一份。

```bash
# 编译用基座镜像
skopeo copy "docker://<your-mirror>/ubuntu:20.04" \
  "docker-daemon:ubuntu:20.04"
skopeo copy "docker://<your-mirror>/tencentos/tencentos4-minimal:latest" \
  "docker-daemon:tencentos/tencentos4-minimal:latest"

# 运行时镜像
for img in \
  "cube-sandbox-image.tencentcloudcr.com/opensource/coredns/coredns:1.14.2" \
  "cube-sandbox-image.tencentcloudcr.com/opensource/mysql:8.0" \
  "cube-sandbox-image.tencentcloudcr.com/opensource/redis:7-alpine" \
  "cube-sandbox-image.tencentcloudcr.com/opensource/openresty:1.21.4.1-6-alpine-fat" \
  ; do
  skopeo copy "docker://$img" "docker-daemon:$img"
done

skopeo copy "docker://<your-mirror>/docker/compose:1.29.2" \
  "docker-daemon:docker/compose:1.29.2"
```

验证：

```bash
docker images | grep -E 'ubuntu|tencentos|coredns|mysql|redis|openresty|compose'
```

应能看到上面每一个镜像各一行。缺少哪个，对应的编译或安装阶段就会失败，补拉该镜像即可。

### 3.1.4 获取源码

从 GitHub 拉取 v0.7.1 的源码。

```bash
git clone --depth 1 --branch v0.7.1 \
  https://github.com/TencentCloud/CubeSandbox.git /path/to/CubeSandbox
cd /path/to/CubeSandbox
```

验证：

```bash
git describe --tags --abbrev=0 && git rev-parse HEAD && git status --porcelain
```

第一行应输出 `v0.7.1`，第二行是 40 位 commit id，第三行应无任何输出（工作树干净）。若第三行有输出，说明工作树被改动过，需要确认改动来源后再继续。

## 3.2 准备 guest 内核

### 3.2.1 放置 `vmlinux`

guest MicroVM 需要的内核文件必须放在 `deploy/one-click/assets/kernel-artifacts/vmlinux`，缺失时构建脚本会直接退出。内核 tag 由仓库中的 `deploy/release-assets.yaml` 钉死，按目标架构取对应文件即可，两平台的文件不同。

```bash
ARCH=$(uname -m); case "$ARCH" in x86_64) A=amd64;; aarch64) A=arm64;; esac
KERNEL_TAG=$(grep -E "^kernel_bm_${A}:" deploy/release-assets.yaml | awk '{print $2}')

mkdir -p deploy/one-click/assets/kernel-artifacts
curl -L -o deploy/one-click/assets/kernel-artifacts/vmlinux \
  "https://cnb.cool/CubeSandbox/CubeSandbox/-/releases/download/${KERNEL_TAG}/vmlinux-${A}"
```

验证：

```bash
ls -l deploy/one-click/assets/kernel-artifacts/vmlinux && sha256sum deploy/one-click/assets/kernel-artifacts/vmlinux
```

应看到文件存在（约 26 MB 或 50 MB，随架构不同），并打印一行 sha256。若文件大小为 0 或第一行报 No such file，说明 tag 取错或网络失败。该 sha256 需要记录，用于两台机器互相核对。

## 3.3 源码构建发布包

### 3.3.1 配置 `build.env`

从模板复制构建配置，保持 `local` 模式才会从源码编译各组件。

```bash
cp deploy/one-click/build.env.example deploy/one-click/build.env
```

验证：

```bash
grep -E 'BUILD_MODE|BUILDER_IMAGE|KERNEL_VMLINUX' deploy/one-click/build.env
```

应能看到若干 `ONE_CLICK_*_BUILD_MODE=local` 行。若某一行不是 `local`，该组件会改用预编译二进制。

### 3.3.2 构建发布包

先用统一的 builder 镜像固定工具链版本（Go、Rust、clang 等），再用它编译全部组件并在宿主机上生成 guest 镜像、打包成发布包。这一步产出后续所有步骤都依赖的 tar 包。

```bash
make builder-image
./deploy/one-click/build-release-bundle-builder.sh 2>&1 | tee /tmp/cube-build-$(uname -m).log
```

验证：

```bash
BUNDLE=$(ls deploy/one-click/dist/cube-sandbox-one-click-*.tar.gz | head -1)
echo "$BUNDLE"
sha256sum "$BUNDLE"
tar -tzf "$BUNDLE" | grep -E 'VERSION.txt|release-manifest.json|install.sh|env.example'
```

第一行应打印出 tar 包路径，第二行是它的 sha256，随后四行应分别列出 `VERSION.txt`、`release-manifest.json`、`install.sh`、`env.example`。若 `ls` 报找不到文件，查看 `/tmp/cube-build-*.log` 的末尾定位失败阶段。

### 3.3.3 归档构建元数据

发布包里的 `VERSION.txt` 与 `release-manifest.json` 记录了本次构建的 commit、各组件版本与 digest、guest 镜像与内核的摘要，是两台机器核对「跑的是不是同一个东西」的依据，也是 4.6 填表的直接来源。

```bash
OUT=/tmp/cube-artifacts-$(uname -m); mkdir -p "$OUT"
tar -xzf "$BUNDLE" -C "$OUT" --strip-components=1 \
  --wildcards '*/VERSION.txt' '*/release-manifest.json' 2>/dev/null || true
cat "$OUT/VERSION.txt"
```

验证：

```bash
python3 -m json.tool "$OUT/release-manifest.json" | head -30
```

应打印出格式化的 JSON，且顶层能看到 `components`、`guest_image`、`kernel` 三个键。缺任一个键会导致 3.4.4 安装时报 `invalid release manifest`。

## 3.4 目标机安装

### 3.4.1 安装前检查

安装脚本自身会做这些检查，但提前发现比在安装中途失败代价低；其中端口占用安装脚本不会检查，必须在这里排查。下面命令的输出直接给出每一项的结果，对照后面的表判断即可，结果决定 3.4.3 是否需要改端口、是否需要处理文件系统。

```bash
ls -la /dev/kvm
grep MemTotal /proc/meminfo
ldd --version | head -1
grep bpf /proc/filesystems; df -T /sys/fs/bpf
stat -fc %T /sys/fs/cgroup
lsblk -o NAME,SIZE,FSTYPE,MOUNTPOINT
df -hT / /data 2>/dev/null

for p in 3000 8082 8089 3010 3306 6379 9000 9001 80 443 9090 9091 12088 8083; do
  ss -lntp "( sport = :$p )" | tail -n +2 | sed "s/^/port $p: /"
done
```

对照下表判断结果：

| 检查项 | 要求 | 不满足时 |
|---|---|---|
| `/dev/kvm` | 存在 | 不是物理机或 KVM 未开启，无法继续 |
| 内存 | ≥ 7500000 KB | `CUBE_MIN_MEMORY_KB` 只能调高不能调低 |
| glibc | ≥ 2.31 | 系统过旧 |
| bpffs | `/sys/fs/bpf` 类型为 `bpf` | eBPF 无法 pin |
| cgroup | v2 需有 `cpu` 控制器；v1 直接通过 | 启动会失败 |
| `/data` 所在 FS | XFS | 需要按 3.4.2 处理 |
| 端口 | 空闲 | 按 3.4.3 改端口 |

### 3.4.2 准备磁盘布局

有两个硬约束。第一，`/data/cubelet` 必须在 XFS 上，安装脚本会向上查找最近的已存在目录并检查文件系统类型，不是 XFS 就直接退出。第二，安装根被写死为 `/usr/local/services/cubetoolbox` 且在脚本中设为只读，写进 `.env` 会被静默忽略，因此当根分区空间不足时，只能通过挂载把该路径指向大分区。这两处都由下面的 bind mount 解决。

```bash
mkdir -p /mnt/bss/cube-data /data
mount --bind /mnt/bss/cube-data /data

mkdir -p /home/cubetoolbox /usr/local/services/cubetoolbox
mount --bind /home/cubetoolbox /usr/local/services/cubetoolbox

echo "/mnt/bss/cube-data /data none bind 0 0" >> /etc/fstab
echo "/home/cubetoolbox /usr/local/services/cubetoolbox none bind 0 0" >> /etc/fstab
```

验证：

```bash
df -T /data && df -T /usr/local/services/cubetoolbox && xfs_info /data | grep reflink
```

第一行应显示 `/data` 的类型为 `xfs`，第二行显示安装根已指向目标分区，第三行应输出 `reflink=1`。若第一行不是 xfs，说明 `/mnt/bss/cube-data` 所在分区不是 XFS，需要换一个 XFS 分区重新绑定。

### 3.4.3 配置 `.env`

解压发布包并生成配置文件。端口、节点 IP、镜像区域、是否重启 Docker 等都由 `.env` 决定，其中 `ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR` 一旦设为 `1`，安装脚本会改写 `/etc/docker/daemon.json` 并重启 Docker，共享服务器上会停掉所有 `RestartPolicy=no` 的容器，因此必须保持 `0`。

```bash
tar -xzf cube-sandbox-one-click-<version>.tar.gz
cd cube-sandbox-one-click-<version>
cp env.example .env
```

按 3.4.1 的结果编辑 `.env`：

```bash
CUBE_SANDBOX_NODE_IP=192.168.x.x
MIRROR=cn

# 若 3000 / 8082 被占用
CUBE_API_BIND=0.0.0.0:3001
CUBE_API_HEALTH_ADDR=127.0.0.1:3001
CUBE_PROXY_ADMIN_PORT=8083

ONE_CLICK_DEPLOY_ROLE=control
ONE_CLICK_ASSUME_YES=1
ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR=0
CUBE_PVM_ENABLE=0
ONE_CLICK_ENABLE_S3LVOL=0
```

验证：

```bash
grep -E 'CUBE_API_BIND|CUBE_API_HEALTH_ADDR|CUBE_PROXY_ADMIN_PORT|CUBE_PVM_ENABLE|S3LVOL|TENCENT_DOCKER' .env
```

应能列出上面设置的各行；其中 `CUBE_API_BIND` 与 `CUBE_API_HEALTH_ADDR` 的端口必须一致，否则健康检查会失败。`ONE_CLICK_ENABLE_S3LVOL=0` 表示不创建约 512 GiB 的 WAL 镜像。

### 3.4.4 运行安装

安装脚本完成解压组件、写配置、安装 systemd unit、启动服务与健康检查的全过程。

```bash
./install.sh --mode=install 2>&1 | tee /var/log/cube-install-$(uname -m).log
```

验证：

```bash
tail -20 /var/log/cube-install-$(uname -m).log
```

日志末尾应出现健康检查通过的信息，且无 `die` 或 `ERROR` 字样。

常见失败与处理：

| 现象 | 原因 | 处理 |
|---|---|---|
| 提示 `/data/cubelet` 不是 XFS | 3.4.2 未做 | 回到 3.4.2 |
| `port ... already in use` | 端口冲突 | 回到 3.4.3 改端口 |
| `invalid release manifest` | manifest 结构不符 | 检查 3.3.3 |
| DNS preflight 失败 | 缺少 resolvectl 或 NetworkManager | 装 dnsmasq 或设 `CUBE_PROXY_DNSMASQ_MODE=standalone` |

## 3.5 部署验收

确认系统可用，并归档安装后指纹。指纹文件是事后归因问题的依据。

```bash
systemctl list-units 'cube-sandbox-*' --no-pager
ss -tlnp | grep -E ':(3001|8089|3010|8083|12088)'
curl -s http://127.0.0.1:3001/health || curl -s http://127.0.0.1:3010/health
docker ps --format '{{.Names}}\t{{.Status}}'
./smoke.sh

python3 collect_baseline.py --out postinstall-$(uname -m).json
{ systemctl list-units 'cube-sandbox-*' --no-pager
  docker ps --format '{{.Names}}\t{{.Image}}\t{{.Status}}'
  docker images --digests
  cat /usr/local/services/cubetoolbox/VERSION.txt 2>/dev/null
  head -c 3000 /usr/local/services/cubetoolbox/release-manifest.json 2>/dev/null
} > /tmp/postinstall-$(uname -m).txt 2>&1
```

验证：

```bash
ls -l /tmp/postinstall-$(uname -m).txt postinstall-$(uname -m).json
```

应显示两个归档文件存在且非空。上面 `curl` 返回健康状态内容即为通过；若无输出，按 3.0 的完成判据逐项核对，未通过不要进入第 4 篇。

---

# 第 4 篇　测试

## 4.0 测试总览

```
4.1 建模板 ──► 4.2 冒烟 ──► 4.3 创建并发（100）──┐
                  │                              ├──► 4.6 结果记录与汇总
                  └──────► 4.4 Pause/Resume（10）┘
                            （4.5 可选：快照 / 密度）
```

两台机器固定使用同一组参数：

| 参数 | 值 |
|---|---|
| 创建并发档位 | 100 |
| 每档迭代数 `-n` | c × 25 = 2500 |
| 热身 | `-w 3`（结果丢弃） |
| 正式轮次 | 3 轮，取 `avg` 平均 |
| Pause/Resume 并发 | 10 |
| Pause/Resume 测量轮数 | `-n 5` |
| 沙箱规格 | 2 vCPU / 2 GiB |
| 模板镜像 | `sandbox-code:latest`，按 digest 钉死 |

每轮开始前执行复位：

```bash
cubemastercli box list 2>/dev/null || true
sleep 30
```

在 aarch64 上，同一镜像与配置的模板会随构建顺序劣化，延迟差别可达 1.2～1.7 倍，因此鲲鹏侧每轮正式测量前必须冷重启并重建模板，同时记录模板构建顺序；x86 侧不受此影响，但两边的模板构建顺序应保持一致。

## 4.1 建模板

创建沙箱必须指定模板，后续所有测试都复用这一个模板。必须带 `--probe 49999`，否则容器虽然无法启动也会被判定为 READY。

```bash
cubemastercli tpl create-from-image \
  --image cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/sandbox-code:latest \
  --writable-layer-size 1G \
  --expose-port 49999 --expose-port 49983 \
  --probe 49999

cubemastercli tpl watch --job-id <job_id>
cubemastercli tpl list | tee /tmp/tpl-$(uname -m)-$(date +%H%M).txt
```

`tpl list` 输出中状态为 `READY` 的那一行，其第一列就是 `tpl-` 开头的模板 ID，记下它作为后续的 `CUBE_TEMPLATE_ID`。若长时间没有 READY，用 `tpl watch` 查看卡在哪个阶段。

## 4.2 冒烟测试

先用单并发创建 1 个沙箱确认端到端可用，把环境问题与性能问题分开。

```bash
export E2B_API_URL=http://127.0.0.1:3001
export E2B_API_KEY=e2b_000000
export CUBE_TEMPLATE_ID=<模板ID>

cd examples/cube-bench && make
./bin/cube-bench -m create-only -n 1 -c 1 -w 0
```

命令输出的 `Success Rate` 应为 `100.0%`。若失败，先查 cubelet 日志与模板状态，不要直接进入并发测试。

## 4.3 创建延迟与并发扩展

在 100 并发下压测沙箱创建，跑 3 轮。延迟的计时口径是「从发出请求到收到响应头」，不含读取与解析响应体。

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

验证：

```bash
for f in /tmp/bench-$(uname -m)/create-c100-r*.json; do
  python3 -c "
import json; d=json.load(open('$f'))
print('$f', d['summary']['success_rate'], round(d['create']['avg'],1), round(d['create']['p95'],1), round(d['summary']['throughput_qps'],1))
"
done
```

应打印 3 行，每行依次是文件名、成功率、create 平均值、create p95、吞吐。成功率应为 1.0，三行的平均值不应相差过大；若某一轮明显偏离，先怀疑环境不稳定而不是平台差异。

## 4.4 Pause / Resume

`cube-bench` 只能测创建与删除，没有 pause/resume 能力，这一步使用官方 Python 脚本。

```bash
cd examples/snapshot-rollback-clone
pip install -r requirements.txt

export CUBE_API_URL=http://127.0.0.1:3001
export CUBE_TEMPLATE_ID=<模板ID>

python3 bench_pause_resume_concurrency.py -c 10 -n 5 | tee /tmp/pause-resume-$(uname -m).txt
```

验证：

```bash
awk 'NF>0 {print "columns:", NF}' /tmp/pause-resume-$(uname -m).txt
```

应打印一行 12 列的数据，依次为并发数、轮数、pause 的 avg/min/p95/max、per_pause、resume 的 avg/min/p95/max、per_resume，单位为毫秒。若脚本报错或列数不足，检查 `CUBE_API_URL` 与 SDK 是否装好。pause 当前采用全内存拷贝，耗时会随沙箱内存线性增长，解释数据时需要说明这一点。

## 4.5 快照 / 回滚 / 克隆、密度测试

需要这几项指标时，使用同一目录下的其它官方脚本，用法与 4.4 相同。

```bash
python3 bench_snapshot_concurrency.py -c 10 -n 5 | tee /tmp/snapshot-$(uname -m).txt
python3 bench_rollback_concurrency.py -c 10 -n 5 | tee /tmp/rollback-$(uname -m).txt
python3 bench_clone_concurrency.py    -c 10 -n 5 | tee /tmp/clone-$(uname -m).txt
```

验证：

```bash
awk 'NF>0 {print FILENAME, "columns:", NF}' /tmp/snapshot-$(uname -m).txt /tmp/rollback-$(uname -m).txt /tmp/clone-$(uname -m).txt
```

应为三个文件各打印一行，列数均大于 0。快照类指标与页大小相关，两台机器的基础页大小若不同，需要按实际脏页字节数归一化后再比较。密度测试前，两边的 balloon 上报开关必须设成同值，否则内存开销不可比。

## 4.6 结果记录与汇总

`checking_points.xlsx` 是本次测试的输入表，用来登记两台机器各自是在什么版本与什么环境下测的；压测产出的性能数据不属于这张表，另存为独立文件。两者分开保存：前者说明数据的来源，后者是数据本身。

### 4.6.1 回填 `checking_points.xlsx`

表中各项按下面的来源填写，两台机器各填一列。

| 表格行 | 取值来源 |
|---|---|
| 测试版本（commit / tag） | `git describe` 与 `git rev-parse HEAD` |
| CubeSandbox repo 版本 | `release-manifest.json` 的 `release_version` |
| docker / docker-compose 版本 | `docker --version`、`docker-compose --version` |
| 镜像版本 | `release-manifest.json` 各组件 version 与 digest；`docker images --digests` |
| guest 镜像 / 内核 | `release-manifest.json` 的 `guest_image`、`kernel` |
| 内核版本 / 大页 / NUMA / 核数 | `baseline-<arch>.json` 与 `postinstall-<arch>.json` |
| OS 版本 | `/etc/os-release` 与 `uname -r` |

其中机器环境项可以直接从归档的基线文件打印出来：

```bash
python3 -c "
import json
d=json.load(open('baseline-$(uname -m).json'))
print(json.dumps(d['_fairness_highlights'], ensure_ascii=False, indent=2))
"
```

输出包含该机器的架构、CPU 型号、逻辑核数、SMT 状态、NUMA 节点数、页大小与 governor，照此填入表中对应行。

### 4.6.2 保存测试结果

把 3 轮的压测结果汇总，写到独立的结果文件里，不写进输入表。

```bash
OUT=/tmp/bench-summary-$(uname -m).txt
python3 - <<'EOF' | tee "$OUT"
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

验证：

```bash
ls -l /tmp/bench-summary-*.txt && ls /tmp/bench-*/create-c100-r*.json | wc -l
```

第一行应显示每台机器的汇总文件存在且非空，第二行应输出 6（两个架构目录各 3 个报告）。若不足 6，说明某一轮未产出报告，需要补跑。

报告比值时必须附上第 2 篇的硬件与固件对照表。若轮间极差超过 20%，先排查环境稳定性。

## 4.7 多轮执行的检查清单

- [ ] 清掉上一轮残留沙箱
- [ ] 轮间静默 ≥ 30 s 已执行
- [ ] 鲲鹏侧：冷重启并重建模板
- [ ] 模板 ID 已更新，构建顺序已记录
- [ ] 并发档位、`-n`、`-w 3` 与另一台机器完全一致
- [ ] 本轮 JSON 已落盘并归档

## 4.8 测试完成后停机

安装脚本把这些 systemd 单元设为开机自启并常驻运行，测试结束后若只停不取消自启，Cubelet 与各容器仍会占用 CPU 与内存，机器重启后也会自动拉起。这一步同时取消自启并停止服务，把资源让给同机的其它工作。

```bash
cd <解压后的包目录>
./down.sh
systemctl disable cube-sandbox-control.target
systemctl disable cube-sandbox-s3lvol.service 2>/dev/null || true
```

验证：

```bash
systemctl is-enabled cube-sandbox-control.target; systemctl is-active cube-sandbox-control.target; docker ps --filter name=cube-sandbox --format '{{.Names}}' | wc -l
```

三行输出应依次为 `disabled`、`inactive`、`0`。若第一行仍是 `enabled`，说明自启未取消，重启后会再次占用 CPU 与内存；若第三行不为 0，用 `docker ps --filter name=cube-sandbox` 查出残留容器并单独处理。

---

# 第 5 篇　附录

## 5.1 双平台差异速查表

| 动作 | x86_64 | aarch64（鲲鹏 950） |
|---|---|---|
| 架构标识 | `uname -m` = `x86_64`，资产后缀 `amd64` | `uname -m` = `aarch64`，资产后缀 `arm64` |
| guest 内核资产 | `vmlinux-amd64` | `vmlinux-arm64` |
| guest 镜像资产 | `cube-guest-image-amd64.tar.gz` | `cube-guest-image-arm64.tar.gz` |
| 内核 config | `configs/kernel-oc9.x86_64.config` | `configs/kernel-oc9.aarch64.config` |
| PVM | 可用，但本手册不使用 | 不支持，必须原生 KVM 裸金属 |
| 热迁移 | 支持 | 仅 x86_64 |
| KVM 模块 | `kvm_amd` | `kvm` + GICv3 |
| 虚拟化扩展 | SVM / AMD-Vi | SMMU |
| 中断控制器 | x2APIC / IOAPIC | GICv3/GICv4 + ITS |
| 深度空闲 | Global C-state Control | LPI = Off |
| 互联相关 | 无对应项 | UFS = Off |
| 一键脚本 | 自动发现 | `online-install.sh` 不自动发现 ARM64 |

## 5.2 版本钉死清单

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

## 5.3 `collect_baseline.py` 说明

| 参数 | 说明 |
|---|---|
| `--out FILE` | 输出 JSON 路径，默认 `baseline-<arch>.json` |
| `--pretty` | 美化输出 |
| `--quiet` | 只输出 JSON，不打印进度 |

收集内容：CPU 型号与 SKU、SMT 状态、NUMA 拓扑、内存与速率、页面大小与 THP、大页、内核与 cmdline、governor、漏洞缓解、KVM 与虚拟化能力、firmware/BIOS、磁盘与文件系统、Docker 版本与镜像 digest、已安装的 CubeSandbox 版本。

脚本只读系统信息，不修改任何配置；缺少某个工具（如 `numactl`、`dmidecode`）时会在输出的 `_errors` 字段中记录，不影响其余字段采集。输出中的 `_fairness_highlights` 字段把最需要先比对的几项（架构、CPU 型号、SMT、NUMA 节点数、页大小、governor）单独抽出，便于两台机器并排核对。
