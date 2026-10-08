# heavy/language_summary.pptx · 第 3 页「腾讯场景介绍 / 字节场景介绍」文案

> 更新：2026-10-08 ｜ 分支：`feat/language-summary`

## 简介

为 `heavy/language_summary.pptx` 第 3 页右侧的「腾讯场景介绍」和「字节场景介绍」两个信息框，生成可直接粘贴的应用描述文案。

任务是"三方并行生成再对比"：**deepseek-flash（本会话）、claude-opus-5.5、gpt-6-sol** 各出一版，交由人工挑选或合并。本目录保存第三轮（最新、口径已稳定）的输入与三方产出。

**为什么放在这里而不是 `.dsh.local/`**：`.dsh.local/` 被 `.gitignore` 的 `*.local` 命中，**不受版本控制**；工作环境迁移前把有效产出落到受跟踪的路径，避免丢失。

**边界**：PPTX 由人工修改和提交，本记录不含任何 `.pptx` 改动。

## 术语列表

| Term | Full Name | Meaning |
|---|---|---|
| LAS | Lake AI Service | 火山引擎「**AI 数据湖服务**」，孵化于字节跳动大模型训练场景。**注意**：与同缩写的企业级「湖仓一体分析服务」是两个产品 |
| veDaft | — | LAS 中封装的 Daft，`daft.read_las_dataset` 等接口 |
| Lance | — | 面向 AI 的新一代列式湖格式，Rust 实现，支持免重写加列（schema evolution）与点查询列投影；LAS 湖存储的核心格式 |
| E2B | — | 第三方 Agent 沙箱 SDK，CubeSandbox 与其 drop-in 兼容 |
| BMI5 | Bare Metal Instance 5 | 腾讯云内存型裸金属实例，CubeSandbox 官方基准的测试机型（375 GiB 内存） |
| PVM | — | CubeSandbox 在普通云服务器上启用嵌套 KVM 的部署方式，**仅支持 x86_64** |

## 核心内容

### 文件清单

| 文件 | 内容 |
|---|---|
| `page3-source.md` | 第 3 页原始内容文字化（页面结构、本次改版决定、版式与字号、Slide 3 抽取原文） |
| `brief-v3.md` | 第三轮下发给两个 subagent 的简报，附前两轮口径演进 |
| `out-deepseek-flash-v3.md` | deepseek-flash 产出（①②③④ 四条，无导语；**唯一带字节量化数字**） |
| `out-opus-v3.md` | claude-opus-5.5 产出（导语 + 1–4 编号，最像讲稿） |
| `out-gpt-6-sol-v3.md` | gpt-6-sol 产出（段落式无编号；**唯一带官方实测数字**，正文含免责声明） |

三方 v3 均为**纯文本、无 markdown 标记**，可直接粘进 PowerPoint 文本框。

### 口径变更（两轮被否的原因）

| 轮次 | 口径 | 结果 |
|---|---|---|
| v1 | 三段式：① 客户/业务是什么 ② **这个场景里 Rust 落在哪** ③ **为什么是鲲鹏的亲和优化点** | **被否**。②③ 不是这一页要讲的东西：左侧已有一整列说明 Rust 落在哪；本页也不是优化点页 |
| v2 | 只讲"解决了哪些问题" | 方向对了，但交付格式是报告体（`**摘要**：`、加粗、来源分节），**粘不进 PPT**，还得人工清符号 |
| v3 | 位置空间给定，形式交由模型自定；硬性要求纯文本可直接粘贴 | 当前状态 |

**只有一件事要讲**：腾讯 / 字节分别用什么、**解决了哪些问题**。

### 版面（第 3 页本次改版）

页面 960 × 540 pt。「腾讯场景介绍」与「字节场景介绍」占满右侧整栏（x 490 → 927，宽 ≈ 437 pt；y 80 → 510，高 ≈ 430 pt），上下排布、**每个约 437 × 200 pt**。原右侧里程碑甘特（2026-09-30 / 2026-12-30 / 2027-3-30）**移到别的页**。左侧场景表与基础库表不动。

排版参照：本页现有项目说明框 125 × 65 pt、10 pt 字、约 55 字。换算后每个框 **350–450 字**为舒适区（10–12 pt），超 500 字会挤。

### 可复用的事实结论

**1. 产品对应关系**

- 「腾讯场景介绍」→ **CubeSandbox**（腾讯云开源的 AI Agent 沙箱）
- 「字节场景介绍」→ **LAS**，其主体/主力引擎是 **Daft**

> ⚠️ 原始需求的表述是"腾讯和字节分别使用 daft 和 cubesandbox 解决哪些问题"，字面顺序与括号内说明（"las 主要是 daft，腾讯就是 cubesandbox"）相反。本目录按**腾讯→CubeSandbox、字节→Daft** 执行。依据：CubeSandbox 是腾讯云自己的开源项目（github.com/TencentCloud/CubeSandbox），Daft 是 LAS 的引擎，反向映射不成立。**此假设尚未经人工最终确认。**

**2. LAS 是哪个产品（易错点，已定案）**

火山引擎有**两个同缩写 LAS** 的产品：

| 产品 | 文档线 | 与本页关系 |
|---|---|---|
| **AI 数据湖服务**（Lake AI Service） | `docs/LakeAIService/*` | 多模态数据湖、Lance、Daft/veDaft、对接火山方舟 ← **本页用这个** |
| 湖仓一体分析服务（Lakehouse Analytics Service） | `docs/LakeHouseAnalyticsServiceLASprivatization/*` | Serverless 湖仓分析，Spark/Presto/Flink，与 Daft/Lance 无关 |

