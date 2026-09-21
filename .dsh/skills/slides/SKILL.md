---
name: slides
description: 生成单文件 HTML 演示幻灯片（16:9 固定画布、全中文、离线自包含、无需打印）。按"人工汇报 PPT"的三套骨架排版：标题即结论，一侧图一侧文，结论用通栏横条收口。当用户要求"做幻灯片/做个 slide/deck/演示页面/汇报页"，或要求把已有报告压缩成若干页对照展示时使用。也适用于需要左右对照（A vs B）的两栏或多栏对比页。
whenToUse: 用户说"做个 slide"、"生成 PPT/幻灯片/汇报页"、"把这份报告变成几页演示"、"做个 X 与 Y 的对比页"时加载。
---

# slides — 单文件 HTML 幻灯片

把内容写成**一个自包含的 HTML 文件**：双击即可在任何浏览器打开，无网络、无构建、无外部依赖。
默认**全中文**、**16:9 固定画布**、**不做打印样式**（用户明确要 PDF 时再单独确认）。

目标观感是**人工做的商务汇报页**，不是"把 markdown 放进网页"：浅色底、标题即结论、一侧图一侧文、结论用横条收口。
下文三套骨架与组件全部来自一份真实的汇报 deck（33 页，见「人工汇报 PPT 的模式清单」），照抄即可。

## 硬性约束

1. **单文件、零外部引用**。字体只用系统字体栈（`PingFang SC` / `Microsoft YaHei` / `Noto Sans CJK SC` 等）。
   **不要**引入 Google Fonts、CDN 上的 reveal.js/Slidev、外链图片。需要背景纹理就用纯 CSS。
2. **16:9 固定画布**。一个 `.canvas` 固定 1280×720，用 `transform: scale()` 居中适配窗口。
   不要在 `.slide` 内部做自适应流式布局——固定画布下排版才是确定的。
3. **不要 `@media print`**。除非用户明确要求 PDF/打印。
4. **中文排版**：
   - 字体栈必须含中文字体，正文 `font-weight: 400` 起，中文不要用极细字重（发虚）。
   - 中英混排时给西文与数字加 `font-variant-numeric: tabular-nums`，数字列才对齐。
   - 行高 `1.6` 左右，比纯英文演示更大；每个要点不超过 **2 行**。
   - 不要用首行缩进；不要用全角括号包裹英文术语。
5. **溢出的处理顺序**：先删字，再减行，最后才缩字号。固定画布上溢出是失败，不是"差一点"。
   单页要点上限：分点 ≤7 条、每条 ≤2 行；表格行 ≤7；表内单元格 ≤2 行。
6. **一页只用一套骨架 + 最多 2 个视觉块**。先选骨架（方案 1/2/3），再往里填；不要每页自创布局。
7. **标题必须是结论句**，不是名词短语。反例"沙箱优化进展"；正例"沙箱系统：软件生态多样，需要高并发、低时延、高密度部署，可大比例超分"。

