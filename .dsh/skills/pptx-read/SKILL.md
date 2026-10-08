---
name: pptx-read
description: 只读地查看 .pptx 演示文稿——用 Windows 上真实的 PowerPoint 引擎把每页渲染成 PNG，并抽出全部文字与演讲者备注，使 Agent 能真正"看到"幻灯片版式、图表和配图，而不只是读到文字。当用户要求读取、总结、分析某份 PPT，或说"上下文在 PPT 里"、需要从演示文稿中提取资料时使用。绝不修改原文件。
whenToUse: 用户提到"读取/打开这个 ppt"、"PPT 里写了什么"、"总结这份演示"、"PPT 里的上下文/资料"，或给出 .pptx 路径需要内容分析时加载；用户要求把演示文稿内容作为后续研究或写作的输入时也加载。
---

# pptx-read — 只读地"看到"PPT

把一份 `.pptx` 变成 Agent 能消费的两样东西：

- **`text.md`** —— 逐页、逐形状的全部文字，用 `read` 工具读；
- **每页一张 PNG** —— 每页的**真·PowerPoint 渲染**，用 `read_image` 工具看。

渲染走 Windows 宿主机上已装的 PowerPoint（经 WSL interop 调 COM），因此**保真度 100%**：Agent 看到的和用户在 PowerPoint 里看到的是同一个渲染器。这比 LibreOffice 方案（约 85%，字体替换最易翻车）可靠，且**零安装**。

**但有两条路径，先选对再跑：**

| | 默认（COM 渲染） | `--no-render`（纯 python-pptx） |
|---|---|---|
| 产物 | 每页 PNG + PDF + `text.md`（文字来自 PowerPoint） | `text.md`（来自 python-pptx，**含演讲者备注**） |
| 要 PowerPoint 吗 | ✅（借用用户那次实例） | ❌ 完全不碰 |
| 要网络吗 | ❌ | ❌（复用缓存里的 python-pptx） |
| 实测耗时（9 页） | ~8s | **0.24s** |

只想知道"PPT 里写了什么"时，`--no-render --dump` 就够了，而且与用户是否开着 PowerPoint 无关；只有需要**看版式/图表/配图**时才走默认路径渲染。

## 硬性约束

1. **只读，永不写入原文件。** 两条路径都先复制再读：COM 路径复制到 Windows 临时目录并以 `ReadOnly` 打开，`--no-render` 路径复制到 `.dsh.local/pptx-read/<slug>/`（WSL 侧，已 gitignore）。任何"改 PPT"的需求都不属于本技能。
2. **不装任何东西，也尽量不联网。** COM 路径零依赖；python-pptx **优先复用 `.dsh.local/uv-cache/archive-v0/` 里已解开的副本 + uv 自带的同版本解释器**（纯离线，实测 0.2s），只有缓存确实缺失时才退到 `uv run --with python-pptx` 联网解析（缓存目录 `.dsh.local/uv-cache`，已 gitignore，不污染系统 Python）。**每个外部调用都必须有超时**：PowerShell 一步（`--timeout`）、python-pptx 一步（缓存 120s，`uv` 用 `--net-timeout`）——没有超时的阻塞调用会无声挂死整轮运行。
3. **PNG 是给眼睛的，text.md 是给上下文的。** 先读 `text.md` 建立全局理解，**只对真正需要看版式/图表/配图的页**调 `read_image`。一份 20+ 页的 deck 全量读图会吃掉大量上下文——这是本技能最容易犯的错。
4. **不往仓库里写文件。** 产物只落在两处机器本地 scratch（Windows 临时目录，或 `.dsh.local/pptx-read/`，后者已 gitignore）；只有显式给 `--out` 才复制到工作区某个目录。
5. **绝不打扰用户已经打开的 PowerPoint。** `PowerPoint.Application` 是**单实例** COM 服务器——用户开着 PowerPoint 时，`New-Object -ComObject PowerPoint.Application` 会**附着到用户那个实例上**，而不是新建一个。所以：只关自己打开的那一份、**只在实例是自己启动时才 `Quit()`**。违反这条的后果是用户的 PowerPoint 被关掉，或留下一个无窗口实例让用户"再也打不开自己的 ppt"。细节见下文「副本、轮次清理与实例隔离」。

## 副本、轮次清理与实例隔离

这是本技能**最容易伤到用户**的部分，改动脚本前务必读懂。

### 每轮一份副本，下一轮开始前清掉

