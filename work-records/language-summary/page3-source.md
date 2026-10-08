# 第 3 页原始内容（文字化）

> 来源：heavy/language_summary.pptx 的 Slide 3，用 pptx-read 的 --no-render 路径抽取（120,212 字节版本，2026-10-08 20:00）。
> 页面：960 × 540 pt。标题「以鲲鹏业务为核心，当前主要客户中 Rust 应用场景和亲和优化点」。

## 页面结构（自左向右、自上而下）

1. **「场景」列** —— 三个应用框架，各挂一个开源项目：Agent 沙箱 → **CubeSandbox**；LLM 数据底座 → **Daft**；K+X/K+A 推理 → **SGLang**
2. **「基础库」列** —— 10 个 Rust 基础库：arrow-rs、Containerd-*、rust-vmm、tokio、pyo3、serde、tracing、axum、reqwest、hashers
3. **硬件底座**：Kunpeng 920B / Kunpeng 950 / Kunpeng 950 V200
4. **右侧三个信息框**：Rust当前进展（552,301,314x29）、腾讯场景介绍（523,83,177x29）、字节场景介绍（723,84,184x29）——后两个是本次要写正文的目标

## 本次改版决定

「腾讯场景介绍」和「字节场景介绍」两个框占满页面右侧整栏（x 490 → 927 pt，宽 ≈ 437 pt；y 80 → 510 pt，高 ≈ 430 pt），上下排布、**每个约 437 × 200 pt**。原右侧的里程碑甘特（2026-09-30 / 2026-12-30 / 2027-3-30 那条时间轴）整体移到别的页。左侧场景表与基础库表不动。

排版参照：本页现有项目说明框为 125 × 65 pt、10 pt 字、约 55 字；本页正文字号 10–11 pt。换算后每个框 **350–450 字**为舒适区（10–12 pt）。

## Slide 3 抽取原文

```
## Slide 3

### 标题 1
以鲲鹏业务为核心，当前主要客户中 Rust 应用场景和亲和优化点

### 矩形 46
Agent 沙箱

### 矩形 47
LLM 数据底座

### 矩形 48
K+X/K+A 推理

### 矩形 80
应用框架

### 矩形 94
CubeSandbox

### 矩形 64
基础库

### 矩形 65
arrow-rs

### 矩形 66
Containerd-*

### 矩形 67
rust-vmm

### 矩形 68
tokio

### 矩形 71
pyo3

### 矩形 80
场景

### 矩形 94
Daft

### 矩形 94
sglang

### 矩形 94
CubeSandbox是开源智能体沙箱，以KVM微虚拟机隔离代码执行和浏览器自动化，支持快照及单机或集群部署。

### 矩形 94
Rust负责写数据面与虚拟化面：cube-api、cube-hypervisor、CubeShim；

### 矩形 94
SGLang是大模型与多模态推理服务框架，面向智能体和强化学习，支持高并发、缓存复用及OpenAI兼容接口。

### 矩形 94
SGLang的Rust用于模型网关、缓存树核心和、tokenizor和多模态预处理，还经PyO3接入Python调度进程。

### 矩形 94
Daft的Rust承担表达式、计划、本地执行和数据读写等核心引擎功能，并通过PyO3接入Python。

### 矩形 94
Daft是面向AI的多模态数据引擎，以Python接口处理图像、音视频与结构化数据

### 矩形 71
Kunpeng 920B / Kunpeng 950 / Kunpeng 950 V200

### 矩形 71
serde

### 矩形 71
tracing

### 矩形 71
axum

### 矩形 71
reqwest

### 矩形 71
hashers

### 矩形 42
Rust当前进展

### 矩形 42
腾讯场景介绍

### 矩形 42
字节场景介绍

### 圆角矩形 71
2026-09-30

### 圆角矩形 89
完成技术穿刺
提升达到 5

### 圆角矩形 98
2026-12-30

### 圆角矩形 98
2027-3-30

### 圆角矩形 89
实现 7 条 pipeline 端到端提升 5%

### 圆角矩形 90
实现插件式加载多模态算子库和算子下推到Lance

### 圆角矩形 89
CubeSandbox

### 圆角矩形 89
Daft

### 圆角矩形 89
sglang

### 矩形 35
进展页面单独放到每个项目中

> notes: tokio（+tokio-stream、tokio-util） --- 三场景 --- 异步运行时与任务调度、网络与 IO 驱动；三场景的服务、网关与数据读写全落在其上
> pyo3（+pyo3-async-runtimes、rust-numpy） --- SGLang、Daft --- Rust↔Python 绑定、原生扩展与异步桥接；Daft 全引擎 cdylib、SGLang 多处原生扩展
> arrow-rs 家族（arrow、arrow-array、arrow-buffer、arrow-ipc、arrow-schema、arrow-select、arrow-row、arrow-csv、arrow-json、parquet、arrow-flight） --- Daft --- 列式内存、数组与模式计算、Parquet/IPC 读写、shuffle 数据面；Daft 引擎地基
> rust-vmm 内存与 KVM 底座（vm-memory、vmm-sys-util、kvm-bindings、kvm-ioctls） --- CubeSandbox --- Guest 物理内存映射、KVM ioctl 封装、VMM 系统能力；微虚拟机隔离基础
> rust-vmm 设备与后端（virtio-queue、virtio-bindings、vhost、vhost-user-backend、vfio-ioctls） --- CubeSandbox --- virtio 队列与绑定、vhost/vhost-user 后端、设备直通；沙箱 virtiofs/blk/net IO 前提
> containerd shim 栈（containerd-shim、containerd-shim-protos、ttrpc） --- CubeSandbox --- Shim v2 协议契约与轻量 RPC；沙箱被容器编排调度的唯一入口
> seccompiler（rust-vmm/seccompiler） --- CubeSandbox --- syscall 过滤编译与加载；VMM 线程最小权限，安全沙箱靠它落地
> serde + serde_json（serde-rs/serde） --- 三场景 --- 序列化/反序列化框架与 JSON 编解码；协议、配置与消息的通用层
> tracing + tracing-subscriber（+opentelemetry、opentelemetry-otlp） --- 三场景 --- 结构化日志、Span 与订阅分发、OTLP 导出；三场景可观测性统一出口
> axum + tower/tower-http（+hyper） --- CubeSandbox、SGLang --- HTTP 路由、提取器与中间件栈；CubeAPI 与 SGLang server/renderer/gateway 服务面
> tonic + prost（+tonic-prost） --- SGLang、Daft --- gRPC 服务与客户端、Protobuf 代码生成；SGLang 进程内 pipeline、Daft shuffle 与 OTLP 通道
> SGLang 解析与协议栈（dynamo-parsers、dynamo-protocols、dynamo-renderer、dynamo-tokenizers、llm-tokenizer、reasoning-parser、tool-parser、openai-protocol、smg-*、wfaas、data-connector） --- SGLang --- 分词、推理与工具调用解析、OpenAI 协议与渲染；网关真正实现，本地仅薄 handler
> opendal（apache/opendal） --- Daft --- 多后端对象存储统一抽象与分层服务；daft-io 的读写通道
> reqwest（+reqwest-middleware、reqwest-retry） --- 三场景 --- 异步 HTTP 客户端、中间件与重试链；出向调用（CubeAPI rustls-tls、网关转发、daft-io）
> 哈希与 SIMD 编码族（sha2、blake3、xxhash-rust、simdutf8、memchr） --- SGLang、Daft --- 内容哈希、SIMD UTF-8 校验与字节查找；cache-aware 路由哈希与去重热路径

```
