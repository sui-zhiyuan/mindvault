# CubeSandbox 测试方案 —— 交接与讨论结果

> **用途**：在另一台设备上恢复本次工作的上下文。读完本文档 + [`CubeSandbox-配置名称清单.md`](CubeSandbox-配置名称清单.md) 即可继续，**不需要原始会话记录**。
> **更新**：2026-09-29

---

## 1. 任务目标

分析 `slides/tt1/` 下的三份原始材料（`X86-136-DEPLOY.md`、`checking_points.xlsx`、`bios_config.docx`），产出：

1. 一份新的 markdown 文档：把预编译一键包安装改为**从源码构建安装**，按「篇→章」单一层级组织，明确区分「部署」与「测试」，每个步骤统一给出「一段文字 + 一段命令 + 一个验证」。
2. 输出 `checking_points.xlsx` 中涉及测试结果的关键信息需要修改的内容清单（**不直接修改该 Excel**）。
3. 一个 Python 脚本，用于收集「公平性基线」一节要求的各项环境与硬件证据。

目标读者设定：**对 CubeSandbox 完全无知的人也能照着完成操作，并理解每一步在干什么、产出给谁用。**

适用平台：**x86_64（AMD EPYC 9654/9755 级）** 与 **aarch64（鲲鹏 950）**，两者用完全一致的方法测出可互相对比的数据。

---

## 2. 交付物现状

| 文件 | 内容 | 状态 |
|---|---|---|
| [`CubeSandbox-源码构建与双平台对比测试手册.md`](CubeSandbox-源码构建与双平台对比测试手册.md) | 主文档，5 篇（开始之前 / 公平性基线 / 部署 / 测试 / 附录） | 已按多轮评审改定，约 810 行 |
| [`CubeSandbox-待验证事项.md`](CubeSandbox-待验证事项.md) | 33 项待验证事项，分 A0/A/B/C/D/E 组 + 优先顺序 | 已建 |
| [`collect_baseline.py`](collect_baseline.py) | 公平性基线采集脚本（只读、零依赖） | 已建，**仅在 x86_64 实测过**（exit 0、JSON 合法、缺命令不崩） |
| [`CubeSandbox-配置名称清单.md`](CubeSandbox-配置名称清单.md) | 测试规格清单的配置名称建议（95 项） | **待用户定稿** |
| [`X86-136-DEPLOY.md`](X86-136-DEPLOY.md) | 原始材料：预编译包单机速查手册 | 只读，不改 |
| `checking_points.xlsx` · `bios_config.docx` | 原始材料 | 只读，不改 |

提交历史（分支 `feat/cube-src-test-doc`）：

```
d6fda69  docs(tt1): name the spec list by role and only ever read from it
e8054f3  docs(tt1): treat the baseline sheet as the spec, not a record to fill in
50b468c  docs(tt1): drop the verification step where the command output already answers
6a02091  docs(tt1): apply review feedback to the manual
af07d62  docs(tt1): move unsettled claims into a separate verification file
f9091f7  docs(tt1): restructure the manual around prose, one command and one check
3d73ff0  chore(tt1): record the baseline xlsx as re-saved by Excel
d9aa571  feat(tt1): add fairness baseline collector script
d076810  docs(tt1): add source-build dual-platform test manual
```

---

## 3. 用户已确认的要求（硬约束）

这些是多轮评审中逐条提出的要求。**改动文档时必须继续满足**，不要回退。

### 3.1 文档形态