有两个 scratch root，各归一条路径：COM 路径是 Windows 侧的 `%TEMP%\pptx-read\`，`--no-render` 路径是 WSL 侧的 `.dsh.local/pptx-read/<slug>/`（不能用 `/tmp`——每次 bash 调用都是新的 `/tmp`，产物活不到下一次调用）。**每次运行开始时把上一轮的目录整个删掉**，然后重新复制源文件进去。因此：

- 上一轮的副本与渲染产物，在本轮开始前必然已不存在；
- 临时目录里任何时候只有**当前这一轮**的东西；
- 源文件从头到尾只被 `Copy-Item` 读过，**PowerPoint 永远不碰它**。

源文件可能在两次运行之间被用户编辑（实测遇到过：某一轮 23 页，下一轮 24 页）。所以**每一轮都要重新复制、重新渲染**，不要复用上一轮的 `text.md` 或 PNG。

### 副本必须改名，不能沿用原文件名

副本名固定为 `deck-<slug>.pptx`，`slug` 是**源文件完整 Windows 路径**的 sha1 前 10 位：

```
slides/演示.pptx                 -> deck-7971155b1b.pptx
.dsh.local/samename/演示.pptx    -> deck-d4100256fc.pptx   # 同名文件，互不冲突
```

原因是历史问题：**不同路径下的同名 pptx 会互相影响**（PowerPoint 内部按文件名认定身份）。用路径派生的 slug 命名，同名文件天然分开；纯 ASCII 也免掉任何代码页问题。

### 绝不碰用户已打开的 PowerPoint

`PowerPoint.Application` 是单实例 COM 服务器：用户开着 PowerPoint 时，`New-Object -ComObject PowerPoint.Application` **不会新建实例，而是附着到用户那个实例上**。于是：

- 无脑 `$app.Quit()` → **把用户的 PowerPoint 连同他正在编辑的文稿一起关掉**；
- 留下一个无窗口（`WithWindow=0`）的实例 → 用户之后双击 pptx 会落进这个不可见的窗口，表现为**"我的 ppt 打不开了"**。

脚本的处理：

| 时机 | 动作 |
|---|---|
| 创建 COM **之前** | 探测 `Get-Process POWERPNT`，记录 `POWERPOINT_WAS_RUNNING` |
| 运行中 | 只 `Close()` 自己打开的那一份；借用期间改过的 `DisplayAlerts` 用完还原 |
| 结束时 | **仅当实例是本次运行自己启动的**才 `Quit()`；否则打印 `LEFT_USER_POWERPOINT_RUNNING=1` 并原样留着 |

因此输出里出现 `POWERPOINT_WAS_RUNNING=1` 与 `LEFT_USER_POWERPOINT_RUNNING=1` 都是**预期行为**，不是错误——它表示"你当时开着 PowerPoint，我借用了它，并且没有关掉它"。

## 先看环境：`--check`

读之前先确认三件事：PowerShell 能不能调到、interop 开没开、PowerPoint 现在开着什么。

```bash
.dsh/skills/pptx-read/scripts/read_pptx.sh --check
```

它只读地把 COM 附着到 PowerPoint（**绝不 `Quit()`**，也不打开/关闭任何文稿），然后报告：

```
distro=Ubuntu
powershell=/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
interop=enabled (enabled)
appendWindowsPath=false
uv=uv 0.11.8 (x86_64-unknown-linux-gnu)
app_version=16.0
app_visible=msoTrue
presentations=1
open=language_summary.pptx saved=0 readonly=0
```

- `appendWindowsPath=false`（本机就是）时 Windows 的 exe 不在 PATH 上，脚本会退到绝对路径，并把实际用的那一条打出来；
- `open=` 那几行是**排障关键**：一次被超时杀掉的运行可能在用户的 PowerPoint 里留下隐藏副本（`deck-<slug>.pptx`），这里能看见；`saved=0` 表示用户那份文稿有未保存改动，渲染前最好先让他存档。

## 快速开始

```bash
.dsh/skills/pptx-read/scripts/read_pptx.sh "slides/某份演示.pptx"                      # PNG + PDF + text.md
.dsh/skills/pptx-read/scripts/read_pptx.sh "slides/某份演示.pptx" --no-render --dump   # 只要文字，不用 PowerPoint
```

输出（关键行）：

```
STATUS=COPIED
COPY=C:\Users\...\Temp\pptx-read\<slug>\deck-<slug>.pptx
POWERPOINT_WAS_RUNNING=1
SLIDES=24
PAGESIZE_PT=960.4x540
TEXT_MD=C:\Users\...\Temp\pptx-read\<slug>\text.md
PNG_DIR=C:\Users\...\Temp\pptx-read\<slug>
PDF=C:\Users\...\Temp\pptx-read\<slug>\deck.pdf
STATUS=DONE
LEFT_USER_POWERPOINT_RUNNING=1
WSL_WORKDIR=/mnt/c/Users/.../Temp/pptx-read/<slug>
---
text : /mnt/c/Users/.../Temp/pptx-read/<slug>/text.md
png  : /mnt/c/Users/.../Temp/pptx-read/<slug>
work : /mnt/c/Users/.../Temp/pptx-read/<slug>
copy : /mnt/c/Users/.../Temp/pptx-read/<slug>/deck-<slug>.pptx
```

**直接用 `---` 之后那几行给的路径**：`read` 读 `text :`，`read_image` 读 `png :` 目录里的 PNG（**文件名随 PowerPoint 界面语言变**，中文是 `幻灯片N.PNG`，英文是 `Slide N.PNG`——别按文件名猜，看 `png :` 那一行）。`copy :` 是本次运行使用的**私有副本**，所有读取都只读它；`--no-render` 时它是 WSL 侧路径而不是 `/mnt/c/...`。

> **一次运行拿全你要的东西。** 因为每次运行开始会清空 scratch root，所以别分两次跑（先跑一次取文字、再跑一次 `--dump`）——第二次会把第一次的 PNG 删掉。需要什么就在**同一次调用**里把参数给全。

## 参数

| 参数 | 作用 |
|---|---|
| `--no-render` | 只要文字，且**完全不用 PowerPoint**：python-pptx 读一份 WSL 侧副本，输出 `text.md`（含备注），离线、约 0.2s |
| `--dump` | 额外跑 python-pptx 结构化 dump：形状名/类型、`pt` 坐标尺寸、表格逐格（**合并单元格补回 origin 的值**）、**演讲者备注** |
| `--out DIR` | 把 `text.md` 和 PNG 复制到工作区某目录（`DIR/text.md`、`DIR/png/`） |
| `--width/--height` | 渲染尺寸，默认 1600×900 |
| `--timeout SEC` | PowerPoint 一步的超时，默认 180（`PPTX_TIMEOUT`） |
| `--net-timeout SEC` | `uv` 一步的超时，默认 90（`PPTX_NET_TIMEOUT`） |
| `--check` | 只报告环境（PowerShell / interop / uv / PowerPoint），不做任何读取 |

退出码：`2` 用法或文件不存在，`3` PowerPoint 看不到源文件，`4` PowerShell/COM 失败或超时，`5` `--dump` 一步失败或超时。

## 三个产物，何时用哪个

| | 默认（COM 渲染） | `--no-render` | `--dump` |
|---|---|---|---|
| 产物 | 每页 PNG + PDF + `text.md` | `text.md` | 结构化 markdown 打到 stdout |
| 依赖 | PowerShell + PowerPoint（借用用户实例） | 缓存里的 python-pptx（离线） | 与 `--no-render` 同 |
| 看版式 / 图表 / 配图 | ✅ 唯一途径 | ❌ | ❌ |
| 看**演讲者备注** | ❌ | ✅ | ✅ |
| 表格内容 | ✅ 逐格 `[TABLE RxC]` | ✅ 逐格，合并单元格补回值 | ✅ 逐格 + 几何 |
| 精确几何（`pt` 坐标） | ❌ | ❌ | ✅ |
| 老式 `.ppt` | ✅（COM 能开） | ❌（python-pptx 只支持 OOXML） | ❌ |

**推荐组合**：先 `--no-render --dump` 拿全文 + 备注（0.3s，与用户开不开 PowerPoint 无关），确认哪几页的价值在图里，再单独跑一次默认命令渲染出图。**用户正开着那份 pptx 时也能读**——`--no-render` 不碰 PowerPoint；只有渲染才借用它的实例（`--check` 里 `saved=0` 表示用户有未保存改动，渲染前最好请他先存档）。

## 取某一页

`text.md` 是纯 LF，分页标题是 `## Slide N`，所以严格匹配就能精确切出单页：

