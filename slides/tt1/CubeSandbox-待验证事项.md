# CubeSandbox 待验证事项

本文件记录《CubeSandbox 源码构建与双平台对比测试手册》中不确定、需要在实机上确认的事项。手册正文只写按最大可能性给出的结论，验证状态集中在这里。

依据等级：✅ 源码确认 ｜ 📄 官方文档 ｜ 🔬 上游 issue/PR ｜ ⚠️ 推断，未验证

优先级：**P0** 不确认则测试无法进行或数据无效 ｜ **P1** 影响结果解释 ｜ **P2** 影响效率

目标版本 **v0.7.1**。

---

## A0. 规格完整性（P0，必须先解决）

`checking_points.xlsx` 是这次测试的先决条件：它规定目标机器必须是什么样，操作者据此去找或配置机器。所以规格本身必须先完整、无矛盾，否则根本判断不出该用哪台机器。

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| A0-1 | 表中仍有 4 处标「待确认」：内存规格（E5）、内核版本（E15）、容器操作系统（E18）、核数（E20） | ⚠️ 表格现状 | 目标机器无法确定，选不出符合规格的机器 | 与规格作者确认后补全，再开始第 3 篇 | 不能开始测试，否则只能退化成「有什么机器测什么机器」，违背规格作为输入的前提 |
| A0-2 | 规格自身存在矛盾：x86 列的机型与内存速率（见 D6、D5） | ⚠️ 表格现状 | 规格互相冲突，无法判断该找哪台机器 | 先修正规格再开始 | 同上 |

## A. 与 v0.7.1 直接相关的风险（P0）

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| A1 | **aarch64 上 pause 后 resume 是否失败** | 🔬 v0.7.2 的 changelog 记录修复了「resume failing after pause on ARM（`Could not restore GICv3ITS state`）：vgic 状态保存被编译器错误优化成全零」。目标版本 v0.7.1 **不包含**该修复 | 鲲鹏侧的 Pause/Resume 测试项可能整体不可用 | 部署后执行 `python3 bench_pause_resume_concurrency.py -c 1 -n 1`，观察是否报 `Could not restore GICv3ITS state` | 鲲鹏侧 4.4 无法产出数据，只能记录为「该版本不支持」，或改用 v0.7.2 |
| A2 | **aarch64 模板构建顺序造成的延迟漂移** | 🔬 issue #1803 明确影响 v0.5.1 **与 v0.7.1**：同一镜像与配置，物理机重启后创建的第一个模板最快，之后劣化 1.2～1.7 倍，吞吐降至 0.64 倍，且在 x86_64 上不可复现 | 若两边模板构建次数不同，测出的是该缺陷而非平台差异 | 在鲲鹏侧连续创建 3 个模板，分别测 c=1 创建延迟，比较三者差异；若第 2、3 个明显更慢即命中 | 必须严格执行「每轮测量前冷重启 + 重建模板」，并在报告中记录模板构建顺序 |
| A3 | **cube-bench 在 aarch64 上的吞吐是否正常** | 🔬 issue #1611（v0.6.0）：96 核 aarch64 上「调整各种参数及配置，测出的时延各不相同但吞吐量完全一致」。该 issue 已关闭，但是否在 v0.7.1 完全修复未确认 | 吞吐指标可能是假的（恒定值），导致并发扩展结论错误 | 用 `-c 1` 与 `-c 100` 各跑一次 `-n 100`，比较 `throughput_qps`；若两者相同即异常 | 吞吐相关结论作废，需只报告延迟分位数 |

