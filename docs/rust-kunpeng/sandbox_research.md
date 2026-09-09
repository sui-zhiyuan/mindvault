# LLM sandbox 分析

## 当前 rust 语言生态组 在 Kunpeng 上 我们做了哪些

1. 针对 Daft 的性能优化（sve / 调度 / 编译条件）
    场景：数据工程二期（内部项目），针对字节 LAS（AI 数据湖 = 湖存储 Lance + 湖计算 Daft）场景，
          优化 Daft 任务在 Kunpeng 上的表现。
    Daft 是字节/火山引擎 LAS 选型的分布式多模态数据处理引擎（Rust 实现，单机 + Ray 分布式）。
    覆盖 6 条产线：文本 / 视频 / 图片 / 音频 / PDF / 自动驾驶（详见 daft-kunpeng-overall-design.md）。
    我们做了：
    - SVE / 硬件指令加速：SVE cntp UTF-8 字符长度、SVE gather-load 批量取数、ARM CRC32C hash、text offset 快路径
    - 调度：Ray 模式下的 UDF 并发度自适应（不涉及 NUMA）
    - 编译条件：Kunpeng 指令对应 LLVM 优化 path 未配置、无法 target-cpu=native；实测部分 Kunpeng 优化点在编译 Daft 时约 3~5% 提升，场景差异较大
    实现形式：当前为 embed asm（内联汇编），计划替换为 portable SIMD 库（见条目 2），暂无端到端性能数据。
    状态：已合入 Xuanwu 社区（https://gitcode.com/xuanwu/Daft）

2. Portable SIMD 性能库（规划详见 rust-kunpeng.md）
    定位：计划开源，对标 Google Highway；解决 std::simd 无 stable 计划、且不能依赖 std::arch 的痛点。
    状态：已开始实施，尚未对外发布；内部项目名 portable_simd（尚无公开仓库）。
    动机/场景：Daft / arrow / snap（Daft 依赖的 snap 压缩库）等场景当前大量 hardcode asm，破坏 Rust 安全性，且不同版本 CPU 指令集差异会导致 core。
    后端：按 CPU 选择——Kunpeng 950 优先 SVE，920 用 NEON。

3. 标准库优化/使能（详见 rust-kunpeng.md）
    - std::arch SVE 支持：Rust 1.95 已新增 std::arch 的 ARM SVE 指令调用（合作方华为 2012 实验室开发）
    - 标准库性能：计划中，已做基础性能测试，尚未确定优化点（rustc 仓库自带 benchmark）
    - PyO3 性能：规划中，未做基线测试，目标场景 Daft / Lance

## 当前 沙箱（LLM 安全） 做了哪些？

### 定位与形态
- 沙箱是纯 SaaS（云端）还是也有客户端本地运行？deepseek harness 的 sandbox 规划属于哪种？
- 使用 AgentEnv / CubeSandBox 的主要场景是什么样的？（LLM + Agent + Sandbox 客户端？）

### 性能诉求（定位合作点）
- 核心性能痛点按优先级是哪些？（启动延迟 / 并发实例数 / pause-resume 时间 / 内存占用）
- 这些指标当前量级与目标值分别是多少？
- 沙箱代码里是否存在手工 asm / SIMD 优化？（与 portable SIMD 库要解决的痛点是否一致）
- 是否有 std / alloc / 同步原语等标准库热点需要优化？
- 网关层当前是否是瓶颈？ axum 是否需要优化？

### 当前是否有初步的性能数据
- Kunpeng 上表现如何， 和 Zen5 差距多少
- 是否有初步分析瓶颈点，有没有 cpu ， 内存带宽，io 的评估

### 用户使用场景
- 工作负载画像，访问频率， 沙箱内执行哪些东西，沙箱内是 CPU密集还是 存储密集
- 部署环境方案/集群间通信

### 目前初步想到的优化点， 有哪些尝试过， 有哪些可能有用？
- 核心组件 使用 Euler LLVM 编译
- 暂停场景是否考虑了内存压缩方案， 比如将内存压缩后，不落盘，加快 resume 效率（压缩可 sve 加速）
- 优化 pause / resume 中 大块 memcpy
- NUMA 亲和优化，（优化  overlaybd + ublk 中线程和 内存numa 一致性 ， NUMA-aware 的 warm-pool）
- 页大小（arm 支持不同页大小）
- 配合 Kunpeng CPU 的性能分析工具（有规划吗）

## rust 语言生态组 有哪些可以做的事情

1. 主要目标是什么？
    跑起来 / 并发量 / start pause resume 时间

2. 有哪些穿刺方向的建议

3. 项目/投资/POC 情况

4. 如何交付（最终交付物）

5. 腾讯 / Kimi 当前计划
