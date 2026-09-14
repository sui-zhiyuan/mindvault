---
name: slides
description: 生成单文件 HTML 演示幻灯片（16:9 固定画布、全中文、离线自包含、无需打印）。当用户要求"做幻灯片/做个 slide/deck/演示页面/汇报页"，或要求把已有报告压缩成若干页对照展示时使用。也适用于需要左右对照（A vs B）的两栏或多栏对比页。
whenToUse: 用户说"做个 slide"、"生成 PPT/幻灯片/汇报页"、"把这份报告变成几页演示"、"做个 X 与 Y 的对比页"时加载。
---

# slides — 单文件 HTML 幻灯片

把内容写成**一个自包含的 HTML 文件**：双击即可在任何浏览器打开，无网络、无构建、无外部依赖。
默认**全中文**、**16:9 固定画布**、**不做打印样式**（用户明确要 PDF 时再单独确认）。

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

## 画布骨架（照抄，勿改缩放算术）

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<title>标题</title>
<style>
  :root{
    --bg:#0f1420; --panel:#161d2e; --fg:#e8ecf5; --muted:#8c98b0;
    --line:#28324a; --accent:#3d7eff; --accent2:#e0a340;
    --serif:'Songti SC','SimSun',Georgia,serif;
    --sans:'PingFang SC','Microsoft YaHei','Noto Sans CJK SC',system-ui,sans-serif;
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

## 版式库

### 页眉（每页顶部，统一 64px 内边距）

```html
<header class="s-head">
  <div class="s-kicker">P1 · 社区全景</div>
  <h2 class="s-title">定位 · 治理 · 成员 · 技术 · 版本</h2>
  <div class="s-sub">玄武（Xuanwu）社区 vs openEuler 社区</div>
</header>
```

### 对照表（对比型演示的主力版式）

两列社区 + 首列维度名，最适合"A vs B"。用 `table-layout:fixed` 保证列宽不被内容撑开：

```html
<table class="cmp">
  <colgroup><col style="width:150px"><col><col></colgroup>
  <thead><tr><th>维度</th><th class="xw">玄武社区</th><th class="oe">openEuler</th></tr></thead>
  <tbody>
    <tr><th>定位</th><td>…</td><td>…</td></tr>
  </tbody>
</table>
```
列头用两条不同色带的 `border-top` 区分两个社区，正文只靠这一处上色，不要每格都染色。

### 指标条（关键数字）

```html
<div class="metrics">
  <div class="metric"><div class="m-val">100+</div><div class="m-lab">特别兴趣小组</div></div>
</div>
```
数值用 `--accent`，标签用 `--muted` 10px 大写式小字。

### 结论横条（页脚上方）

```html
<footer class="verdict">
  <div class="v-label">结论</div>
  <div class="v-text">…</div>
</footer>
```
用 `border-left:3px solid var(--accent)`，左边线是唯一强调手段。

### 证据标签（做事实/规划区分时必用）

```html
<span class="tag tag-plan">规划</span><span class="tag tag-fact">公开</span>
```
`.tag-plan` 用 `--accent2`（琥珀，表示未落地），`.tag-fact` 用 `--accent`（蓝，表示已实现）。
**汇报口径纪律**：把"规划目标值"和"已实现值"并列展示时，必须逐项打标签，否则会误导听众。

## 内容脚本化流程

1. **先定页数与每页的唯一职责**。超出一页一个论点就拆页，宁多一页不要挤。
2. **每页写 3–5 行对照 + 1 条结论**。演示页是"提示板"不是文档，细节留在报告里。
3. **收敛完再写 HTML**：先把每页文案定稿成列表，再套版式，避免边写边删。
4. **自检**（必做）：
   - 在浏览器打开，逐页按 `→` 确认无溢出、无遮挡、翻页与缩放正常；
   - 检查中文有无截断、换行孤字、行高过密；
   - 检查是否残留任何 `http://` / `https://` 外链引用（本技能要求零外部引用）。
   有 Playwright/Chromium 时截图逐页核对最可靠。

## 交付约定

- 单文件 HTML 放 `reports/`（报告类旁挂）或用户指定路径；文件名用 kebab-case。
- 用 `present` 声明交付物后再回复。
- 若产出对应用户已存在的 Markdown 报告，在报告中反引号标注 HTML 路径，保持两处口径一致。