## 画布骨架（照抄，勿改缩放算术）

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<title>标题</title>
<style>
  :root{
    /* —— 浅色汇报模板，换主题只改这一块 —— */
    --bg:#ffffff; --fg:#1b1b1b; --muted:#6b6b6b;
    --title:#b3121b;              /* 标题 / 结论条：深红 */
    --line:#d8dde3;               /* 细分隔线 */
    --head:#dce9f7;               /* 表头浅蓝 */
    --card:#f5f7fa;               /* 中性底 */
    --note:#fdf3c8;               /* 结论 / 建议卡：浅黄 */
    --green:#c9e7c2; --orange:#fbd9b5; --blue:#cfe4f7;
    --accent:#1f5fa8;             /* 强调数字 */
    --serif:'Songti SC','SimSun',Georgia,serif;
    --sans:'PingFang SC','Microsoft YaHei','Noto Sans CJK SC',system-ui,sans-serif;
    /* 栅格常量：下方"页面栅格"一节与基础样式表都引用它们，勿删 */
    --pad:27px; --col:598px; --gap:30px;
  }
  html,body{margin:0;height:100%;background:#05070c;font-family:var(--sans);overflow:hidden;}
  #stage{position:fixed;inset:0;display:flex;align-items:center;justify-content:center;}
  .canvas{width:1280px;height:720px;position:relative;transform-origin:center center;flex:0 0 auto;}
  .slide{position:absolute;inset:0;background:var(--bg);color:var(--fg);
         opacity:0;pointer-events:none;transition:opacity .35s ease;overflow:hidden;}
  .slide.active{opacity:1;pointer-events:auto;}
</style>
</head>
<body>
<div id="stage">
  <div class="canvas" id="canvas">
    <section class="slide active" id="s1">…</section>
    <section class="slide" id="s2">…</section>
  </div>
</div>
<div class="nav">…</div>
<script>
  // 固定 16:9 缩放：务必保留 (iw/ih) 的比较与 scale 计算
  function fit(){
    var c=document.getElementById('canvas');
    var s=Math.min(window.innerWidth/1280, window.innerHeight/720);
    c.style.transform='scale('+s+')';
  }
  window.addEventListener('resize',fit); fit();

  var slides=document.querySelectorAll('.slide'), cur=0;
  function go(n){
    slides[cur].classList.remove('active');
    cur=(n+slides.length)%slides.length;
    slides[cur].classList.add('active');
    document.getElementById('counter').textContent=(cur+1)+' / '+slides.length;
  }
  document.addEventListener('keydown',function(e){
    if(e.key==='ArrowRight'||e.key==='ArrowDown'||e.key===' '||e.key==='PageDown') go(cur+1);
    if(e.key==='ArrowLeft' ||e.key==='ArrowUp'  ||e.key==='PageUp')                go(cur-1);
    if(e.key==='Home') go(0);
    if(e.key==='End')  go(slides.length-1);
    if(e.key==='f'||e.key==='F'){ if(!document.fullscreenElement) document.documentElement.requestFullscreen(); else document.exitFullscreen(); }
  });
</script>
</body>
</html>
```

**坑**：禁止用 `sed`/正则全局替换文件内容——会把 `cur-1`、`/720`、`slides.length-1` 这类算术符改坏。
要改就用**精确的唯一字符串替换**，改完务必重读 `<script>` 段确认运算符完好。

## 页面栅格：三套骨架

坐标单位是 **px（画布 1280×720）**。如果你手上是 PPT 尺寸，换算关系是 **960.4×540 pt × 4/3 = 1280×720 px**，
页边距 19.9pt → **27px**，内容宽 931.6pt → **1226px**。三套骨架共用同一套栅格常量：

```
--pad : 27px      左右外边距（1226 = 1280 − 2×27）
--col : 598px     单栏宽（(1226 − 30) / 2）
--gap : 30px      栏间距
标题带 : left 27, top 20,  width 1226, 高 46
总结带 : left 27, top 76,  width 1226, 高 136
主体区 : left 27, width 1226, 两栏 grid
页脚带 : left 27, top 630, width 1226, 高 66
```

| 骨架 | 结构 | 主体区 | 用在哪 |
|---|---|---|---|
| **方案 1** | 标题 / 左栏 ｜ 右栏 | `top 80, height 616` | 纯对照页：一侧图一侧文，不需要额外结论 |
| **方案 2**（默认首选） | 标题 / **总结带** / 左栏 ｜ 右栏 | `top 226, height 470` | 有前提、背景、目标要先交代，再展开两块内容 |
| **方案 3** | 标题 / 左栏 ｜ 右栏 / **结论带** | `top 80, height 528`，页脚带 `top 630` | 先把两块摆完，最后一句话收口 |

**选骨架的顺序**：先问"这一页要不要先给结论/前提？"要 → 方案 2；"结论是不是压轴？"是 → 方案 3；都不需要 → 方案 1。
**一页只允许一套骨架**，不要在同一页混用两套。

### 方案 2 完整骨架（推荐默认，直接改文字即可）

```html
<section class="slide active">
  <h2 class="s-title">标题写成结论句：谁、做了什么、达到什么量级</h2>

  <div class="band">
    <div class="band-label">背景<br>与目标</div>
    <div class="band-body">
      <div>· <b>名词A</b>：一句话说明它是什么、解决什么问题。</div>
      <div>· <b>名词B</b>：一句话说明它与本项目的关系。</div>
      <div>· <b>商业价值</b>：一句话说明做这件事的收益。</div>
    </div>
  </div>

  <div class="cols m2">
    <div class="pane"><!-- 左：图（架构图 / 流程图 / 步骤条 / 象限图 / 时间轴） --></div>
    <div class="pane"><!-- 右：文（表格 / 编号洞察栏 / 指标卡 / 要点） --></div>
  </div>
</section>
```

方案 1 把 `.cols.m2` 换成 `.cols.m1` 并删掉 `.band`；方案 3 换成 `.cols.m3`，并在末尾加：

```html
<div class="foot"><div class="verdict-bar">结论：一句话收口，不超过 40 字</div></div>
```

## 左右分栏铁律

**一侧图、一侧文，各占约一半**。这是人工 deck 与"网页版 markdown"最大的差别。

| 归"图"（放一侧） | 归"文"（放另一侧） |
|---|---|
| 分层架构图（层 + 模块块） | 表格（含编号列 / 指标列） |
| 流程图、步骤条（带耗时数字） | 编号洞察栏（①②③④ 短句） |
| 生态图、象限散点图 | 指标卡、结论 / 建议卡 |
| 时间轴、阶段条 | 要点 bullet（≤3 条） |
| 双图对照（A 架构 vs B 架构） | 五列表（编号 / 优化点 / 指标 / 交付物 / 难点） |

- **禁止两边都是长文本**；也**禁止两边都是表**（除了 P17 那种"双图夹一表"的三分结构）。
- 图可以是纯 CSS 画的，不需要图片。用 `.stack`（分层图）、`.steps`（步骤条）、`.quad`（象限图）、`.timeline` 四个类就够覆盖全部场景。
- 图里的色块要**有语义且带图例**：绿=已有/达标，橙=非目标部分，浅蓝=中性模块，黄=结论/建议。

## 浅色汇报模板（基础样式表，粘在 `:root` 之后）

```css
/* —— 通用 —— */
.s-title{position:absolute;left:27px;top:20px;width:1226px;
         font-size:27px;font-weight:700;line-height:1.3;color:var(--title);}
.pane{overflow:hidden;}
.pane > .cap{font-size:13px;color:var(--muted);margin:0 0 6px;}

/* —— 总结带（方案 2）/ 页脚带（方案 3 复用 .verdict-bar）—— */
.band{position:absolute;left:27px;top:76px;width:1226px;height:136px;display:flex;}
.band-label{width:76px;flex:0 0 76px;background:var(--title);color:#fff;font-size:17px;font-weight:700;
            display:flex;flex-direction:column;align-items:center;justify-content:center;
            letter-spacing:3px;line-height:1.7;text-align:center;}
.band-body{flex:1;margin-left:14px;display:flex;flex-direction:column;justify-content:center;
           gap:7px;font-size:16px;line-height:1.55;}

/* —— 两栏 —— */
.cols{position:absolute;left:27px;width:1226px;display:grid;
      grid-template-columns:var(--col) var(--col);gap:var(--gap);}
.cols.m1{top:80px;height:616px;}
.cols.m2{top:226px;height:470px;}
.cols.m3{top:80px;height:528px;}
.foot{position:absolute;left:27px;top:630px;width:1226px;height:66px;display:flex;}

/* —— 分层架构图 —— */
.stack{height:100%;border:1px solid var(--line);background:var(--card);
       padding:8px;display:flex;flex-direction:column;gap:6px;box-sizing:border-box;}
.stack .row{display:flex;gap:6px;}
.stack .cell{flex:1;background:var(--blue);border:1px solid #b9d4ee;
             padding:6px 4px;text-align:center;font-size:13px;line-height:1.35;}
.stack .cell.g{background:var(--green);border-color:#a8d39f;}
.stack .cell.o{background:var(--orange);border-color:#eec296;}
.stack .cell.wide{flex:1 1 100%;}
.stack .lbl{font-size:12px;color:var(--muted);padding:2px 4px;}

/* —— 步骤条（带右侧耗时）—— */
.steps{display:flex;flex-direction:column;gap:4px;font-size:13px;}
.steps .st{display:flex;justify-content:space-between;align-items:center;
           background:var(--blue);border:1px solid #b9d4ee;padding:5px 8px;}
.steps .st .ms{color:var(--title);font-weight:700;font-variant-numeric:tabular-nums;}

/* —— 编号洞察栏 —— */
.insights{margin:0;padding:0;font-size:15px;line-height:1.6;}
.insights li{list-style:none;position:relative;padding-left:26px;margin-bottom:10px;}
.insights li::before{content:attr(data-n);position:absolute;left:0;color:var(--title);font-weight:700;}
.insights b{color:var(--title);}

/* —— 表格（细线 + 浅蓝表头 + 窄 No 列）—— */
table.t{width:100%;border-collapse:collapse;table-layout:fixed;
        font-size:13.5px;line-height:1.45;font-variant-numeric:tabular-nums;}
table.t th{background:var(--head);border:1px solid var(--line);padding:6px 8px;
           text-align:left;font-weight:600;}
table.t td{border:1px solid var(--line);padding:6px 8px;vertical-align:top;}
table.t td.no{text-align:center;color:var(--muted);}

/* —— 结论：两种形态 —— */
.verdict{background:var(--note);border-left:4px solid var(--title);
         padding:10px 14px;font-size:15px;line-height:1.55;}
.verdict-bar{flex:1;height:100%;background:var(--title);color:#fff;font-size:16px;
             display:flex;align-items:center;justify-content:center;padding:0 16px;}

/* —— 图例 / 签名块 / 章节导航 —— */
.legend{display:flex;gap:18px;align-items:center;font-size:12px;color:var(--muted);}
.legend i{display:inline-block;width:22px;height:12px;margin-right:6px;vertical-align:-1px;}
.byline{width:150px;background:var(--note);font-size:16px;text-align:center;padding:12px 0;}
.tabs{position:absolute;right:27px;top:14px;display:flex;font-size:12px;}
.tabs span{padding:3px 10px;border:1px solid var(--line);color:var(--muted);}
.tabs span.on{background:var(--title);color:#fff;border-color:var(--title);}

/* —— 象限图（散点用绝对定位摆放）—— */
.quad{position:relative;height:100%;border-left:1px solid var(--line);border-bottom:1px solid var(--line);}
.quad .dot{position:absolute;font-size:11px;white-space:nowrap;color:var(--muted);}
.quad .dot::before{content:'●';color:var(--accent);margin-right:4px;}
.quad .ax{position:absolute;font-size:11px;color:var(--muted);}
```

## 组件选型速查

| 你要表达 | 用哪个 |
|---|---|
| 项目背景 / 目标 / 价值（页顶交代） | 方案 2 的 `.band`（标签块 + 3 条 bullet，每条 1 行） |
| 结论、建议、对齐总结（页脚收口） | 方案 3 的 `.verdict-bar`（红底白字，≤40 字）或 `.verdict`（黄底左边线，2 行） |
| 有编号的清单式论证 | `.insights`（①②③④，每条 ≤2 行） |
| 五列结构化信息（编号/优化点/指标/交付物/难点） | `table.t`，No 列 `td.no` |
| 左右两个方案 / 两个社区对照 | `.cols` + 两侧 `.stack`，中间夹一张 `table.t`（三分结构） |
| 分层系统（控制面 / 应用面 / Host / Guest） | `.stack`，色块分 Rust / 非 Rust 并配 `.legend` |
| 分步骤 + 每步耗时 | `.steps`，耗时靠右用 `.ms` |
| 竞争点取舍 | `.quad` 象限散点 + 右下 `.verdict` 建议卡 |
| 阶段演进 | `.timeline`（三段：年份区间 + 每段 1–2 条要点） |
| 一句话告诉听众"这是谁做的" | `.byline`（黄底姓名块放左下） |

## 人工汇报 PPT 的模式清单（33 页实测）

| # | 模式 | 结构 | 出处 |
|---|---|---|---|
| 1 | 封面 | 大标题 + 左下"部门/作者/日期" | P1 |
| 2 | 结束页 | "Thank you." + 版权声明 | P14 |
| 3 | **方案 2：顶带总结 + 左右** | 标签块 + 3 bullet，左图右文 | P4 / P10 / P12 / P29 / P31 / P32 |
| 4 | **方案 3：左右 + 页脚结论** | 左图右文，底部红底结论条 | P8 / P13 / P26 / P28 |
| 5 | **方案 1：纯左右** | 一侧图一侧文/表，无总结带 | P3 / P11 / P19 / P23 / P25 |
| 6 | 大表 + 右上象限 + 右下建议卡 | 表占左侧 2/3，右侧上象限散点、下黄底"建议方向" | P7 |
| 7 | 双图 + 中表 + 底部图例 | 左右各一张分层图，中间一张对照表 | P17 |
| 8 | 三栏步骤流 + 底表 | Start / Pause / Resume 三栏色块，底部优化点表 | P8 / P27 |
| 9 | 左生态图 + 右编号策略 + 底时间轴 | 左一张大分层图，右上编号 1/2，右下三阶段 | P26 |
| 10 | 全页矩阵 | 行标签（场景/框架/基础库/标准库/生态）× 列项目卡，右侧编号洞察栏 | P30 |
| 11 | 顶带 + 左图 + 右编号技术点 | 上两条并列说明，下左架构图、右下 ①–⑦ 关键技术 | P19 |
| 12 | 单图整页 | 立项表 / benchmark 表 / 截图铺满，用于附录 | P15 / P16 / P33 |
| 13 | 章节导航条 | 右上 4 个 tab，当前章节高亮 | P29 |
| 14 | 签名块 + 结论红条 | 左下黄底姓名 + 底部红底一句话 | P6 / P26 / P29 / P30 |
| 15 | ⚠️ 反例：单框长文 | 整页一个文本框，段落式堆字（"working in progress"） | P20 / P21 / P22 |

**模式 3–5 就是三套骨架**，占全 deck 的 17/33 页——优先用它们；模式 6–11 是骨架的变体，需要时再套。

## 视觉规范：从人工 deck 学到的 8 条

1. **标题即结论**：每页标题都是一个可判断真伪的句子，带具体量级或对比对象，读者只看标题就能复述这页在说什么。
2. **浅色底、深红标题**：白底 + 深红（`#b3121b` 一类）标题，正文近黑；不要用深色主题做汇报页（深色适合技术分享，不适合评审）。
3. **色块承载语义，且必须配图例**：绿/橙区分"我方 vs 非我方"、"Rust vs 非 Rust"，底部一行 `.legend` 交代清楚，正文不再重复解释。
4. **数字带单位、带对照**：实测值 + 括号内目标（"950 390 ms（300 ms，挑战 200 ms）"），不要只写"有提升"。
5. **表格用细灰线 + 浅蓝表头**，No 列最窄且居中；不要在表格里做大面积底色。
6. **每页固定元素**：右下页码、页脚密级（如"XX Confidential"）、左下数据来源小字——三件套固定位置，跨页对齐。
7. **一页最多 2 个视觉块 + 1 个结论**。内容装不下就拆页，宁多一页不要挤（人工 deck 33 页，平均每页 30–60 个形状却是 3 个区块）。
8. **颜色只用于区分，不用于装饰**：一页里的颜色不超过 4 种（标题红、表头浅蓝、语义绿/橙、结论黄）。

## 反例（明确不要做）

- ❌ **整页一个文本框堆段落**（人工 deck 的 P20/P21/P22 就是这么干的，自己都标了 "working in progress"）——段落超过 5 行就该拆成"图 + 编号洞察栏"。
- ❌ **两栏都是长文本**：读者不知道先看哪边。一侧必须是图。
- ❌ **同页混用两套骨架**：比如顶带 + 左右 + 页脚结论三件全上，页面会被压成三条窄带。
- ❌ **标题是名词短语**（"进展汇报"、"方案说明"）——等于浪费了页面最贵的一行。
- ❌ **没有图例的彩色块**：绿橙蓝黄满页，读者只能猜。

## 内容脚本化流程

1. **先定页数与每页的唯一职责**。超出一页一个论点就拆页，宁多一页不要挤。
2. **给每页选骨架**：要前置结论 → 方案 2；要压轴结论 → 方案 3；纯对照 → 方案 1。
3. **给每页分左右**：谁放图、谁放文，图用哪个组件（`.stack` / `.steps` / `.quad` / `.timeline`）。
4. **每页写 3–5 行对照 + 1 条结论**。演示页是"提示板"不是文档，细节留在报告里。
5. **收敛完再写 HTML**：先把每页文案定稿成列表，再套版式，避免边写边删。
6. **自检**（必做）：
   - 在浏览器打开，逐页按 `→` 确认无溢出、无遮挡、翻页与缩放正常；
   - 检查中文有无截断、换行孤字、行高过密；
   - 检查每页是否只有一套骨架、两侧是否一图一文、结论是否 ≤40 字；
   - 检查是否残留任何 `http://` / `https://` 外链引用（本技能要求零外部引用）。
   有 Playwright/Chromium 时截图逐页核对最可靠。

## 交付约定

- 单文件 HTML 放 `reports/`（报告类旁挂）或用户指定路径；文件名用 kebab-case。
- 用 `present` 声明交付物后再回复。
- 若产出对应用户已存在的 Markdown 报告，在报告中反引号标注 HTML 路径，保持两处口径一致。