```bash
# 某一页的全部文字
awk '/^## Slide 4$/{f=1} /^## Slide 5$/{f=0} f' "$TEXT_MD"

# 某一页的表格
awk '/^## Slide 4$/{f=1} /^## Slide 5$/{f=0} f' "$TEXT_MD" | grep -A20 'TABLE'

# deck 里所有表格
grep -A20 'TABLE' "$TEXT_MD"
```

要不要看图由内容决定：**标题 + 表格 + 备注已经能回答大多数问题**，只有当某一页的价值在图（架构图、流程图、图表）里时才去 `read_image` 对应 PNG。`--no-render` 产出的 `text.md` 与 COM 产出的用同一套 `## Slide N` / `### 形状名` 标题，所以上面的 `awk` 配方两边通用。

## 实现要点（改动脚本前必读）

1. **非 ASCII 路径必须 base64 走 argv。** Windows PowerShell 以 ANSI 代码页接收命令行参数，中文路径会乱码。脚本把 Windows 路径编码成 `base64(UTF-8)`（纯 ASCII）传入 `-SrcB64`，PowerShell 侧解码。
2. **`render_pptx.ps1` 必须保持纯 ASCII。** PowerShell 5.1 把无 BOM 的 `.ps1` 当 ANSI 读，文件里任何中文字面量都会损坏。所有中文只出现在 `text.md` 里（运行时用 `UTF8Encoding` 显式写出），不要写进脚本。
3. **脚本经 UNC 路径执行，但 PowerShell 不能只按名字找。** `-File '\\wsl.localhost\<distro>\...\render_pptx.ps1'` 这条路是对的，因此不需要把脚本复制到 Windows 侧；然而 `/etc/wsl.conf` 里的 `[interop] appendWindowsPath = false`（本机就是）会让 `powershell.exe` 不在 PATH 上，而 `$(powershell.exe ... 2>&1 || true)` 会把 `command not found` 悄悄吞掉，让整轮运行退化成"什么都没做、也不报错"。脚本现在按 **名字 → 绝对路径**（`/mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe`）解析，两者都找不到就**明确报错退出**；`PPTX_POWERSHELL` 可直接指定。
4. **`UV_CACHE_DIR` 必须重定向，而且只作兜底。** 沙箱下 `~/.cache/uv` 只读，所以 `uv` 那条路要指向 `<repo>/.dsh.local/uv-cache`；但正常路径根本不调 `uv`——先从缓存里挑出"解释器 + site-packages"直接跑 `dump_pptx.py`（离线），只有缓存缺失时才联网解析。
5. **每轮清空 scratch root。** COM 路径一开始就删掉整个 `%TEMP%\pptx-read`（不只是 workDir 里的 `*.PNG`/`*.pdf`——上一轮的**副本**同样必须清掉）；`--no-render` 路径删掉 `<repo>/.dsh.local/pptx-read/<slug>`，删除前用 `case "$WORK" in "$SCRATCH_ROOT"/*)` 兜住，避免变量为空时误删。
6. **实例所有权决定要不要 `Quit()`。** 创建 COM **之前**先探测 `Get-Process POWERPNT`；**只有本次运行自己启动了 PowerPoint 才 `Quit()`**，否则原样留着（那是用户的）。无脑 `Quit()` 会连用户正在编辑的文稿一起关掉。
7. **只 `Close()` 自己打开的那一份演示文稿**，绝不遍历 `Presentations` 去关别的。
8. **表格必须单独走 `HasTable`，不能只读 `HasTextFrame`。** 表格文字不在页级 TextFrame 里，只判断 `HasTextFrame` 会**静默丢掉整张表**——而这恰恰是最该拿到的内容。见 `render_pptx.ps1` 里的 `[TABLE RxC]` 分支。
9. **写 `text.md` 前必须把行尾统一成 LF。** `AppendLine` 产出 CRLF，而正文自带 LF，混在一起会让 `awk '/^## Slide 4$/'` 因行尾的 `\r` 匹配失败。这一条和上一条都是"不报错但丢内容/失配"型缺陷，改动脚本后务必回归。
10. **缓存路径必须四级上溯。** `REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"`——脚本在 `.dsh/skills/pptx-read/scripts/`，三级只到 `.dsh`，会把 uv cache 指到 `<repo>/.dsh/.dsh.local/uv-cache`（本机真踩过，那儿躺着 27MB 谁都不看的缓存，等于每次都当"没缓存"重新联网解析）。
11. **python-pptx 的运行器要"解释器 + site-packages"成对验证。** 缓存里的 `archive-v0/<hash>/lib/python3.11/site-packages` 只对 3.11 有效；脚本按版本找解释器（`python3.x` 或 uv 自带的 `~/.local/share/uv/python/cpython-<ver>*`），并真的 `import pptx` 一次才算数，否则换回 `uv`。
12. **合并单元格必须按 span 回填。** python-pptx 只在 origin 格存文本，被覆盖的格子报空字符串（`is_spanned`）；不回填的话，中文汇报表里"向量化 / 调度&并行"这类纵向合并的标签会整列消失——COM 路径不会。见 `dump_pptx.py` 的 `table_rows()`。
13. **形状名与 PNG 文件名都会本地化。** 同一个形状 COM 报 `Rectangle 10`、python-pptx 报 `矩形 10`；PNG 在中文 PowerPoint 里叫 `幻灯片N.PNG`。任何比对/遍历都不能硬编码这些名字。

