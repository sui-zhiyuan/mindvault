#上海/杭州/东莞# 【软硬结合】【计算】 软件使能硬件，充分发挥CPU/NPU/DPU算力

地点： #上海 #杭州 #东莞 #均有岗

我们部门专注于计算基础设施的核心计算、网络与智能加速，通过极致软件优化来充分发挥硬件潜能，
用算法和数学创新来释放每一瓦的性能。覆盖 CPU、DPU、NPU 三大技术方向，全栈攻坚，决胜下一代AI基础设施。
无论你是系统软件高手、编译器专家、AI框架开发者，还是底层性能优化发烧友，这里都有你的舞台。

## CPU方向——系统性能的极致打磨

聚焦 Python / Rust 双语言生态的性能优化与运行时加速，面向鲲鹏处理器深度挖掘硬件性能潜力，
结合其鲲鹏特有指令集扩展、向量化计算、原子操作、预取机制、Cache 层次及 NUMA 等硬件特性，
开展 CPython JIT、Free-Threading、NumPy / Pandas / SciPy 等数据科学栈的 ARM 架构调优，
并覆盖 Linux Kernel、GCC / LLVM 编译器、内存管理与系统调度等底层系统优化。

同时深耕 AI 编译与前端优化，包括 PyTorch 前端的图捕获、图优化与算子下发，
以及 Triton-CPU 的适配与增强，让 AI 模型在鲲鹏上运行的效率更高，从框架层面实现昇腾与鲲鹏的协同。

目标：让数据工程与 AI 推理训练在鲲鹏上跑出极致性能，全面对标并超越友商。

领域开源代码仓：
cinderx jit加速：https://gitcode.com/openeuler/cinderx
torch npu加速：https://gitcode.com/Ascend/pytorch
numpy：https://gitcode.com/boostkit/numpy
pandas：https://gitcode.com/boostkit/pandas
daft: https://gitcode.com/xuanwu/Daft
snap: https://gitcode.com/xuanwu/snap
rust-bench：https://gitcode.com/xuanwu/rust-bench
triton-cpu: https://atomgit.com/openeuler/triton-cpu


## 我们的优势
从芯片到业务的全栈视野，真正动手改 Kernel、Compiler、Runtime
面向互联网 TOP 客户的真实场景
Python / Rust 双语言生态深度自研（华为 Python / Rust 开源社区核心贡献）
上海、杭州、东莞多地可选

## 我们需要这样的你
扎实的系统软件功底（OS / Compiler / 架构 / 网络存储协议）
对性能优化有执念，习惯用数据说话
有 Rust / C / C++ / Python 底层开发经验
了解 AI 框架或 RDMA 网络加分

-----------------------------------------------

下面是同一个部门其他组的。

## DPU方向——可编程智能网卡与数据中心加速

基于华为 FlexDA DPU/NIC 开放可编程框架，面向数据中心网络、存储与数据加速场景，
打造 OVS、RoCE、DTOE 等高性能网络与存储软件栈。依托 L2/L3 层开放编程能力与 DPU 硬件卸载能力，
支持自定义报文解析、流表 Key/Action、自定义 Pipeline、RDMA 拥塞控制算法以及存储协议卸载等能力，持续释放 DPU 硬件算力。

FlexDA已面向所有开发者发布，欢迎了解：
https://www.hikunpeng.com/cn/dakit/flexda


## NPU方向——AI芯片驱动与异构计算栈

构建昇腾 NPU 从 Driver / HAL 到 Runtime 的全栈软件底座，打通 Host 与 Device 之间的关键链路。
深入 PCIe / UB 高速互联、Host-Device 通信、RoCE 网络、DVPP 视频编解码、统一虚拟内存与设备虚拟化、
任务调度及 AI CPU 微内核（LiteOS / EulerOS） 等核心技术，向上提供 Device、Stream、Event、Memory、Kernel 
等高性能 Runtime 能力。

领域开源代码仓：
CANN driver：https://gitcode.com/cann/driver
CANN runtime：https://gitcode.com/cann/runtime