- **单一层级**：篇 → 章（如 `3.1.1`）。不要「第 N 章」与「阶段 A/B/C」两套并行编号。
- **明确区分部署与测试**。测试部分单独列出，不要标注「同一台机器可重复多次」这类说明。
- **每个步骤**统一为：**一段通顺的文字描述** + **一段执行命令** + **一个验证命令与输出描述**。做什么 / 为什么 / 产出 / 产出给谁用，要融进那一段通顺的文字里，不要拆成字段。
- **不写「前置条件」字段**。读者按顺序执行，重复上一步的产出没有意义。
- **元叙述一律不写进文档**：不要「本篇的目的是……」「即使你从未听说过 CubeSandbox……」「本手册的步骤写法是……」这类自述。**要求要通过文档本身实现，不要写成说明**。也不要把读者不需要知道的具体要求写进去。
- **命令输出本身就能判断成功的步骤，不要再写一个验证小节**（例：安装前检查，命令输出就是要查的事实）。**更不能把同一条命令跑两遍**（曾出现 `cube-bench` 冒烟命令在正文与验证里各跑一次；pause/resume 与快照脚本也重复执行过）。
- **详略按可推断性区分**：能从标题推断作用的步骤一句话即可（例：「从 GitHub 拉取 v0.7.1 的源码。」）；无法推断的步骤要说明产出给谁用（例：预拉运行时镜像，必须说明每个镜像在哪里被谁使用）。

### 3.2 内容取向

- 假设读者对 CubeSandbox 完全无知，也要能完成操作。
- 每个步骤让读者知道「这一步在干什么、它的产出给谁用」，避免很久之后才知道某步输出的用途。
- **主文档不含任何验证状态信息**：不要 `⚠️`/`✅`/`📄`/`🔬`，不要「待验证」「未验证」字样。不确定处**按最大可能性直接写**。
- 需要验证的问题**单独成文**，即 `CubeSandbox-待验证事项.md`。
- **不要出现交叉编译 / 交叉构建的内容**（本项目只支持同架构原生构建）。

### 3.3 测试规格清单（原 `checking_points.xlsx`）

- 它是**整个测试的输入 / 规格书**，不是记录表。操作者的动作是：**先读它 → 按规格去找或配一台符合规格的机器 → 按规格执行**。不是「有什么机器测什么机器，再把实际值写回去」。
  - 例：从它读出「测试版本」，再据此决定 `git clone` 的版本。
  - 例：规格要求内存 6400MT/s，就去找一台 6400MT/s 的机器来测。
- 文档中一律称它 **「测试规格清单」**，**不要出现文件名** `checking_points.xlsx`。
- 所有提及必须是**读取性质**：从…获取 / 从…查询 / 将输出与…比较 / 按…设置。
- **不要写「禁止修改」「不得写入」这类话** —— 要求靠措辞体现，不要写成声明。

### 3.4 测试参数（用户拍板）

| 项 | 值 |
|---|---|
| 目标版本 | **v0.7.1** |
| create 并发 | **100** |
| pause / resume 并发 | **10** |
| 热身 | **`-w 3`**（结果丢弃） |
| 正式轮次 | **3 轮取平均**（并保留轮间极差） |
| 沙箱规格 | 2 vCPU / 2 GiB |

### 3.5 镜像拉取

- **所有镜像统一在一步内拉完**（builder 基座 + guest 基座 + 全部运行时镜像），避免多次走慢网络。
- **不要 `--override-arch`**（同架构构建，不需要）。

---

## 4. 已核实的关键事实（不必重新调研）

### 4.1 上游源码

本地 clone：`.dsh.local/scratch/CubeSandbox`，commit `80614ab`（master，2026-09-28），`.gitmodules` 为空，无需 `--recursive`。