## B. 构建阶段（第 3 篇 3.1～3.3）

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| B1 | 腾讯 CR 或其它可达镜像源是否存在 **`ubuntu:20.04`** | ⚠️ 手册按「用 skopeo 从可达源拉取后打本地 tag」给出，但未确认任何具体镜像源里有该镜像 | 3.1.3 无法完成，`make builder-image` 失败 | `skopeo inspect docker://<mirror>/ubuntu:20.04` | 需改用其它 Ubuntu 20.04 镜像源，或改 `docker/Dockerfile.builder` 第一行（偏离上游，需注明） |
| B2 | 是否存在 **`tencentos/tencentos4-minimal`** 与 **`docker/compose:1.29.2`** 的可达副本 | ⚠️ 同上，两者都在 Docker Hub | guest 镜像构建失败；或安装阶段 compose 容器起不来 | `skopeo inspect docker://<mirror>/docker/compose:1.29.2` | 需为这两个镜像另找来源 |
| B3 | `deploy/release-assets.yaml` 在 **v0.7.1** tag 下钉死的 kernel / guest-image tag 具体值 | ⚠️ 手册用命令从该文件读取，未写死值（master 上是 `kernel-release-260921-1` 与 `guest-image-260820-1`，v0.7.1 可能不同） | 内核与 guest 镜像版本可能与 manifest 不符 | clone 后 `cat deploy/release-assets.yaml` | 需按实际值调整 3.2.1 |
| B4 | 各发行版上 `docker` / `skopeo` / `pigz` / `e2fsprogs` 的**包名**是否如手册所写 | ⚠️ 手册按常见命名给出，未在 openEuler 24.03 实测 | 3.1.2 安装失败 | 直接执行 3.1.2 命令，看是否有 `No match for argument` | 逐个查 `dnf search` 改包名 |
| B5 | 宿主机 **npm 版本**是否满足 WebUI 构建 | ⚠️ WebUI 在宿主机构建（builder 镜像内无 Node），但未确认最低 npm/Node 版本要求 | 构建在最后阶段失败 | `npm --version`，或先用 `ONE_CLICK_WEB_DIST_DIR` 复用已构建 dist | 需升级 Node/npm，或提供预构建 dist |
| B6 | **builder 容器内的下载是否走代理** | ⚠️ 宿主机设置了代理，但容器内是否需要额外传参未确认（Dockerfile 中 apt / rustup / go / crates / npm 均需出网） | 构建在装工具链阶段失败 | 观察 `/tmp/cube-build-*.log` 中是否出现连接超时 | 需为 builder 容器显式传 `--build-arg` 或配置 daemon 代理 |
| B7 | 构建机**磁盘与内存**是否足够 | ⚠️ 手册给出 ≥100 GB 与 ≥32 GB 的经验值，未实测 | Rust fat-LTO 链接阶段 OOM 或 ENOSPC | 构建前 `df -h`、`free -g`；必要时降 `ONE_CLICK_BUILD_JOBS` | 构建失败，需换大分区或降低并行度 |

## C. 安装阶段（第 3 篇 3.4～3.5）

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| C1 | **bind mount 绕过只读安装根**是否可行 | ✅ 源码确认 `CUBE_SANDBOX_INSTALL_ROOT` 被强制覆盖并 `readonly`（`lib/common.sh:9-12`）；⚠️ 但手册给出的 bind mount 解法**上游未支持也未测试** | 3.4.2 是手册的关键变通，若无效则安装根无法改位置 | 3.4.2 执行后 `df -T /usr/local/services/cubetoolbox` 确认指向目标分区，再跑 3.4.4 | 若安装脚本检测到异常或写入失败，需要另找方案（如把根分区扩容） |
| C2 | 目标机**端口占用**实际情况 | ⚠️ 手册示例假设 3000 与 8082 被占用 | 端口冲突会让 unit 启动失败 | 3.4.1 的 `ss -lntp` 循环 | 按实际占用改 `.env` |
| C3 | **DNS preflight** 是否通过 | ✅ 源码确认需要 `resolvectl`，或 NetworkManager 已加载，或 `CUBE_PROXY_DNSMASQ_MODE=standalone`；⚠️ 目标机实际具备哪一个未确认 | 安装中断 | 3.4.1 中检查 `systemctl show -p LoadState --value NetworkManager` 与 `which resolvectl` | 需要装 dnsmasq 或改 standalone 模式 |
| C4 | 目标机 **cgroup 版本**及运行时 `cpuset` 控制器是否可用 | ✅ 源码确认安装脚本在 cgroup v2 下只检查 `cpu` 控制器；运行时还需要 `memory` 与 `cpuset`（诊断脚本 `scripts/cube-diag/check-deps.sh` 会检查） | cgroup v1 下能否安装、以及能否做 CPU 绑定未确认 | 3.4.1 中 `stat -fc %T /sys/fs/cgroup` 与 `cat /sys/fs/cgroup/cgroup.controllers` | v1 环境可能缺 `cpuset`，影响任何 CPU 绑定的压测 |
| C5 | **Docker 版本**能否运行 `docker/compose:1.29.2` 容器 | ⚠️ 手册要求保持既有 Docker 不重启；若目标机 Docker 版本较旧（如 18.09），能否正常跑该 compose 容器未确认 | 安装阶段各 compose 包装脚本失败 | 3.4.3 完成后 `docker run --rm docker/compose:1.29.2 version` | 需升级 Docker（但共享服务器上不可接受），或改用其它 compose 方式 |
| C6 | 目标机 `/data` 与安装根**容量是否足够** | ⚠️ 未实测 | 安装中途 ENOSPC | 3.4.1 的 `df -hT` | 需换分区或清理 |
| C7 | `install.sh` 能否一次通过全部 preflight | ⚠️ 首次安装可能有未知 preflight 失败 | 安装中断 | 3.4.4 后查看日志末尾 | 按日志提示逐项处理后重跑 |