定案证据：`https://www.volcengine.com/product/las` 的**页面标题即「AI 数据湖服务 LAS-火山引擎」**，正文含"深度优化新一代湖格式 Lance、Iceberg""内置数百个多模态数据处理算子""深度优化 Ray、PySpark""无缝对接火山方舟"。且本页里程碑"算子下推到 Lance"与该产品线直接对应。

**3. CubeSandbox 的 ARM64 / 鲲鹏事实（来自本地 clone，比官网更细）**

- 软件本体**已原生支持 aarch64**：v0.5.0 与 Arm 工程团队联合完成全栈适配（SysCtrl 由 x86 专属 PIO 改写为 MMIO、引导路径改用 UEFI 替代 SeaBIOS、seccomp 规则按 AArch64 系统调用号重写）；v0.5.1 修复了 64KB 页下快照不完整的问题（dirty bitmap 粒度改用宿主机页大小）。
- **部署方式有限制**：PVM 宿主机内核**仅 x86_64**；ARM64 必须用**原生 KVM 的物理机 / 裸金属**。
- 因此**鲲鹏（原生 KVM 的 ARM64 服务器）恰好落在其唯一推荐的 ARM64 部署形态上** —— 这是本页最有价值的落点。但**本轮 v3 口径已删去鲲鹏优化点论述**，此结论留作后续页面备用。

**4. 数字有两套口径，别混用**

| 来源 | 数字 | 含义 |
|---|---|---|
| 官网首页 | 冷启动 `<60ms`、内存开销 `<5MB`、密度 `1000+` | 标称值；`<5MB` 指沙箱**自身额外开销** |
| [官方基准](https://cubesandbox.com/blog/posts/2026-06-01-cubesandbox-perf-benchmark) | 串行创建平均 `47.8ms`；1000 实例单台均摊 `~25.7MB` | BMI5 裸机实测；`25.7MB` 是**含 Guest 内核的整机均摊** |

两者定义不同、**不矛盾**，但同一页同时出现会被追问，**建议定稿只留一套**。

## 延伸

### 三方产出对比

| | 形式 | 数字口径 | 字节量化 | 篇幅 |
|---|---|---|---|---|
| deepseek-flash | ①②③④，无导语 | **两套都给** | **有**（存储成本 1/4、8×A100 60%→96%、交付缩短 40%） | 340 / 360 字 |
| opus | 导语 + 1–4 编号 | 只有官网口径 | 无（误判"官网无量化数据"，实为简报未转述） | 400 / 420 字 |
| gpt-6-sol | 段落式、无编号 | 只有实测口径，**正文自带免责声明** | 无 | 415 / 424 字 |

合并建议：取 deepseek-flash 的字节量化 + opus 的腾讯讲稿感 + gpt-6-sol 的实测数字与免责姿态，统一成一套口径后压到 ~300 字。

## Q&A

**Q：为什么不直接放进 `docs/`（mdBook）？**
A：`AGENTS.md` 规定 agent 不得自行往 `docs/` 写东西，且要求走知识笔记模板 + `docs/SUMMARY.md` 条目。本内容是工作记录而非知识笔记，故放在 `work-records/`，不影响 mdBook 构建（`book.toml` 的 `src = "docs"`）。

**Q：`.dsh.local/scratch/pptx-p3/` 里还有 v1、v2 的草稿，要保留吗？**
A：已被否，未纳入本目录。如需追溯演进，见上文「口径变更」表。工作环境迁移后该 scratch 目录不会保留。

## 参考资料

- CubeSandbox 官网：https://cubesandbox.com/ （访问日期 2026-10-08）
- CubeSandbox 官方性能基准：https://cubesandbox.com/blog/posts/2026-06-01-cubesandbox-perf-benchmark （2026-10-08）
- 火山引擎 AI 数据湖服务 LAS 产品页：https://www.volcengine.com/product/las （2026-10-08）
- LAS 产品概述（BytePlus）：https://docs.byteplus.com/en/docs/Byteplus_LAS/What_is_AI_data_lake_service （2026-10-08）
- LAS 的 Daft 组件文档：https://docs.byteplus.com/id/docs/Byteplus_LAS/Overview （2026-10-08）
- 基于 veDaft 的数据集使用：https://docs.byteplus.com/en/docs/Byteplus_LAS/Daft-dataset-usage （2026-10-08）
- Lance 智驾案例（第三方转载）：https://server.it168.com/a2025/0826/6896/000006896386.shtml （2026-10-08）
- 本地 CubeSandbox clone：`31d911e`（2026-09-11），ARM 支持博客 `docs/zh/blog/posts/2026-07-08-cubesandbox-arm-support.md`、changelog `v0.5.0.md` / `v0.5.1.md`、性能基准 `docs/zh/blog/posts/2026-06-01-cubesandbox-perf-benchmark.md`

## 待办

- [ ] 人工确认产品对应关系（腾讯→CubeSandbox / 字节→Daft）无误
- [ ] 三选一或合并成终稿；决定留哪一套数字口径
- [ ] 决定 gpt-6-sol 正文里的免责声明是否保留
- [ ] 可选：让 opus 补一版带 LAS 量化数字的重写
- [ ] 文案定稿后由人工写入 PPTX（本目录不含 pptx 改动）
- [ ] 分支收尾：`git_complete`