| 事实 | 依据 |
|---|---|
| `CUBE_SANDBOX_INSTALL_ROOT` 被强制为 `/usr/local/services/cubetoolbox` 且 `readonly`（写进 `.env` 会被静默忽略，上游无官方绕过方式） | `deploy/one-click/lib/common.sh:9-12` |
| `/data/cubelet` 路径写死，且所在文件系统必须是 **XFS** | `deploy/one-click/install.sh:1042-1078` |
| builder 镜像基座 `FROM ubuntu:20.04`（**Docker Hub**） | `docker/Dockerfile.builder:3` |
| guest 镜像基座 `tencentos/tencentos4-minimal`（**Docker Hub**） | `deploy/guest-image/Dockerfile:1` |
| **WebUI 在宿主机用 npm 构建**，builder 镜像内没有 Node | `deploy/one-click/build-release-bundle.sh:621,665-667` |
| **install.sh 不检查端口占用**，只在 unit 启动时报错 | `scripts/one-click/up-cube-proxy.sh:260-270` |
| install.sh 校验 manifest 结构，要求含 `components`/`guest_image`/`kernel` 三键 | `deploy/one-click/lib/common.sh:872-901` |
| 拒绝未知版本：`unknown forbidden` | `deploy/one-click/install.sh:1336` |
| 两平台内核 config 分开：`configs/kernel-oc9.{x86_64,aarch64}.config` | `configs/` |
| 内核/镜像 tag 钉死在 `deploy/release-assets.yaml`（master 上为 `kernel-release-260921-1`、`guest-image-260820-1`；**v0.7.1 下需重新读取**） | 同左 |
| `.env` 中**没有任何大页 / NUMA / cpuset 变量**，这些全是宿主机层设置 | 全文检索确认 |
| `ONE_CLICK_ENABLE_TENCENT_DOCKER_MIRROR=1` 会改写 `daemon.json` 并**重启 Docker** | `deploy/one-click/install.sh:1370-1417` |
| `ONE_CLICK_ENABLE_S3LVOL=1` 会创建约 **512 GiB** 的 WAL 镜像 | `deploy/one-click/install.sh:1997-2009` |

### 4.2 测量工具能力

- **`cube-bench`（`examples/cube-bench/`）**
  - 只有 `create-delete` 与 `create-only` 两种模式；**没有任何 pause/resume 能力**（全目录检索确认）。
  - 指标：`min avg std P50 P90 P95 P99 max` + `success_rate` + `throughput_qps`，可 `-o` 导出 JSON。
  - **延迟计时到「收到响应头」为止**，不含读 body 与解析；**没有 TTFB 指标**。
  - `-c` 是**在途请求上限**（不是 worker 数）；`-n` 是总迭代数。
  - `-w` 热身**严格串行且完全不计入统计**；`--dry-run` 会忽略它。
  - 要求 **Go 1.25.0**（README 写 1.21 是过期的）。
  - 任一迭代出错则退出码为 1。
- **pause / resume 必须用官方 Python 脚本**：`examples/snapshot-rollback-clone/bench_pause_resume_concurrency.py`
  - 输出 12 列：并发、轮数、`pause_avg/min/p95/max`、`per_pause`、`resume_avg/min/p95/max`、`per_resume`。
  - 脚本内部先跑 1 轮热身并丢弃，再跑 `-n` 轮正式测量，轮间 `-s`（默认 2 s）。
  - 依赖 `cubesandbox>=0.2.0`。
  - 同目录另有 snapshot / rollback / clone / dirty 官方脚本。

### 4.3 v0.7.1 相关风险（已列入待验证事项）

- **v0.7.1 不含 ARM 上 pause→resume 的修复**（该修复在 v0.7.2，`Could not restore GICv3ITS state`）→ 鲲鹏侧 4.4 可能整体不可用。
- **issue #1803 明确影响 v0.7.1**：aarch64 上同一镜像与配置的模板会随构建顺序劣化 **1.2–1.7 倍**，吞吐降至 0.64 倍，x86_64 上不可复现 → **每轮测量前冷重启 + 重建模板**，并记录构建顺序。
- issue #1611：96 核 aarch64 上 cube-bench 吞吐可能为恒定值（v0.6.0，已关闭，v0.7.1 是否完全修复未确认）。

### 4.4 公平性关键约束