## D. 公平性基线（第 2 篇）

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| D1 | **鲲鹏 950 的具体 SKU 必须写进规格** | 🔬 官方发布会称 950 有两个型号：**96 核 /192 线程** 与 **192 核 /384 线程**，核心数相差一倍。规格中只写「鲲鹏 950」不足以确定要找哪台机器 | 核数直接决定并发能力，规格不明确就选不出机器 | 与规格作者确认 SKU 并补进表格；机器到手后用 `lscpu` 与 `dmidecode -t processor` 核对是否相符 | 核数若与 x86 侧不对等，比值失去意义 |
| D2 | 鲲鹏侧 **NUMA 相关 BIOS 项的实际名称** | ⚠️ 手册用 AMD 术语 NPS 描述 x86 侧；鲲鹏侧的对应物是 SCCL 粒度、Node Interleaving、SNC、UFS，具体 BIOS 项名未确认 | 无法确认两边 NUMA 配置是否等价 | 进 BIOS 逐项记录，或 `numactl -H` 看节点数，`dmidecode -t bios` 看版本 | 只能退而记录 `numactl -H` 的实际拓扑，并在报告中说明无法确认 BIOS 项 |
| D3 | **大页参数在 arm64 上是否生效** | ⚠️ 手册与现有表格使用 `default_hugepagesz=2M hugepagesz=2M hugepages=65535`。arm64 的巨页尺寸随基础页大小变化，若目标机是 **64 KB 基础页**，2 M 巨页可能不成立 | 两边大页配置实际不等价，内存类指标不可比 | `getconf PAGESIZE`、`grep -i huge /proc/meminfo`、`cat /proc/cmdline` | 需按实际基础页调整巨页参数，或改为两边统一使用显式巨页池 |
| D4 | 鲲鹏侧 **超线程是否真的开启** | 🔬 鲲鹏 930/950 支持 SMT2，但 916/920 不支持；华为云文档称**绝大多数鲲鹏规格默认关闭**该特性。规格要求「开」 | 线程数可能相差一倍，直接改变并发甜点 | 机器到手后 `cat /sys/devices/system/cpu/smt/active`，应为 1；不是则在 BIOS 打开并冷重启 | 与规格不符，数据不可比 |
| D5 | **内存速率规格与候选机器的实际速率** | ⚠️ 规格要求 24×64G DDR5 6400MT/s；EPYC 9654（Genoa）官方支持 DDR5-4800，6400 更接近 9005 系列（Turin）。需确认确实存在符合该速率的机器 | 找不到符合规格的机器时，只能修改规格或换机型 —— 那属于改规格，不是把实际值记录下来 | 候选机器到手后 `dmidecode -t memory` 看 `Configured Memory Speed`，必须达到规格要求；建议再用 STREAM 实测带宽 | 用不符合规格的机器测，测的不是规格要求的环境，数据不能代表规格 |
| D6 | **规格中 x86 列的机型到底是 9654 还是 9755** | ⚠️ 该列标题写 `9654(Zen4)`，但其数据（TDP/PPT=500、NPS8、512 线程、DDR5-6400）全部指向 **9755**；而 `X86-136-DEPLOY.md` 描述实机为「EPYC 9654 96核 ×2 = 384核」 | 规格自身矛盾，判断不出该找哪台机器 | 先与规格作者确认并修正表格，再开始 | 照矛盾规格选机器，x86 侧基准对象就是错的 |
| D7 | 两边 **balloon free-page 上报**的变量名与默认值 | 🔬 有 PR 将 aarch64 的该上报默认关闭、保留 x86_64 默认；变量名与 v0.7.1 上的实际默认值未确认 | 内存开销与密度数据不可比 | `grep -ri balloon` 查 cubelet 配置与源码；两边显式设成同值 | 密度类指标作废 |
| D8 | 两边 **guest 内核 config 的具体差异** | ✅ 源码确认两平台是不同 config 文件（`configs/kernel-oc9.x86_64.config` 与 `.aarch64.config`） | 影响哪些特性在 guest 内可用 | 部署后 `diff` 两份 config，记录 guest 内核 sha256 | 报告中不能声称「guest 内核相同」 |
| D9 | **Power Profile 与 Hardware Prefetcher** 的实际设置 | ⚠️ 这两项**无法从操作系统读取**，只能从 BIOS/Redfish 获取 | 频率策略不一致会让所有延迟数据带系统偏差 | 导出 Redfish BIOS Attributes JSON 或拍 BIOS 截图 | 若拿不到，必须在报告中声明该项未受控 |

