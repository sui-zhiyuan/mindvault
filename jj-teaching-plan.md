# Jujutsu（jj）教学计划

> 更新：2026-09-17
> 学习者背景：Git 熟练、Linux CLI 熟练、jj 零基础
> 版本基线：jj 0.45.1、git 2.43.0（jj 要求 git ≥ 2.41）、Ubuntu 24.04、fish shell

## 教学目标

课程结束后，学习者应能：

1. 不查文档完成「建仓 → 改文件 → 组织提交 → 查看历史 → 回到历史版本」的日常闭环；
2. 用 jj 的自动 rebase / squash / split 安全地改写历史，并知道何时用 `jj undo` 兜底；
3. 完成一次完整的多人协作：bookmark、remote、push、fetch、rebase 到远端最新、解决冲突；
4. 说清 change id / commit id、`@`、operation log、immutable commits、冲突对象这五个核心机制；
5. 明确知道哪些 Git 习惯在 jj 里是错的（尤其是“分支自动前进”和“暂存区”）。

## 教学方式（约定）

- **一步一步推进**：每个回合只推进一个模块；模块验收通过才进入下一个。
- **命令由学习者亲手敲**：讲师只给命令、预期输出形态和验收标准，不代跑操作（除非学习者明确要求）。
- **报错原样贴回**：不做二次转述，讲师据此判断真实卡点。
- **计划即进度**：本文件下方的「进度表」由讲师在每步完成后更新。
- 实验目录：`/home/suine/projects/jj-playground`（学习者自建，M1 与 M2 的图都在这里）；**不在真实项目仓库（如 mindvault）里试错**。

## 概念映射总表（Git → jj）

| Git 概念 | jj 对应物 | 关键差异 |
|---|---|---|
| `.git` 仓库 | `.jj`（colocate 时与 `.git` 并存） | jj 是前端，存储后端可换；今天生产可用的是 Git 后端 |
| commit | commit | jj 的 commit 额外带 **change id** |
| HEAD / detached HEAD | `@`（工作副本 commit） | 没有“HEAD 指向分支”；`@` 永远是真实 commit |
| 暂存区 / index | 不存在 | 改动直接属于 `@`，无需 `git add` |
| 工作区脏改动 | 自动 snapshot 进 `@` | 每条命令开始前自动快照 |
| `git stash` | 不需要 | 半成品留在 `@`，`jj new` 即“翻页” |
| 分支 branch | bookmark | 无“当前分支”，bookmark **不会自动移动** |
| `origin/main` | `main@origin`；主线别名 `trunk()` | 语义更显式 |
| commit message | description | 任意 commit 随时可改（`jj describe`） |
| `git reflog` | operation log | 任何操作可 `jj undo`，可 `--at-op` 穿越查看 |
| `git rebase` | `jj rebase` + 自动重挂后代 | 改写后后代自动 rebase |
| 冲突（需 `--continue`） | 冲突是可存储对象 | 有冲突也能提交，无需 continue/abort |
| `git revert` | `jj revert` | 同概念 |

## 模块路线图

> 每个模块固定四段：**学习目标 / 涉及命令族 / 验收标准 / 交付物（学习者要贴给讲师看的东西）**。

### M0 准备与建仓（已完成 2026-09-17）

- 学习目标：理解 `.jj` 与 `.git` 的关系；理解 `jj log` 里 `@` 与 `root()` 的含义；理解 change id 与 commit id 的“双 id”现象。
- 涉及命令族：`jj config set --user`、`jj git init --colocate`、`jj st`、`jj log`。
- 验收标准：能解释 `jj log` 输出中每一列是什么；能说出当前仓库里有几个 commit、`@` 的父是谁。
- 交付物：`jj config list --user` 的输出 + `jj log` 的输出 + 对上述两点的口头解释。
- 验收记录（2026-09-17）：change id / commit id / `@` / `@-` / `root()` 五项理解正确；
  **`(empty)` 的含义需修正** —— 它是“该提交内容与其父相同”，不是“最早祖先提交”。

### M1 第一次提交与快照模型（已完成 2026-09-18）

- 学习目标：彻底理解“工作副本即 commit”“无暂存区”“description 与 change id 的稳定性”。
- 涉及命令族：`touch`/`echo` 建改文件、`jj st`、`jj diff`、`jj describe`、`jj new`。
- 验收标准：能预测每条 jj 命令之后 change id / commit id 各自是否变化，并说明原因。
- 交付物：改动前后两次 `jj log`，以及“哪个 id 变了、为什么”的说明。
- 验收记录（2026-09-18）：`(empty)` 已修正；change id / commit id 的“身份 vs 哈希”已理解；
  `jj squash` 语义先答错、后自行纠正为“目标提交保留 change id 但 commit id 变化，
  新建的 `@` 是新 change，源提交变 hidden”。

### M2 历史查看与 revset 入门（已完成 2026-09-18）

- 学习目标：掌握 `jj log/show/diff/file annotate`；能用 revset 表达“我自己的、带描述的、最近 N 个”这类查询。
- 涉及命令族：`jj log -r`、`jj show`、`jj diff -r/--from/--to`、`jj file annotate`、`jj evolog`；revset 运算符 `@ - + :: .. | & ~` 与函数 `trunk() bookmarks() mine() description()`。
- 验收标准：给出自然语言需求，能独立写出对应 revset 并解释结果。
- 交付物：至少 3 条自写 revset 及其输出。
- 验收记录（2026-09-18）：`heads(all())`、`bookmarks() ~ ::main`、`@ | @-` 三条自写 revset 均正确；
  `all()` 与默认 `jj log` 在该图里重合的原因（默认 = `present(@) | ancestors(immutable_heads().., 2) | trunk()`）已讲解。
  遗留修正：被 abandon 的提交只写了 `(hidden)`，漏了 **change offset** —— 单独用 `<change id>` 会报
  `doesn't exist`，必须写 `<change id>/<offset>`（实测 `<change id>/0` 才显示 `(hidden)`）。

