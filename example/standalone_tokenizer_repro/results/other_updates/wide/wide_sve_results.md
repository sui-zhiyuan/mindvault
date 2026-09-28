# hashbrown 宽组 SVE 实测结果

2026-09-24，192.168.41.51。结论：**此 tokenizer 负载保留 NEON8。** 在保持 tokenizer、ahash 源码不变的条件下，SVE16/32 可以显著改善高占用率未命中微测试，但最终端到端耗时增加约 2.4%～5.1%，没有获得整体加速。

## 修改范围与环境

- Rust 1.98.1，rustc `48a229ceaefd4985c50990b14116b6d856af0985`，LLVM 22.1.8；标准库 hashbrown 0.17.1，基于 `c62a63a61b7caf2de8f9ecb7b06a66b0ab6bdf3d`。
- tokenizers 0.21.4（`e892882fd4608b468dcf9dc33ea95283882b8e6d`）、ahash 0.8.12、transformers 4.48.3、Python 3.12.14。
- TaiShan-v120 / Kunpeng 920 7280Z，AlmaLinux 9.4，Linux 6.6.0，SVE VL=32 字节；performance governor，boost 关闭。
- 功能修改仅在 hashbrown。344 个 tokenizer 源文件核对未变，仅更新构建锁文件；1128 个标准库相关 Rust 源文件核对未变。通过定制 sysroot 重编译原版 tokenizer wheel，使 `std::HashMap` 使用修改后的 hashbrown。
- 构建标准库使用同一发行版编译器及 `RUSTC_BOOTSTRAP=1`；应用构建取消 bootstrap，显式指定目标与 sysroot。这是实验构建方式，不是未修改的官方预编译标准库。
- 本轮与此前 hashbrown 0.15.3 应用侧接入实验分开，不能混用两轮基线。

## 尽量发挥 SVE 的实现

比较 NEON8、NEON16、SVE16、SVE32，并继续探索融合扫描、寄存器掩码、私有栈 scratch。最终五组是 NEON8、NEON16、SVE16-fused、SVE32-fused、SVE32-fused-private。

SVE 同时比较 16/32 个控制字节；融合 load、tag 匹配、EMPTY 检查，并融合插入时的首个 EMPTY/DELETED 定位。无 tag 候选时跳过掩码落栈。private 版本将 scratch 限制在汇编私有栈，改善编译器对外部内存副作用的处理。另测了寄存器稀疏掩码方案，32 字节方案表现较差，未进入最终端到端候选。

因此“不止 8 字节组宽”可以只改 hashbrown 实现，但不能只替换 load：还需同步修改组掩码、探测、尾部控制字节及小表容量约束。本轮补齐 32 字节组宽的小表布局。逻辑宽度固定，不随线程 VL 改变；SVE32 限定 VL≥32，测试在进程执行前设置并核对 VL=32，不能直接用于 VL=16 的机器。

## 先微测试

每类 10 轮，五个后端交错顺序，CPU 240、NUMA 3。微测直接使用各 sysroot 的 `std::HashMap`。

| 测试 | SVE16-fused 相对 NEON8 | SVE32-fused 相对 NEON8 | SVE32-fused-private 相对 NEON8 |
|---|---:|---:|---:|
| 原有 12 个 lookup / lookup_fail 用例 | 耗时增加 11.48%～37.97% | 增加 10.98%～72.06% | 增加 12.75%～77.42% |
| 85% 占用、8192 桶、未命中 | 耗时减少 34.10% | 减少 63.70% | 减少 64.75% |
| 85% 占用、65536 桶、未命中 | 减少 22.20% | 减少 39.29% | 减少 39.03% |
| 85% 占用、524288 桶、未命中 | 减少 17.76% | 减少 34.25% | 减少 32.88% |
| 附件文本 byte-offset collect，固定 hasher | 增加 6.05% | 减少 0.96% | 减少 2.00% |
| 同上，随机 std hasher | 增加 5.70% | 减少 0.80% | 减少 1.41% |

补充微测共 18 个 lookup 场景，覆盖不同工作集、50%/85% 占用率、命中/未命中、删除残留与字符串；另有 8 个插入场景。宽组减少高占用率 miss 的探测次数，但其他场景存在退化。**约 65% 是特定微测的耗时下降，不是 tokenizer 加速比例，也不等于吞吐提高 65%。**

## 再端到端测试

沿用附件固定输入：batch=1，58834 字符、78960 UTF-8 字节，输出 25600 tokens。CPU 固定为 240,242,244,246,248,250,252,254，NUMA 3。三种配置为附件 default、rayon1、rayon8。

正式比较采用 10 个 block × 5 后端，轮转及反转运行顺序，串行执行；每次进程沿用 3 轮 ×（10 warmup + 30 samples）。共 13500 次正式调用。额外 NEON8 A/A 三个 block 共 1620 次，总计 **15120 次正式调用**，不含预热。