## E. 测试阶段（第 4 篇）

| # | 事项 | 不确定性来源 | 影响 | 验证方法 | 不成立的后果 |
|---|---|---|---|---|---|
| E1 | 模板镜像按架构解析后的 **digest** | ⚠️ 官方称只有 `sandbox-code:latest` 是 Multi-Arch，但具体 digest 需实测 | 无法证明两边 rootfs 相同 | `docker manifest inspect <image>` 记录两个架构的 digest | 若某架构无对应 manifest，该平台无法运行该镜像 |
| E2 | 沙箱规格 **2 vCPU / 2 GiB** 是否在模板中真正生效 | ⚠️ 模板创建参数中未显式给出 vCPU 与内存 | 两边规格若不同，延迟不可比 | 创建沙箱后在 guest 内 `nproc`、`free -h` | 需要调整模板参数 |
| E3 | **CubeVS eBPF 是否真的挂载** | ⚠️ 若沉默失败，网络会走退化路径 | 网络相关指标失真 | 两边执行 `bpftool prog show` 与 `tc filter show` | 需排查 clang 版本与 per-arch `vmlinux.h` |
| E4 | guest 内 **PMU 是否可用** | 📄 官方文档称部分 aarch64 主机不向 guest 暴露 PMUv3 | 不能用 guest 内硬件计数器做对比 | guest 内 `perf stat` 试跑 | 该类指标只能从宿主机侧测量 |
| E5 | **tap 池数量**是否足够 | ⚠️ 默认 `tap_init_num=500`；本轮最高 100 并发，理论上够用 | 高并发时创建失败 | 4.3 若出现创建失败，查 cubelet 配置中的 `tap_init_num` | 需调大并重启 cubelet |
| E6 | `collect_baseline.py` 在 **aarch64** 上是否正常 | ⚠️ 仅在 x86_64 上验证过（exit 0、JSON 合法、缺命令不崩） | 鲲鹏侧证据采集可能缺字段 | 在鲲鹏侧执行 `python3 collect_baseline.py --out baseline-aarch64.json`，检查 `_errors` | 按 `_errors` 补装工具 |

---

## F. 汇总：优先处理顺序

1. **A0**（规格完整性）—— 规格是测试的输入，它不完整或有矛盾，就选不出符合规格的机器，后面全部无法开始。先补全 E5/E15/E18/E20 四处「待确认」，并解决 D6、D5 的矛盾。
2. **A1**（v0.7.1 的 ARM pause/resume）—— 决定 4.4 在鲲鹏侧是否可做，建议先做单项验证。
3. **A2**（ARM 模板顺序漂移）—— 决定整个鲲鹏侧数据是否可信，必须落实冷重启纪律。
4. **B1 / B2**（镜像源可达性）—— 不确认则构建与安装都走不下去。
5. **A3、C4、D3、D4、E3** —— 影响具体指标的可用性与可比性。