## 已知坑

- **`/tmp` 在每次 bash 调用后销毁**（每次调用是独立环境）。中间产物**不能**放 `/tmp`：COM 路径用 Windows 临时目录（跨调用持久，WSL 侧经 `/mnt/c/...` 可读），`--no-render` 路径用 `<repo>/.dsh.local/pptx-read/`（沙箱只允许写工作区内）。
- **bash 不能写 `/mnt/c`**（只读挂载/沙箱）。Windows 侧的一切落盘都必须由 PowerShell 完成；WSL 侧只能读 `/mnt/c`。
- **PowerPoint 是单实例的。** 用户开着 PowerPoint 时脚本会**借用**那个实例（`POWERPOINT_WAS_RUNNING=1`），这是正常的；但这也意味着脚本运行期间用户的 PowerPoint 里会短暂多出一份隐藏文稿。
- **WSL 新建路径经 `\\wsl.localhost` 有可见性延迟。** 刚 `cp` 出来的文件可能马上被 Windows 判为不存在（`ERR_NO_SOURCE`），稍后重试即可。源文件放在工作区等长期存在的路径不受影响。
- **PowerPoint 会弹窗阻塞**。脚本已设 `DisplayAlerts`（并在归还实例时还原），但若某份 deck 触发修复提示仍可能卡住；现在 `--timeout` 到点会杀掉 PowerShell 一步并给出诊断（exit 4），而不是无限等。**被杀掉的那一轮不会执行 `.ps1` 的 `finally`**，可能留一份隐藏副本（`deck-<slug>.pptx`）在用户的 PowerPoint 里——用 `--check` 看 `presentations=` / `open=` 确认；用户自己那份文稿不要动。
- **python-pptx 优先走缓存，联网只是兜底。** pypi 这条链路可能极慢或时通时断（实测 TLS 在 ClientHello 之后被重置或直接挂住），所以正常路径不解析索引；万一要联网解析，有 `--net-timeout`（默认 90s，超时 exit 5）。
- **形状名与 PNG 文件名都会本地化**（见实现要点 13）。别按名字对齐两条路径的输出。
- **`.dsh/.dsh.local/uv-cache`（约 27MB）是历史遗留。** 早期版本的 cache 路径少上溯一级落在那里；正确位置是 `.dsh.local/uv-cache`，旧目录可以直接删。

