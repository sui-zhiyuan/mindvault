# Claude 模型档位：Fable / Opus / Sonnet / Haiku

> 更新：2026-09-18

## 简介

Claude 是 Anthropic 的模型系列，它的**档位名取自文学 / 音乐体裁**：Fable（寓言）、Opus（作品）、Sonnet（十四行诗）、Haiku（俳句）。这四个名字**不是同一个模型的不同"后缀"**，而是能力—延迟—价格阶梯上的不同档位；档位名后面的数字（`5.1`、`4.5`）才是同档位内部的代次。当前（2026-09）阶梯为 **Fable > Opus > Sonnet > Haiku**，另有一个与 Fable 同级但限量发布的 Mythos。本文记录各档位的规格差异、选型方法与成本结构。

Related: 只覆盖 Claude 公开阵容，不含第三方托管平台（Bedrock / Google Cloud / Foundry）的差异定价。

## 术语列表

| Term | Full Name | Meaning |
| --- | --- | --- |
| Fable | —（文学体裁"寓言"） | 能力最强的公开档位，面向高要求推理与长周期 agentic 工作 |
| Opus | —（"作品 / 巨著"） | 复杂 agentic 编码与企业级工作的主力高档位 |
| Sonnet | —（"十四行诗"） | 速度与智能平衡的通用主力档位 |
| Haiku | —（"俳句"） | 最快、最便宜的轻量档位 |
| Mythos | —（"神话"） | 与 Fable 同级能力，仅向 Project Glasswing 参与者限量提供 |
| MTok | Million Tokens | 百万 token，官方定价的计量单位 |
| effort | — | 控制思考投入程度的参数，在同一模型内以智能换延迟与成本 |
| Adaptive thinking | Adaptive Thinking | 自适应思考：思考深度由模型自行决定（Fable / Opus / Sonnet 为常开） |
| Extended thinking | Extended Thinking | 扩展思考：较早期的显式思考模式（Haiku 4.5 使用） |

## 核心内容

### 档位阶梯总览

| | **Fable 5.1** | **Opus 5** | **Sonnet 5** | **Haiku 4.5** |
| --- | --- | --- | --- | --- |
| 官方定位 | 高要求推理、长周期 agentic 工作 | 复杂 agentic 编码与企业级工作 | 速度与智能的最佳平衡 | 最快、接近前沿的智能 |
| 延迟 | 最慢 | 中等 | 快 | 最快 |
| 输入 / 输出（每 MTok） | $10 / $50 | $5 / $25 | $2 / $10 | $1 / $5 |
| 上下文窗口 | 1M | 1M | 1M | 200K |
| 最大输出 | 128K | 128K | 128K | 64K |
| 思考模式 | Adaptive（常开） | Adaptive | Adaptive | Extended |
| 默认 effort | `high` | `high` | `high` | 不支持 effort |
| 可靠知识截止 | 2026-06 | 2026-05 | 2026-01 | 2025-02 |
| API ID | `claude-fable-5-1` | `claude-opus-5` | `claude-sonnet-5` | `claude-haiku-4-5-20251001` |

所有当前档位均支持文本与图像输入、文本输出、多语言、vision 与 tool use。

### 各档位定位与选型

Anthropic 的默认建议：**大多数工作负载从 Opus 5 起步**，只有在 `xhigh` / `max` effort 下评估仍不达标时，才升级到 Fable 5.1。

| 需要什么 | 从哪个模型起步 | 示例场景 |
| --- | --- | --- |
| 最高可用能力 | Fable 5.1 | 运行数小时的智能体会话、多步骤深度研究、把文档 / 电子表格 / 演示稿一路推进到完成 |
| 复杂 agentic 编码与企业工作 | Opus 5 | 持续数小时的自主编码 agent、大规模重构、复杂系统工程、重度视觉工作流、computer use |
| 日常编码 / agentic / 企业负载的速度与能力 | Sonnet 5 | 代码生成、数据分析、内容创作、视觉理解、工具调用 |
| 最低延迟与价格，仍要扩展思考 | Haiku 4.5 | 实时应用、高容量智能处理、成本敏感部署、子智能体任务 |

Fable 5.1 是 Fable 5 的加强版：**价格不变**，长时 agentic 编码、知识工作与研究能力更强，且**缓存读取成本降至四分之一**（按输入价的 0.025 倍计费，其他模型是 0.1 倍）。

### 何时换档位、何时调 effort

调整 effort 通常比换模型更划算，因为它在**同一个模型内部**用智能换延迟和成本：

- Fable 5.1 / Opus 5 / Sonnet 5 支持 effort，从默认值 `high` 起步，按评估结果上下调。
- Opus 4.8 / 4.7 上，介于 `high` 与 `max` 之间的 `xhigh` 是大多数编码与 agentic 用例的最佳设置。
- Haiku 4.5 不支持 effort；它靠 `Extended thinking` 提供推理能力。