- **PVM 仅 x86_64** → 两台机器都必须**原生 KVM 裸金属**（红线 R1）。
- 两平台 guest 内核是**不同文件、不同 config**；guest 镜像也是**不同 tarball**；官方只有 `sandbox-code:latest` 是 Multi-Arch。
- 架构固有、**无法对齐**的差异：基础页大小（4 KB vs 64 KB）、中断控制器恢复成本（aarch64 有 vGIC 逐字重放）、balloon free-page 上报默认值不同、guest PMU 可能不可用。
- 上游官方性能报告**只有 x86_64 数据**（Intel Xeon 8255C），不能当 ARM 基线。

---

## 5. git 与仓库现状（重要）

| 项 | 状态 |
|---|---|
| 当前分支 | `feat/cube-src-test-doc` |
| 分支基点 | **`tmp/tt1` 的提交 `7120504`** |
| ⚠️ 关键 | **`master` 里没有 `slides/tt1`**。从 `master` 建分支会把 `slides/tt1` 从工作区移走。**切勿从 master 建分支。** |
| 未提交 | `slides/for_update_0928_u1.pptx`（**用户正在改，绝对不要提交**） |
| 未跟踪 | `slides/tt1/~$checking_points.xlsx`（Excel 锁文件，**不要提交**；`.gitignore` 只忽略了 `~$*.pptx`，没有 xlsx 规则） |
| `.dsh.local/git-flow.toml` | 本 session 的 claim 被**手工改过**（`branch = "feat/cube-src-test-doc"`、`worktreeName = "[MAIN]"`）。原因是 `git_start` 会从 `master` 建分支从而丢文件；手工建分支 + 改 claim 是当时的解法 |

> 新设备上若仓库状态不同，先确认 `git rev-parse --abbrev-ref HEAD` 与 `git ls-tree master -- slides/tt1`，再决定怎么处理。

---

## 6. 未决事项 / 下一步

1. **等用户定稿配置名称清单**（增删、分组），并确认**哪些项属于「操作者必须按规格配好」、哪些属于「操作者只需核对记录」**。
2. 清单定稿后：把手册**第 2 篇（2.2 / 2.3）与 4.0** 的表格与清单对齐。
3. `CubeSandbox-待验证事项.md` 中 **A0（规格完整性）优先级最高**：测试规格清单里仍有 4 处「待确认」（内存规格、内核版本、容器操作系统、核数），且 x86 列的机型与内存速率自相矛盾（列名写 9654，数据全是 9755）。**规格不完整或有矛盾就选不出该用哪台机器。**
4. 用户会**自行**修改测试规格清单，**我不要改那个 Excel**。
5. 主文档中所有结论**均未在实机验证**（用户未提供 SSH）。

---

## 7. 本地参考资料

| 路径 | 内容 |
|---|---|
| `.dsh.local/scratch/CubeSandbox/` | 上游源码 clone（commit `80614ab`） |
| `.../docs/zh/guide/self-build-deploy.md` | 从源码构建部署的权威依据 |
| `.../docs/zh/guide/bare-metal-deploy.md` | 裸金属部署与 ARM64 限制 |
| `.../docs/zh/guide/downloads.md` | 资产命名与版本 pin |
| `.../deploy/release-assets.yaml` | 内核 / 镜像 tag 钉死 |
| `.../deploy/one-click/env.example` | 全部目标机变量（415 行） |
| `.../deploy/one-click/build.env.example` | 构建期变量 |
| `.../examples/cube-bench/` | 并发创建基准（Go） |
| `.../examples/snapshot-rollback-clone/bench_pause_resume_concurrency.py` | pause/resume 基准 |
| `.../docs/zh/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md` | 官方基线（**仅 x86_64**） |

上游 issue / PR 值得看的：#1803（ARM 模板顺序漂移）、#1865（aarch64 vGIC 恢复 ioctl）、#1826（balloon 默认值按架构不同）、#1658（ARM pause/resume 修复，v0.7.2）、#1611（aarch64 吞吐异常）。
