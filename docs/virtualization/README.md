# 虚拟化

本主题记录虚拟化技术的系统性调研：它从软件绕行走到硬件补位的过程、硬件虚拟化到底依赖哪些硬件能力、
"多层虚拟化"的几种含义，以及各家厂商的产品形态与沙箱生态。

调研面向的实际问题是：**在 ARM / 鲲鹏平台上做虚拟化相关规划时，哪些能力是架构标配、哪些取决于芯片代次、
哪些根本没有硬件路径。** 因此本主题对"已证实 / 未证实"做了严格标注，不把架构版本反推当作事实陈述。

## 笔记

- [硬件虚拟化：发展过程、多层方案与硬件能力依赖](hardware-virtualization.md)
  — 虚拟化演进时间线、硬件虚拟化的可判据定义、软件虚拟化的分档性能损失、
  多级/嵌套虚拟化的方案族与代价、Intel / AMD / ARM / 鲲鹏 的能力矩阵，以及鲲鹏 920 无硬件嵌套路径的结论。

## 与其他笔记的关系

- `docs/rust-kunpeng/cubesandbox-rust-analysis.md`、`docs/rust-kunpeng/agentenv-cubesandbox-comparison.md`
  —— CubeSandbox / AgentENV 是"硬件辅助虚拟机级沙箱"的实例，是虚拟化能力的下游产品形态。
- `docs/arm-kunpeng/kunpeng-920b-specs.md`
  —— 其中记录的 920B 实测 flags（含 `sve`）与官方 920 的 ARMv8.2 规格存在一处待消解的矛盾，
  已在 `research/virtualization/04-hw-capabilities-arm-kunpeng.md` 的未决问题中记录。

## 中间调研稿

本次调研的六路分册与交叉验证记录保留在 `research/virtualization/`（不进入本书目录），
其中 `08-verification-notes.md` 记录了高风险结论的证据链与降级过程。