### M3 改写历史与自动 rebase（进行中）

- 学习目标：掌握 `squash/split/diffedit/describe/edit/abandon`；亲眼验证“改写一个 commit，后代自动 rebase”。
- 涉及命令族：上述命令 + `jj next/prev`、`jj log` 观察拓扑变化。
- 验收标准：能把“错放进上一个 commit 的改动”搬回去；能拆一个提交成两个；能解释为什么这些操作不需要手工 rebase 后代。
- 交付物：操作前后的 `jj log` 拓扑 + 一句话说明“jj 替你做了什么”。

### M4 回到历史版本与 undo

- 学习目标：区分 `jj new <rev>`、`jj edit <rev>`、`jj restore`、`jj revert`、`jj undo/redo/op restore` 五种“回到过去”的语义差异。
- 涉及命令族：`jj new`、`jj edit`、`jj restore [--from]`、`jj revert -B @`、`jj undo`、`jj redo`、`jj op log`、`jj op restore`、`jj --at-op`。
- 验收标准：给出 5 种需求（看老版本 / 在老版本上改 / 丢弃改动 / 生成反向提交 / 撤销刚才的操作），各自选对命令。
- 交付物：一次“abandon 后 undo 救回”的完整 `jj op log`。

### M5 多人协作：bookmark / remote / push / fetch / 冲突

- 学习目标：理解 bookmark 不自动移动、push 的对象是 bookmark、fetch 不做合并、冲突不中断操作这四件事。
- 涉及命令族：`jj bookmark create/list/move/delete/track`、`jj git remote add`、`jj git push -b/-c/--all/--deleted`、`jj git fetch`、`jj git clone`、`jj rebase -b/-o`、`jj resolve`。
- 验收标准：完成一次双仓库协作往返（本地裸库当远端），并解决一次 rebase 冲突。
- 交付物：两个仓库最终的 `jj log`（能看到 `main@origin`）+ 冲突解决过程说明。

### M6 colocate 与 Git 互操作

- 学习目标：知道 colocate 下哪些 git 命令只读安全、哪些危险；知道 `refs/jj/*` 是什么。
- 涉及命令族：`git status/log/show/diff`（只读）、`jj git` 子命令、`.gitignore` 与 `snapshot.auto-track`。
- 验收标准：能解释“为什么 git 里看到一个 detached HEAD”“为什么 `git log --all` 里有一堆幽灵提交”。
- 交付物：colocate 仓库中 git 侧与 jj 侧的两份输出对照。

### M7 概念深潜与毕业验收

- 学习目标：把 change id vs commit id、operation log、immutable commits、hidden/obsolete/divergent change、revset 求值这五块讲清楚。
- 验收标准：能用自己的话向他人解释 jj 的数据模型，并完成毕业综合练习。
- 毕业练习：在空目录从零完成「建两个并行改动 → 合成一个 → 推送 → 与“同事”的提交汇合 → 解冲突 → 用 revset 审计历史 → 用 op log 回滚一次误操作」的完整流程。

## 进度表

| 模块 | 状态 |
|---|---|
| M0 准备与建仓 | 已完成 |
| M1 第一次提交与快照模型 | 已完成 |
| M2 历史查看与 revset 入门 | 已完成 |
| M3 改写历史与自动 rebase | 进行中 |
| M4 回到历史版本与 undo | 未开始 |
| M5 多人协作 | 未开始 |
| M6 colocate 与 Git 互操作 | 未开始 |
| M7 概念深潜与毕业验收 | 未开始 |

## 常见坑清单（课程中逐步展开）

1. `jj new` 之后 bookmark 不移动 —— 正常，需 `jj bookmark move`。
2. 推送前必须有 bookmark，否则 `jj git push --all` 报 “Nothing changed”。
3. `jj squash` / `jj describe` / `jj new` 不带 `-m`/`-u` 会调起编辑器（本环境 `EDITOR="code --wait"`，会弹 VS Code 窗口）。
4. `push -c` 遇到“无描述的空祖先提交”会被拒绝。
5. 用 `jj edit` 改「带冲突」的提交容易踩坏冲突标记，应 `jj new` + `jj squash`。
6. revset 差集是 `~`（不是 `-`）；`A..B` 与 `A::B` 语义不同。
7. `jj log` 默认不显示全部提交，用 `-r 'all()'`。
8. jj 0.45 仍标注 experimental，1.0 前磁盘格式与选项可能不兼容变化。
9. colocate 仓库里用 git 做写操作可能造成 bookmark 冲突 / divergent change。

## 参考资料

- 安装与配置：<https://docs.jj-vcs.dev/latest/install-and-setup/>
- 教程：<https://docs.jj-vcs.dev/latest/tutorial/>
- Git 命令对照表：<https://docs.jj-vcs.dev/latest/git-command-table/>
- Git 专家视角：<https://docs.jj-vcs.dev/latest/git-experts/>
- revset 语言：<https://docs.jj-vcs.dev/latest/revsets/>
- 仓库：<https://github.com/jj-vcs/jj>（访问日期 2026-09）
