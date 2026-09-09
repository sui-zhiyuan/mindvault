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

1. 针对 Daft 的性能优化，主要在 sve ， 调度， 编译条件等.
    场景（数据工程二期）
    Daft 是 xxxx
    我们做了 1234

    > 待确认（AI 提问第一组）：
    OverAll 这个文档是一个交流的提前准备，分为三部分 我们（rust 语言生态组）当前做了什么， 我们希望知道对方（LLM 安全组）哪些信息，最终目标 （我们能帮助 沙箱组做什么）。 必须验证所有内容是有助于双方找到合作点的。 

    > 1. 「数据工程二期」场景具体是什么？负载类型/数据规模/内部平台 or 对外产品？
        内部项目， 主要针对 字节 LAS 场景，优化其 Daft 任务在 Kunpeng 上的表现。（LAS 是公开项目，可以搜索其内容）
    > 2. Daft 定位？分布式 DataFrame / Arrow 生态？替代 Spark/Dask/Polars？
        Daft 是数据处理框架 客户（字节）选型，具体作用搜索 LAS 获取。
    > 3. SVE 优化了哪些热点（arrow compute kernel / 字符串 / hash / 拷贝）？为何是瓶颈？
        先读取 /home/suine/projects/mindvault/docs/rust-kunpeng/daft-kunpeng-overall-design.md ， 然后再继续询问我。 
    > 4. 调度改了什么（线程池/rayon/tokio / NUMA 亲和 / 任务窃取）？是否针对多核多 NUMA？
        目前调度不涉及 NUMA ，主要是 Ray 模式下运行情况。
    > 5. 编译条件具体指什么（target-cpu / SVE feature / 交叉编译 / cfg 分支）？
        当前 kunpeng 支持的指令可可以执行的 llvm 优化 path 在 llvm 上没有配置， 因此不能指定  target-cpu = native ，我们测试了一部分kunpeng 相关优化点 在编译daft 的情况下的优化度， 大概 3~5%。 不同场景偏差较大。 
    > 6. 效果/指标：提升多少、什么 benchmark、与 x86 对比？
        针对 LAS 给出的场景，测试端到端吞吐速度。
    > 7. 状态与交付：完成/进行中/合入上游？仓库或 PR 链接？
        合入 Xuanwu社区， （这是一个公开社区 地址 https://xuanwu.openatom.org/about/）

2. Protable Simd 性能库
    我们规划了xxxx
    打算做xxx
3. 标准库优化/使能
    提供 std::arch 的 sve 支持
    优化 标准库性能（rustc 仓库自带 benchmark）
    优化 pyo3 性能

## 当前 沙箱（LLM 安全） 做了哪些？

1. Sandbox 是纯 SaaS 场景还是Client 端也有？
    deepseek harness 目前也有 sandbox 的规划，但是没有实现。

2. 使用 AgentEnv / CubeSandBox 的主要场景是什么样的
    LLM + Agent + Sandbox 里的客户端吗？

## rust 语言生态组 有哪些可以做的事情

1. 主要目标是什么？
    跑起来 / 并发量 / start pause resume 时间

2. 有哪些穿刺方向的建议


 