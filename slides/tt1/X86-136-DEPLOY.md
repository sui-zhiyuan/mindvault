# CubeSandbox x86_64 部署与百并发基线 — 80.48.23.136 执行手册

> **服务器**: 80.48.23.136 (root/PlatForm12#$)
> **架构**: x86_64 BM (裸金属, /dev/kvm 存在)
> **OS**: openEuler 24.03 LTS-SP3, kernel 6.6.0-132.0.0.111.oe2403sp3.x86_64
> **代理**: `http://80.253.48.42:3128` (可连 Google/GitHub/腾讯 CR/pypi, **不可连 Docker Hub**)
> **关键约束**: 共享服务器, 15 个容器运行中 (10 个 RestartPolicy=no), **不重启 Docker**

---

## 0. 环境摘要 (已实机验证)

| 项目 | 状态 | 说明 |
|------|------|------|
| CPU | AMD EPYC 9654 96核 ×2 = 384核 | 8 NUMA, 754GB RAM |
| / 分区 | ext4, 69G, **98% 满 (1.4G free)** | 不能放任何大数据 |
| /home | ext4, 29T, 6% (26T free) | Docker Root Dir = /home/docker-data |
| /mnt/bss | **XFS reflink=1**, 7T, 4% (6.8T free) | NVMe, 可写, 用于 /data |
| /tmp | tmpfs 378G | 充足 |
| Docker | 18.09.0, API 1.39, overlay2, cgroupfs | 不升级不重启 |
| docker-compose | **未安装** | 需 dnf 安装 v1.22.0 |
| skopeo | 1.14.2 **已安装** | 用于绕过 Docker Hub 拉取镜像 |
| dnsmasq | **未安装** | dnf 可装 (253KB) |
| Go | **未安装** | benchmark 阶段需要 |
| 网络 | enp194s0f0, 80.48.23.136/22 | 主网卡 |
| 端口 3000 | **被 multica-next.service 占用** | 需改 CUBE_API_BIND |
| 端口 8082 | **被 BSS_TM01_4U16G_1 容器占用** | 需改 CUBE_PROXY_ADMIN_PORT |
| TPROXY 内核模块 | xt_TPROXY.ko.xz + xt_socket.ko.xz 存在 | CONFIG_NETFILTER_XT_TARGET_TPROXY=m |
| eBPF | CONFIG_BPF=y/SYSCALL=y/JIT=y/LSM=y/SCHED=y | /sys/fs/bpf 已挂载 |
| cgroup | v1 | install.sh 接受 v1 (源码确认跳过 v2 检查) |
| SELinux | Permissive | 不阻止 |
| 防火墙 | inactive | 不阻止 |

### 共享服务器影响评估

| 操作 | 影响现有容器? | 原因 |
|------|-------------|------|
| 安装 docker-compose | ❌ 不影响 | dnf 安装, 不碰 Docker daemon |
| 安装 dnsmasq | ❌ 不影响 | dnf 安装, 不碰 resolv.conf |
| skopeo 拉取镜像 | ❌ 不影响 | 写 Docker image store, 不重启 daemon |
| bind mount /data | ❌ 不影响 | 新建挂载点 |
| 运行 install.sh | ❌ 不影响 | 不重启 Docker (源码确认) |
| TPROXY 规则 | ❌ 不影响 | 仅匹配 cube-dev 接口 + cube fwmark |
| DNS 接管 | ❌ 不影响 | 15 个容器 DNS 独立 (8.8.8.8 / 127.0.0.11) |

---

## 1. 执行流程 (10 步)

### Step 1: 设置代理 + 安装基础软件 (≈3 min)

```bash
# 代理变量 (每次新 session 都要设)
export http_proxy="http://80.253.48.42:3128"
export https_proxy="$http_proxy"

# 安装 docker-compose (v1.22.0, 21 个包 2.8MB)
dnf --setopt=proxy=$http_proxy install -y docker-compose

# 安装 dnsmasq (DNS preflight 需要)
dnf --setopt=proxy=$http_proxy install -y dnsmasq

# 验证
docker-compose --version    # docker-compose version 1.22.0
which dnsmasq               # /usr/bin/dnsmasq
```

**不重启 Docker。** dnf 使用 `--setopt=proxy` 绕过 DNS 空白问题。

---

### Step 2: 设置 bind mount (≈2 min)

```bash
# /data → /mnt/bss/cube-data (XFS reflink=1, 6.8T)
# 解决: / 分区 98% 满 + /data/cubelet 需要 XFS reflink
mkdir -p /mnt/bss/cube-data
mkdir -p /data
mount --bind /mnt/bss/cube-data /data

# /usr/local/services/cubetoolbox → /home/cubetoolbox (26T)
# 解决: CUBE_SANDBOX_INSTALL_ROOT 被 common.sh 硬编码为 readonly, 不可改
mkdir -p /home/cubetoolbox
mkdir -p /usr/local/services/cubetoolbox
mount --bind /home/cubetoolbox /usr/local/services/cubetoolbox

# 持久化到 fstab (重启后保持)
echo "/mnt/bss/cube-data /data none bind 0 0" >> /etc/fstab
echo "/home/cubetoolbox /usr/local/services/cubetoolbox none bind 0 0" >> /etc/fstab

# 验证
df -T /data          # Filesystem: /dev/nvme0n1p1, Type: xfs
df -T /usr/local/services/cubetoolbox  # Filesystem: /dev/mapper/..., Type: ext4
xfs_info /data | grep reflink          # reflink=1
```

**关键**: `CUBE_SANDBOX_INSTALL_ROOT` 在 `lib/common.sh:9-12` 被设为 `readonly`，无法通过 .env 修改，必须用 bind mount 绕过。/data 绑定到 /mnt/bss 后, /data/cubelet 自然在 XFS 上, 通过 preflight。

---

### Step 3: skopeo 预拉取镜像 (≈15-30 min)

> **为什么不配 Docker proxy?** Docker 18.09 的 proxy 配置在 systemd service 环境变量中, 修改后必须 `systemctl restart docker`, 会导致 10 个 RestartPolicy=no 的容器停止且不自动恢复。skopeo 用自己的进程代理, 直接写入 Docker image store, 完全绕过 daemon。

```bash
export http_proxy="http://80.253.48.42:3128"
export https_proxy="$http_proxy"

# 3a. 依赖容器镜像 (env.example 中明确指定, 从 cube-sandbox-image CR)
SKOPEO_IMAGES=(
  "cube-sandbox-image.tencentcloudcr.com/opensource/coredns/coredns:1.14.2"
  "cube-sandbox-image.tencentcloudcr.com/opensource/mysql:8.0"
  "cube-sandbox-image.tencentcloudcr.com/opensource/redis:7-alpine"
  "cube-sandbox-image.tencentcloudcr.com/opensource/openresty:1.21.4.1-6-alpine-fat"
)

for img in "${SKOPEO_IMAGES[@]}"; do
  echo ">>> skopeo copy: $img"
  HTTPS_PROXY=$http_proxy skopeo copy "docker://$img" "docker-daemon:$img" 2>&1 | tail -2
done

# 3b. CubeSandbox 组件镜像 (MIRROR=cn, 从 cube-sandbox-cn CR)
# 这些镜像的 tag 由 install.sh 从 release-manifest.json 读取
# 先拉 latest, install.sh 如果需要特定 tag 会报错, 届时补拉
CN_IMAGES=(
  "cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-api:latest"
  "cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-proxy:latest"
  "cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-lifecycle-manager:latest"
  "cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-egress:latest"
  "cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/minio:latest"
)

for img in "${CN_IMAGES[@]}"; do
  echo ">>> skopeo copy: $img"
  HTTPS_PROXY=$http_proxy skopeo copy "docker://$img" "docker-daemon:$img" 2>&1 | tail -2
done

# 3c. 模板镜像 (建模板用)
HTTPS_PROXY=$http_proxy skopeo copy \
  "docker://cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/sandbox-code:latest" \
  "docker-daemon:cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/sandbox-code:latest" 2>&1 | tail -2

# 验证
docker images | grep -E "cube-sandbox|coredns|mysql|redis|openresty"
```

**注意**: 部分组件镜像可能使用版本 tag 而非 latest。如果 install.sh 报错找不到特定 tag 的镜像, 用 skopeo 补拉该 tag:
```bash
HTTPS_PROXY=$http_proxy skopeo copy \
  "docker://cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-proxy:<tag>" \
  "docker-daemon:cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/cube-proxy:<tag>"
```

cube-api:latest 已实机验证可拉取 (23.1MB)。

---

### Step 4: 下载一键安装包 (≈5-10 min)

```bash
export http_proxy="http://80.253.48.42:3128"
export https_proxy="$http_proxy"

# 方式一: 在线安装器 (自动下载包并解析依赖)
curl -sL -x $http_proxy https://cnb.cool/CubeSandbox/CubeSandbox/-/git/raw/master/deploy/one-click/online-install.sh | MIRROR=cn bash

# 方式二: 手动下载 + 安装 (推荐, 版本钉得更死)
# 先获取最新版本
LATEST_JSON=$(curl -sL -x $http_proxy https://download.cubesandbox.com/release/latest.json)
echo "$LATEST_JSON"  # 查看版本和下载 URL
# 下载包
PKG_URL=$(echo "$LATEST_JSON" | python3 -c "import sys,json; print(json.load(sys.stdin)['download_url'])" 2>/dev/null || true)
# 或直接 git clone 获取 install.sh
git -c http.proxy=$http_proxy -c https.proxy=$http_proxy clone --depth 1 \
  https://cnb.cool/CubeSandbox/CubeSandbox.git /tmp/cubesandbox-src
cd /tmp/cubesandbox-src/deploy/one-click
```

---

### Step 5: 配置 .env (≈2 min)

```bash
cd /tmp/cubesandbox-src/deploy/one-click  # 或解压后的一键包目录
cp env.example .env
```

**必须修改的项**:

```bash
# === 端口冲突修改 ===
# 3000 被 multica-next.service 占用 → 改 3001
CUBE_API_BIND=0.0.0.0:3001
CUBE_API_HEALTH_ADDR=127.0.0.1:3001

# 8082 被 BSS_TM01_4U16G_1 容器占用 → 改 8083
CUBE_PROXY_ADMIN_PORT=8083

# === 网络配置 ===
CUBE_SANDBOX_NODE_IP=80.48.23.136
# 网卡名 (从 ip -o addr show 确认)
# CUBE_SANDBOX_NETWORK_CIDR 默认 192.168.0.0/18, 已确认无冲突

# === 镜像区域 ===
MIRROR=cn

# === DNS 模式 ===
# 136 有 NetworkManager 1.44.2, 用默认 networkmanager 模式
# 如 NM 插件不工作, 改 standalone
# CUBE_PROXY_DNSMASQ_MODE=standalone

# === BM 部署 (不需要 PVM) ===
CUBE_PVM_ENABLE=0

# === 关闭腾讯 Docker 镜像加速 (不重启 Docker) ===
ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR=0

# === 角色 ===
ONE_CLICK_DEPLOY_ROLE=control
ONE_CLICK_ASSUME_YES=1
```

**不需要改的项** (保持默认):
- `CUBE_SANDBOX_INSTALL_ROOT` — 硬编码 readonly, 用 bind mount 解决
- `CUBE_SANDBOX_MYSQL_PORT=3306` — 未被占用
- `CUBE_SANDBOX_REDIS_PORT=6379` — 未被占用
- `CUBE_PROXY_HTTP_PORT=80` — 未被占用
- `CUBE_PROXY_HTTPS_PORT=443` — 未被占用
- `WEB_UI_HOST_PORT=12088` — 未被占用
- `CUBEMASTER_ADDR=127.0.0.1:8089` — 未被占用
- `CUBE_OPS_BIND=0.0.0.0:3010` — 未被占用

---

### Step 6: 运行 install.sh (≈10-20 min)

```bash
cd /tmp/cubesandbox-src/deploy/one-click  # 或一键包目录
export http_proxy="http://80.253.48.42:3128"
export https_proxy="$http_proxy"

# 运行安装 (不需要 sudo, 已是 root)
./install.sh --mode=install 2>&1 | tee /var/log/cube-install.log
```

**install.sh 行为预告** (源码确认):
1. 解压 sandbox-package.tar.gz 到 WORK_DIR (/tmp, 378G 充足)
2. preflight 检查: BPF fs, cgroup, DNS, XFS, 代理证书
3. `install_docker()` — 检测到 docker 已存在 → `return 0` (不升级不重启)
4. `install_docker_compose()` — 检测到 docker-compose 已安装 → `return 0`
5. `configure_tencent_docker_mirror()` — `ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR=0` → 跳过 (不重启 Docker)
6. 安装 systemd 单元, 启动容器 (MySQL/Redis/MinIO/WebUI/CoreDNS/cube-proxy)
7. 容器使用本地已有镜像 (skopeo 预拉取), 不需要 docker pull
8. quickcheck 等待健康检查通过

**可能失败的点**:
- 组件镜像 tag 不匹配 → 看 install.log 找到需要的 tag, 用 skopeo 补拉, 重跑 install.sh
- DNS preflight 失败 → 改 `CUBE_PROXY_DNSMASQ_MODE=standalone` 重试
- /data/cubelet XFS 检查 → Step 2 已解决

---

### Step 7: 验证安装 (≈2 min)

```bash
# systemd 服务状态
systemctl list-units 'cube-sandbox-*' --no-pager
# 期望: cube-sandbox-cubelet active, cube-sandbox-dns active/exited, 等

# 端口监听
ss -tlnp | grep -E ':3001 |:8089 |:3010 |:9998 |:8083 |:12088 '
# 3001 = cube-api, 8089 = cubemaster, 3010 = cubeops, 8083 = cube-proxy-admin, 12088 = webui

# 健康检查
curl -s http://127.0.0.1:3001/health || curl -s http://127.0.0.1:3010/health

# 容器状态
docker ps | grep cube-sandbox
# 期望: cube-sandbox-mysql, cube-sandbox-redis, cube-sandbox-minio, cube-webui, cube-proxy, cube-proxy-coredns

# 现有容器未受影响
docker ps | grep -v cube-sandbox | wc -l   # 应为 15
```

**出问题排查**:
```bash
journalctl -u cube-sandbox-cubelet -e --no-pager | tail -30
journalctl -u cube-sandbox-dns -e --no-pager | tail -30
cat /var/log/cube-sandbox-one-click/*.log | tail -50
# 整栈回退: ./down.sh
```

---

### Step 8: 建模板 (≈5-10 min)

```bash
# 用官方预置镜像 (multi-arch 含 amd64)
cubemastercli --address 127.0.0.1 --port 8089 tpl create-from-image \
  --image cube-sandbox-cn.tencentcloudcr.com/cube-sandbox/sandbox-code:latest \
  --writable-layer-size 1G --expose-port 49999 --expose-port 49983 --probe 49999

# 等待模板 READY (镜像已在本地, 不需要 pull)
JOB_ID=<上面输出的 job_id>
cubemastercli --address 127.0.0.1 --port 8089 tpl watch --job-id $JOB_ID

# 查看模板列表, 记下 template_id
cubemastercli --address 127.0.0.1 --port 8089 tpl list
```

**模板红线**:
1. 必须带 `--probe 49999`, 否则死容器也会 READY
2. READY 后先做 n=1 冒烟 (Step 9), 再跑百并发

---

### Step 9: 冒烟测试 (≈2 min)

```bash
# n=1 单沙箱冒烟
E2B_API_URL=http://127.0.0.1:3001 E2B_API_KEY=e2b_000000 \
CUBE_TEMPLATE_ID=<模板ID> ./cube-bench \
  -mode create-only -n 1 -c 1 \
  -api http://127.0.0.1:3001 -key e2b_000000 \
  -tpl <模板ID> -client-timeout 600s

# 期望: 全 2xx, wall <1s
# 然后手动 pause → resume 一轮验证
```

---

### Step 10: 安装 Go + 构建 cube-bench + 跑基线 (≈30-40 min)

```bash
# 10a. 安装 Go (build_bench.sh 需要 go1.25.0+)
export http_proxy="http://80.253.48.42:3128"
export https_proxy="$http_proxy"
curl -L -x $http_proxy -o /tmp/go.tar.gz https://go.dev/dl/go1.25.0.linux-amd64.tar.gz
tar -C /usr/local -xzf /tmp/go.tar.gz
export PATH=$PATH:/usr/local/go/bin
go version    # go version go1.25.0 linux/amd64

# 10b. 构建 cube-bench
# 上传 skill package 的 assets/cube-bench-src/ 和 build_bench.sh 到 136
cd /root/.bench  # 或任意工作目录
bash build_bench.sh    # 原生构建 amd64 cube-bench

# 10c. env_check 立基线
bash env_check_x86.sh --collect /root/env_x86.postinstall

# 10d. 百并发基线
CUBE_TEMPLATE_ID=<模板ID> bash oc_round.sh x86V_$(date +%m%d) 100 25 "25" on
# 记录 create/pause/resume 的 wall / p50 / p95 / 成功率
```

---

## 2. 时间预估

| 步骤 | 内容 | 预估时间 | 瓶颈 |
|------|------|----------|------|
| Step 1 | 安装 docker-compose + dnsmasq | 3 min | dnf 下载 2.8MB + 253KB |
| Step 2 | bind mount /data + INSTALL_ROOT | 2 min | 无 |
| Step 3 | skopeo 预拉取 10 个镜像 | 15-30 min | 代理速度 + 镜像大小 |
| Step 4 | 下载一键安装包 | 5-10 min | 包大小 + 代理速度 |
| Step 5 | 配置 .env | 2 min | 人工编辑 |
| Step 6 | 运行 install.sh | 10-20 min | 解压包 + 启动容器 + quickcheck |
| Step 7 | 验证安装 | 2 min | 无 |
| Step 8 | 建模板 | 5-10 min | 镜像解包 + 快照 |
| Step 9 | 冒烟测试 | 2 min | 单沙箱创建 |
| Step 10a-b | 安装 Go + 构建 cube-bench | 10 min | Go 下载 + 编译 |
| Step 10c-d | 百并发基线 | 15-30 min | 100 沙箱 create/pause/resume |
| **总计** | | **70-120 min** | **≈1-2 小时** |

主要不确定性:
1. **镜像拉取** (Step 3): 代理速度不稳定, 大镜像可能慢; 组件镜像 tag 可能需要试错
2. **install.sh** (Step 6): 首次安装可能有未知 preflight 失败, 需要排查重跑
3. **建模板** (Step 8): sandbox-code 镜像较大, 解包建快照需要时间
4. **百并发基线** (Step 10d): 首轮是立基线, 不是复测, 如果有问题需要排查

---

## 3. 风险矩阵 (全部已验证)

| # | 风险 | 等级 | 解决方案 | 验证状态 |
|---|------|------|----------|----------|
| 1 | docker-compose 缺失 | 🔴→🟢 | `dnf --setopt=proxy install docker-compose` | ✅ dry run 成功 |
| 2 | Docker Hub 不可达 | 🟡→🟢 | skopeo 从腾讯 CR 拉取, 不走 Docker Hub | ✅ 实机验证 |
| 3 | / 分区 98% 满 | 🟡→🟢 | bind mount /data→/mnt/bss, INSTALL_ROOT→/home | ✅ 空间确认 |
| 4 | /data/cubelet 需 XFS reflink | 🟡→🟢 | /data 绑定到 /mnt/bss (XFS reflink=1) | ✅ xfs_info 确认 |
| 5 | 端口 3000 冲突 | 🟡→🟢 | .env 改 CUBE_API_BIND=0.0.0.0:3001 | ✅ ss 确认占用 |
| 6 | 端口 8082 冲突 | 🟡→🟢 | .env 改 CUBE_PROXY_ADMIN_PORT=8083 | ✅ ss 确认占用 |
| 7 | dnsmasq 未安装 | 🟡→🟢 | dnf --setopt=proxy install dnsmasq | ✅ dry run 成功 |
| 8 | Go 未安装 | 🟡→🟢 | Step 10a 安装 Go 1.25.0 | 非安装阶段 |
| 9 | DNS resolv.conf 为空 | 🟢 | 不影响现有容器 (DNS 独立) | ✅ 15 容器已验证 |
| 10 | install.sh 重启 Docker | 🟢 | install_docker() 检测到已存在就 return 0 | ✅ 源码确认 |
| 11 | TPROXY 影响他人 | 🟢 | 仅匹配 cube-dev 接口 + cube fwmark | ✅ 源码确认 |
| 12 | cgroup v1 不兼容 | 🟢 | install.sh 接受 v1 (源码注释确认) | ✅ 源码确认 |
| 13 | 192.168.0.0/18 冲突 | 🟢 | 无现有路由/地址 | ✅ ip route 确认 |
| 14 | eBPF 不支持 | 🟢 | 内核 6.6.0 BPF 全开 | ✅ config 确认 |
| 15 | 腾讯 CR 不可达 | 🟢 | HTTP 200 via proxy | ✅ curl 确认 |

---

## 4. 回退方案

```bash
# 整栈回退 (停止 CubeSandbox 容器 + systemd 服务)
cd <一键包目录>
./down.sh

# 卸载 bind mount
umount /data
umount /usr/local/services/cubetoolbox
# 从 /etc/fstab 删除对应行

# 卸载 docker-compose (可选)
dnf remove -y docker-compose

# 现有 15 个容器不受任何影响, 无需恢复
```

---

## 5. 一句话执行清单

```
1. dnf install docker-compose + dnsmasq (3 min)
2. bind mount /data→/mnt/bss + INSTALL_ROOT→/home (2 min)
3. skopeo 批量拉取 10 个镜像 (15-30 min)
4. 下载一键安装包 (5-10 min)
5. 配 .env: 端口 3001/8083 + NODE_IP + MIRROR=cn (2 min)
6. ./install.sh --mode=install (10-20 min)
7. 验证: systemctl + ss + curl + docker ps (2 min)
8. 建模板: cubemastercli tpl create-from-image (5-10 min)
9. 冒烟: cube-bench -n 1 (2 min)
10. 装 Go + build_bench.sh + oc_round.sh 100 并发 (25-40 min)
总计: ≈1-2 小时
```