### 成本结构要点

- **提示缓存倍率**：5 分钟写入 1.25×、1 小时写入 2×、缓存命中与刷新 0.1×（Fable 5.1 与 Mythos 5.1 为 0.025×）。5 分钟档在一次命中后回本，1 小时档在两次命中后回本。
- **fast mode（快速模式）**：研究预览阶段，仅 Opus 5 / Opus 4.8 支持，按 $10 / $50 换取最高 2.5 倍输出速度，且仅第一方 Claude API 可用（与 Batch API 不可同时使用）。
- **分词器差异**：Claude 4.7 及更高版本与 Mythos Preview 使用新分词器，同一段文本产生的 token 数**约多 30%**，因此"单价更低"不等于"同一份输入更便宜"；Sonnet 4.6 及更早使用旧分词器。
- **推理地理位置**：Claude 4.6 及更高版本可用 `inference_geo: "us"` 锁定美国境内推理，代价是所有 token 类别 **1.1×** 乘数；默认 `global` 为标准价。

## 延伸

### 容易混淆的点

1. **数字是"代"不是"档"**：`Fable 5.1` 与 `Haiku 4.5` 不是同代产品，数字只在同一档位内部表示迭代（如 Opus 4.5 → 4.6 → 4.7 → 4.8 → 5）。
2. **Fable 是后来加在 Opus 之上的新顶档**：早期 Claude 只有 Opus / Sonnet / Haiku 三档，对应"最强 / 均衡 / 最快"；Fable 的加入使阶梯变成四级。
3. **Mythos 不是独立能力档**：`claude-mythos-5-1` 与 Fable 5.1 能力相同、价格相同，仅面向 Project Glasswing 参与者，不属于公开阵容。
4. **别名 vs 快照 ID**：`claude-opus-5` 是别名，会指向最新快照；需要复现结果时钉住带日期的快照 ID（如 `claude-haiku-4-5-20251001`）。
5. **旧世代已停用**：Opus 4.1 / Opus 4、Sonnet 4、Haiku 3.5 已停用，仅在 Bedrock 与 Google Cloud 等部分平台保留。

### 多模型组合策略

把低成本模型与前沿模型配对，让大多数 token 走低费率：

- **执行者 + 顾问**：Haiku / Sonnet 负责主体执行，遇到困难决策时上报给 Fable / Opus。
- **编排者 + 工作者**：编排层用高档位做规划与验收，批量工作委派给 Haiku 4.5（这也是官方给 Haiku 列出的典型场景）。

### 版本升级与弃用

- 选型前先看官方的[迁移指南](https://platform.claude.com/docs/zh-CN/about-claude/models/migration-guide)与[模型弃用](https://platform.claude.com/docs/zh-CN/about-claude/model-deprecations)页面，避免把新项目建在即将退役的快照上。
- 从 Opus 4.8 或更早升级到 Opus 5 时，官方提示整体性能提升较大，提示词需要重新调优。

## Q&A

**Fable 和 Opus 谁更强？**
Fable 5.1 更强，是 Anthropic 广泛发布的能力最强模型；但它更慢、价格翻倍，官方建议先用 Opus 5，只有在 `xhigh` / `max` effort 下仍不达标（高要求推理、长周期 agentic）时才上 Fable。

**Sonnet 5 为什么比 Sonnet 4.6 更便宜？**
$2 / $10 原本是发布时的推介价（截至 2026-08-31），现已转为标准价，原定 2026-09-01 上调到 $3 / $15 的计划取消。

**Haiku 4.5 为什么没有 effort 参数？**
effort 是较新模型（Fable / Opus / Sonnet 5）上的推理投入控制；Haiku 4.5 仍走 `Extended thinking` 路线，因此不支持 effort 调节。

**名字里的数字是版本号还是档次？**
是同档位的代次（版本），不是档次。判断档次看名字本身：Fable > Opus > Sonnet > Haiku。

## 参考资料

- [模型概览](https://platform.claude.com/docs/zh-CN/models/overview) — 阵容对比表、模型 ID（访问日期 2026-09-18）
- [选择合适的模型](https://platform.claude.com/docs/zh-CN/about-claude/models/choosing-a-model) — 选型矩阵、effort 建议、多模型策略
- [定价](https://platform.claude.com/docs/zh-CN/about-claude/pricing) — 各档位价格、提示缓存倍率、fast mode、分词器差异
- [模型 ID 与版本管理](https://platform.claude.com/docs/zh-CN/about-claude/models/model-ids-and-versions) — 别名与快照 ID
- [为成本与智能优化](https://platform.claude.com/docs/zh-CN/about-claude/models/optimizing-for-cost-and-intelligence) — 组合模型策略