统计以独立进程 p50 为配对单位，配对对数比值取几何平均，10000 次 bootstrap，固定随机种子 20260924。下表是 **tokenizer 耗时增加百分比及 95% CI；正数表示变慢**。

| 后端 | default | rayon1 | rayon8 |
|---|---:|---:|---:|
| NEON16 | +1.46% [0.64, 2.20] | +2.27% [1.03, 3.56] | +0.65% [-0.68, 1.93] |
| SVE16-fused | +2.54% [1.35, 3.81] | +3.81% [2.92, 4.58] | +2.53% [1.29, 3.79] |
| SVE32-fused | +4.92% [4.23, 5.62] | +5.10% [3.88, 6.42] | +4.20% [2.66, 5.79] |
| SVE32-fused-private | +4.03% [3.11, 4.96] | +3.43% [2.52, 4.49] | +2.40% [1.46, 3.36] |

NEON8 进程 p50 的中位数分别为 38.3517、38.2323、38.4625 ms。表内比例是配对估计，不能用汇总中位数相除替代。

同为 16 字节组宽时，SVE16-fused 相对 NEON16 耗时增加：default +1.07% [-0.40, 2.58]，rayon1 +1.50% [0.51, 2.54]，rayon8 +1.86% [0.71, 3.02]。没有证据支持同宽 SVE 整体优于 NEON。

A/A 耗时变化为 +0.16% [-0.37, 1.21]、-0.84% [-2.91, 0.36]、-2.57% [-4.26, 0.23]。仅三对，用于揭示噪声，不将微小差异当作可靠收益。

## 正确性与归因

- 最终五个后端库测试分别通过 107、112、113、113、114 项；涵盖小表、零大小类型、删除再插入、rehash、控制掩码、非对齐及页边界。
- std probe 验证 HashSet<u8> 小表容量分别为 7/14/28，确认标准库布局切换生效；校验安装的扩展 `.so` 与对应 wheel 相同。
- 15120 次正式调用输出 IDs 校验通过；另外对原版 stock 和五个候选比较完整 Encoding，包括 tokens、offsets、word_ids、sequence_ids、attention/type/special-token masks，覆盖附件文本、空文本、ASCII、混合 Unicode，六组摘要一致。
- 本地重新读取全部原始 E2E 数据，重新计算 p50/p95、配对比例及 bootstrap 区间，与远端结果完全一致，记录于 `results/local-audit.json`。

对原版 std/tokenizer 的预热后 perf 采样显示，malloc/free、正则、offset 转换也占明显成本；hashbrown reserve_rehash 自身约 6.10%，usize offset map insert 约 5.16%，std RandomState hash_one 约 4.78%。这说明整体负载并非主要由高占用率 lookup miss 构成。宽组 miss 微测的收益不能直接推算到端到端。采样用于归因提示，不等同于精确可优化时间比例。

## 复跑方法与交付范围

服务器保留 `/root/hashbrown-wide-20260924` 的源码、五套 sysroot、wheel、venv、二进制和原始结果。共享工具链及附件位于 `/root/hashbrown-sve-20260924`。以下命令针对已配置好的该服务器；输出目录必须是新目录。

```bash
ssh 192.168.41.51
cd /root/hashbrown-wide-20260924
# 先微测；二进制和 sysroot 已保留
for mode in legacy extended insert; do
  python3 scripts/run_micro.py --rounds 10 --mode "$mode" \
    --variants neon8 neon16 sve16-fused sve32-fused sve32-fused-private \
    --output "results/retest-$mode"
  python3 scripts/summarize_micro.py "results/retest-$mode" neon8
done
# 再端到端，含 A/A
python3 scripts/run_e2e.py --aa --rounds 3 --output results/retest-aa
python3 scripts/summarize_e2e.py results/retest-aa neon8
python3 scripts/run_e2e.py --rounds 10 --output results/retest-e2e
python3 scripts/summarize_e2e.py results/retest-e2e neon8
python3 scripts/summarize_e2e.py results/retest-e2e neon16
```

`scripts/build_sysroots.sh`、`build_clean_wheel.sh`、`build_micro.sh` 保存构建方法，`results/build/` 保存来源、锁文件和校验信息。交付包包含修改后的 hashbrown 源码、测试脚本、原始结果、附件输入、原版 tokenizer 源码归档及共享统计/VL 工具；不包含完整 Rust SDK、sysroot、wheel、venv 和依赖缓存。因此它是源码与证据归档，换机重建需按脚本准备依赖与目录，不能视为任意机器解压即跑的离线安装包。

本结论限定于该 CPU、VL32、当前编译设置及附件预热负载。建议当前 tokenizer 保留 NEON8；SVE 宽组可作为高占用率未命中场景的实验实现保留。