## 自检

0. 先跑 `--check`：`powershell=` 有路径、`interop=enabled`、`python_pptx=` 指到缓存里的解释器、`presentations=` 与运行前一致（没有多出隐藏副本）；
1. `SLIDES=` 的数字与 `png :` 目录里的 PNG 数量一致（**别按 `Slide*.PNG` 匹配**：中文界面的 PowerPoint 写的是 `幻灯片N.PNG`）；
2. `text.md` 用 `file` 确认是 `UTF-8`**且不含 CRLF**，抽查中文不是乱码；
3. 严格分页匹配能取到页（`awk '/^## Slide 1$/'` 有输出）；
4. 若 deck 含表格，`grep -c 'TABLE' text.md` > 0；
5. **运行结束后 `Get-Process POWERPNT` 的进程数，应与运行前一致**（脚本没有多留也没有少留）；
6. 临时目录里只有**当前这一轮**的 `<slug>` 目录；
7. 抽 1–2 页 `read_image` 确认能真正看到内容（背景、图表、中文均清晰）；
8. `--no-render` 的 `text.md`：`file` 说是 UTF-8、不含 CRLF、`awk '/^## Slide 1$/'` 有输出、有备注的页能被 `grep '^> notes:'` 命中（`--no-render` 全程不该出现任何 `POWERPNT` 相关输出，也不该产生 `.dsh/.dsh.local` 之类的新目录）。

## 交付约定

- 默认**不往仓库写文件**。只有用户明确要留档时才 `--out`（建议 `reports/<deck名>/`）。
- 调研结论若要落成笔记，走 `docs/` 的笔记模板，并**先征得用户同意**。
