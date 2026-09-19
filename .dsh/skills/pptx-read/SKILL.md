---
name: pptx-read
description: 只读地查看 .pptx 演示文稿——用 Windows 上真实的 PowerPoint 引擎把每页渲染成 PNG，并抽出全部文字与演讲者备注，使 Agent 能真正"看到"幻灯片版式、图表和配图，而不只是读到文字。当用户要求读取、总结、分析某份 PPT，或说"上下文在 PPT 里"、需要从演示文稿中提取资料时使用。绝不修改原文件。
whenToUse: 用户提到"读取/打开这个 ppt"、"PPT 里写了什么"、"总结这份演示"、"PPT 里的上下文/资料"，或给出 .pptx 路径需要内容分析时加载；用户要求把演示文稿内容作为后续研究或写作的输入时也加载。
---

# pptx-read — 只读地"看到"PPT

把一份 `.pptx` 变成 Agent 能消费的两样东西：

- **`text.md`** —— 逐页、逐形状的全部文字（含备注走 `--dump`），用 `read` 工具读；
- **`SlideN.PNG`** —— 每页的**真·PowerPoint 渲染**，用 `read_image` 工具看。

渲染走 Windows 宿主机上已装的 PowerPoint（经 WSL interop 调 COM），因此**保真度 100%**：Agent 看到的和用户在 PowerPoint 里看到的是同一个渲染器。这比 LibreOffice 方案（约 85%，字体替换最易翻车）可靠，且**零安装**。

## 硬性约束

1. **只读，永不写入原文件。** 脚本先把源文件复制到 Windows 临时目录，再以 `ReadOnly` 打开。任何"改 PPT"的需求都不属于本技能。
2. **不装任何东西。** COM 路径零依赖；`--dump` 用 `uv run --with python-pptx` 临时拉取，装到 `.dsh.local/uv-cache`（已 gitignore），不污染系统 Python。
3. **PNG 是给眼睛的，text.md 是给上下文的。** 先读 `text.md` 建立全局理解，**只对真正需要看版式/图表/配图的页**调 `read_image`。一份 20+ 页的 deck 全量读图会吃掉大量上下文——这是本技能最容易犯的错。
4. **不修改仓库。** 渲染产物默认留在 Windows 临时目录；只有显式给 `--out` 才复制进工作区。

## 快速开始

```bash
.dsh/skills/pptx-read/scripts/read_pptx.sh "slides/某份演示.pptx"
```

输出（关键行）：

```
SLIDES=23
PAGESIZE_PT=960.4x540
TEXT_MD=C:\Users\...\Temp\pptx-read\<slug>\text.md
PNG_DIR=C:\Users\...\Temp\pptx-read\<slug>
WSL_WORKDIR=/mnt/c/Users/.../Temp/pptx-read/<slug>
---
text : /mnt/c/Users/.../Temp/pptx-read/<slug>/text.md
png  : /mnt/c/Users/.../Temp/pptx-read/<slug>
work : /mnt/c/Users/.../Temp/pptx-read/<slug>
```

**直接用 `---` 之后那三行的 `/mnt/c/...` 路径**：`read` 读 `text.md`，`read_image` 读 `<png>/SlideN.PNG`。

## 参数

| 参数 | 作用 |
|---|---|
| `--no-render` | 只要文字，跳过 PNG/PDF 导出（快得多，做内容调研时优先） |
| `--dump` | 额外跑 python-pptx 结构化 dump：形状名/类型、`pt` 坐标尺寸、表格逐格、**演讲者备注** |
| `--out DIR` | 把 `text.md` 和 PNG 复制到工作区某目录（`DIR/text.md`、`DIR/png/`） |
| `--width/--height` | 渲染尺寸，默认 1600×900 |

## 两条路径，何时用哪条

| | COM 渲染路径（默认） | `--dump` 结构化路径 |
|---|---|---|
| 产物 | PNG（真渲染）+ PDF + text.md | markdown/JSON：形状、`pt` 几何、表格、**备注** |
| 依赖 | 无 | `uv`（首次联网拉 python-pptx） |
| 看版式/图表/配图 | ✅ 唯一途径 | ❌ |
| 看**备注** | ❌ | ✅ |
| 精确几何、表格结构 | ❌ | ✅ |
| 老式 `.ppt` | ✅（COM 能开） | ❌（python-pptx 只支持 OOXML） |

**推荐组合**：先 `--no-render --dump` 拿到全文 + 备注（便宜、信息密度最高），确认哪几页有图表要看，再单独跑一次默认命令渲染出图。

## 实现要点（改动脚本前必读）

1. **非 ASCII 路径必须 base64 走 argv。** Windows PowerShell 以 ANSI 代码页接收命令行参数，中文路径会乱码。脚本把 Windows 路径编码成 `base64(UTF-8)`（纯 ASCII）传入 `-SrcB64`，PowerShell 侧解码。
2. **`render_pptx.ps1` 必须保持纯 ASCII。** PowerShell 5.1 把无 BOM 的 `.ps1` 当 ANSI 读，文件里任何中文字面量都会损坏。所有中文只出现在 `text.md` 里（运行时用 `UTF8Encoding` 显式写出），不要写进脚本。
3. **脚本经 UNC 路径执行。** `powershell.exe -File '\\wsl.localhost\<distro>\...\render_pptx.ps1'`——已验证可用，因此不需要把脚本复制到 Windows 侧。
4. **`UV_CACHE_DIR` 必须重定向。** 沙箱下 `~/.cache/uv` 只读；脚本指向 `<repo>/.dsh.local/uv-cache`。
5. **清理旧产物。** 每次运行前删掉 workDir 里的 `*.PNG`/`*.pdf`，否则上一份 deck 的图会被误认成本次的。
6. **用完即退。** COM 用完必须 `$app.Quit()`，否则残留 POWERPNT 进程。

## 已知坑

- **`/tmp` 在每次 bash 调用后销毁**（每次调用是独立环境）。中间产物**不能**放 `/tmp`；本技能因此用 Windows 临时目录做暂存（跨调用持久，且 WSL 侧经 `/mnt/c/...` 可读）。
- **bash 不能写 `/mnt/c`**（只读挂载/沙箱）。Windows 侧的一切落盘都必须由 PowerShell 完成；WSL 侧只能读 `/mnt/c`。
- **PowerPoint 会弹窗阻塞**。脚本已设 `DisplayAlerts=1`，但若某份 deck 触发修复提示仍可能卡住——超时后检查是否有残留 POWERPNT 进程。
- **首次 `--dump` 需要联网**下载 python-pptx；之后走缓存。

## 自检

1. `SLIDES=` 的数字与 `ls <png>/Slide*.PNG | wc -l` 一致；
2. `text.md` 用 `file` 确认是 `UTF-8`，抽查中文不是乱码；
3. 抽 1–2 页 `read_image` 确认能真正看到内容（背景、图表、中文均清晰）。

## 交付约定

- 默认**不往仓库写文件**。只有用户明确要留档时才 `--out`（建议 `reports/<deck名>/`）。
- 调研结论若要落成笔记，走 `docs/` 的笔记模板，并**先征得用户同意**。
