# sve-wide — 标准库 hashbrown 宽组 SVE 实验

> 2026-09-24｜ARM-A（鲲鹏 920B / TaiShan V120，SVE VL=32B）｜主报告：[`wide_sve_results.md`](wide_sve_results.md)
> 一句话：把 std 内 hashbrown 的控制字节扫描从 NEON8 加宽到 NEON16/SVE16/SVE32，**端到端变慢 +2.4%~+5.1%**，本负载应保留 NEON8。

## 目的

`ipc-rootcause/` 与 perf 采样都把一部分成本指向 `std::HashMap` 的 hashbrown 建表/探测。本轮检验当时的假设：既然“ARM 慢”可能与组探测有关，那么把扫描宽度从 8 字节加宽到 16/32 字节，是否能在真实 tokenizer 负载上拿到收益？

约束是只改 **Rust 1.98.1 标准库内的 hashbrown 0.17.1**，tokenizer 与 ahash 源码保持上游原样；通过定制 sysroot 重建原版 tokenizer wheel，让 `std::HashMap` 用上修改后的 hashbrown。

## 结论

- 端到端五后端 × 10 block × 三种 Rayon 配置，共 **15120 次正式调用**，输出 IDs 全部校验通过。
- SVE16/SVE32 相对 NEON8：端到端耗时 **增加 2.4%~5.1%**（正数 = 变慢），没有整体加速。
- 高占用率未命中微测确实变快（最多 −64.75%），但**传不到端到端**：真实负载的瓶颈不是宽组探测，而且寻空槽本来就接近每项一个组，加宽没有多少额外探测可省。
- 建议当前负载保留 NEON8。`optimization/` 后来在另一台 ARM 上复现了同一结论（+2.2%~+3.3%，9/9 格显著）。
- 预热后 perf 采样：`reserve_rehash` 约 6.10%、offset 映射 `insert` 约 5.16%、`RandomState::hash_one` 约 4.78% —— 说明负载主体并不是高占用率 lookup miss。

## 目录

| 路径 | 内容 |
|---|---|
| [`wide_sve_results.md`](wide_sve_results.md) | 主报告：修改范围、五后端实现、微测、端到端、CI、复跑命令 |
| [`plan.md`](plan.md) / [`progress.md`](progress.md) | 实验计划与逐日过程记录（progress 含 perf 发现） |
| [`src/hashbrown/`](src/hashbrown) | 修改后的 hashbrown 0.17.1 源码（NEON16 / SVE16 / SVE32 / private 栈） |
| [`src/micro/`](src/micro) | 微基准 crate（lookup / 高占用率 miss / insert） |
| [`src/demangle/`](src/demangle) | 符号还原工具 |
| [`src/probe/`](src/probe) | 标准库布局探针（验证定制 std 真的生效） |
| [`scripts/`](scripts) | sysroot/wheel 构建、微测与端到端运行、配对统计（原 `wide/scripts/` 与 `shared/scripts/` 合并） |
| [`data/`](data) | 原始结果：`e2e-final/`、`e2e-aa/`、`micro-final-*/`、`profile/`、`build/`、`logs/`、`local-audit.json` |
| [`repro-on-50/`](repro-on-50) | 在 ARM-B 上的复现：`retest-{e2e,aa,legacy,extended,insert}/`、artifacts、日志 |
| [`sources/tokenizers-clean-v0.21.4.tar.gz`](sources) | 干净 tokenizers v0.21.4 源码归档（wheel 构建输入） |

## 怎么读

1. 先看主报告的“结论 / 端到端测试”两节，再看微测与端到端的背离。
2. 为什么微测收益传不到端到端：见 `progress.md` 的 perf 段与主报告“对原版 std/tokenizer 的预热后 perf 采样”。
3. 复跑需要 aarch64 + SVE VL=32 与定制 sysroot，步骤见主报告“复跑方法与交付范围”；`data/local-audit.json` 是本地重算统计与远端一致的核对记录。

## 注意

- 所有 wheel 都由定制 sysroot 产出（`RUSTC_BOOTSTRAP=1` 构建匹配 std，应用侧不启用 bootstrap），**不是官方预编译标准库**。
- SVE32 要求进程 VL≥32；实验在 exec 前固定 VL=32，不能直接用于 VL=16 的机器。
- 与更早的 hashbrown 0.15.3 应用侧接入实验不能混用基线。
- 微测里的百分比是特定微测的耗时变化，不等于 tokenizer 加速比例，也不等于吞吐提升。
