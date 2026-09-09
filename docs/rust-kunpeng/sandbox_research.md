# LLM sandbox 分析

<!--
### 本文件编辑约定（本 session，勿删）

1. 工作流：AI 每轮修改写入工作区；用户 review 后 `git add` AI 的内容入 index；
   用户自己的批注保留在工作区（未暂存）。每轮修改前，AI 必须先通过
   `git status` / `git diff` / `git diff --cached` 读取用户批注，按其意见调整后再动手。
2. 语言风格：参照 Daft 章节「背景 / 应用情况 / 商业价值」——仅列举关键点与关键数据，
   省略为连贯性补全的非重点信息；行文简洁，多要点、少铺垫。
3. 不重复：相同信息不得在不同章节重复出现；已被其他章节覆盖的内容在此处省略或仅引用。
4. 除特殊要求外，每个对话 round 仅能能修改单个 3 级章节，例如（Daft & Lance）/ （背景 / 应用情况 / 商业价值）
   可以参考其他章节，但不得修改其他章节。
-->

## 当前 rust 语言生态组 在 Kunpeng 上 我们做了哪些

> 文档目的（用户批注）：本文是交流前的准备，分三部分——① 我们（rust 语言生态组）当前做了什么；
> ② 我们希望了解对方（LLM 安全组）的哪些信息；③ 最终目标（我们能帮沙箱组做什么）。
> 所有内容须服务于双方找到合作点。

1. 针对 Daft 的性能优化（sve / 调度 / 编译条件）
    场景：数据工程二期（内部项目），针对字节 LAS（AI 数据湖 = 湖存储 Lance + 湖计算 Daft）场景，
          优化 Daft 任务在 Kunpeng 上的表现。
    Daft 是字节/火山引擎 LAS 选型的分布式多模态数据处理引擎（Rust 实现，单机 + Ray 分布式）。
    覆盖 6 条产线：文本 / 视频 / 图片 / 音频 / PDF / 自动驾驶（详见 daft-kunpeng-overall-design.md）。
    我们做了：
    - SVE / 硬件指令加速：SVE cntp UTF-8 字符长度、SVE gather-load 批量取数、ARM CRC32C hash、text offset 快路径
    - 调度：Ray 模式下的 UDF 并发度自适应（不涉及 NUMA）
    - 编译条件：Kunpeng 指令对应 LLVM 优化 path 未配置、无法 target-cpu=native；实测部分 Kunpeng 优化点在编译 Daft 时约 3~5% 提升，场景差异较大
    效果：针对 LAS 场景测端到端吞吐速度。
    状态：已合入 Xuanwu 社区（https://gitcode.com/xuanwu/Daft）

2. Portable SIMD 性能库（规划详见 rust-kunpeng.md）
    定位：计划开源，对标 Google Highway；解决 std::simd 无 stable 计划、且不能依赖 std::arch 的痛点。
    状态：已开始实施，尚未对外发布。
    动机/场景：Daft / arrow / snap 等场景当前大量 hardcode asm，破坏 Rust 安全性，且不同版本 CPU 指令集差异会导致 core。
    后端：按 CPU 选择——Kunpeng 950 优先 SVE，920 用 NEON。

3. 标准库优化/使能（详见 rust-kunpeng.md）
    - std::arch SVE 支持：Rust 1.95 已新增 std::arch 的 ARM SVE 指令调用（合作方华为 2012 实验室开发）
    - 标准库性能：计划中，已做基础性能测试，尚未确定优化点（rustc 仓库自带 benchmark）
    - PyO3 性能：针对 PyO3 自身 benchmark，目标场景 Daft / Lance

    > 待确认（AI 提问第三组）：
    > 1.（Daft/效果）是否有具体性能数据（端到端吞吐提升百分比等）可写进文档？无则保持定性。
        当前尚未开始优化， 当前Daft 优化还是embed asm 形式， 我们的目标是替换这些 asm
    > 2.（Portable SIMD）"snap" 具体指哪个项目/库？
        Daft 依赖的 snap 压缩库
    > 3.（Portable SIMD）库有仓库/项目名了吗？
        还没有，内部就较 portable_simd
    > 4.（PyO3）目前是「已做优化」还是「已测基线、准备优化」？优化方向（GIL/类型转换/序列化）定了吗？
        规划中， 没有做基线测试。 

## 当前 沙箱（LLM 安全） 做了哪些？

1. Sandbox 是纯 SaaS 场景还是Client 端也有？
    deepseek harness 目前也有 sandbox 的规划，但是没有实现。

2. 使用 AgentEnv / CubeSandBox 的主要场景是什么样的
    LLM + Agent + Sandbox 里的客户端吗？

## rust 语言生态组 有哪些可以做的事情

1. 主要目标是什么？
    跑起来 / 并发量 / start pause resume 时间

2. 有哪些穿刺方向的建议


