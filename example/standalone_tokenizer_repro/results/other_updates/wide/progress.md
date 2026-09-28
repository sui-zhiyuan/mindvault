# 第二轮：标准库内部宽组 SVE

- 用户批准只改hashbrown功能，原版tokenizers/ahash源码不变；Rust1.98.1 std内部hashbrown0.17.1。
- 远端/root/hashbrown-wide-20260924，旧轮/root/hashbrown-sve-20260924保留（含免密、Rust1.98.1、Cargo离线缓存、Python3.12.14依赖和原fixture）。
- stdlib构建：scripts/build_sysroots.sh 使用原发行rustc1.98.1+RUSTC_BOOTSTRAP=1编译matching rust-src/sysroot的dist profile，以构建配置patch hashbrown path。应用构建unset bootstrap，--target aarch64 --sysroot选择新std。所有程序exec前prctl VL32+INHERIT。
- 干净tokenizers来自v0.21.4原commit归档，check_clean_sources.py验证344文件原样，仅更新bindings/python/Cargo.lock。
- 后端已实现并审查：neon8基线、neon16 nibble、sve16/sve32 compact+fused lookup；sve16r/32r寄存器nibble；compact可选hb_fused_insert融合两插入路径+brkb/cntp；可选hb_private_scratch私有栈保存predicate以合法使用pure/readonly。
- 各新增selector均先红后绿。全库：最新NEON8=107、NEON16=112、SVE16/32-fused=113、private32-fused=114全部通过；控制组含快照/页边界/全tag/mask测试。
- std::HashSet<u8>capacity(1)在各sysroot为7/14/28，独立std::HashMap程序通过，证明新布局进入标准库。
- stock custom sysroot的原版tokenizer wheel可构建/导入，原附件3种Rayon smoke全过。
- pilot legacy小表SVE普遍变慢；高占用率85% miss，SVE32耗时降低约33–60%。寄存器32r更慢，被淘汰。
- 稳态perf（200次warm编码，setup禁用采样）发现reserve_rehash约6.1%、HashMap<usize,usize,stdRandomState>::insert约5.16%，主要来自BytesToCharOffsetConverter逐UTF8字节建表。因此增加insert/grow/实际offset collect微测与融合插入。
- private栈方案合法性经官方Rust readonly私有栈例外与独立review确认；每条路径SP恢复，无nostack。仅受控AArch64Linux/VL32实验。
- perf工具ACK含NUL，已复现'\x00ack\n'并修正帧分隔解析。profile-stock-03成功，原版代码未动。
- 当前已冻结最后候选，不再新增优化：最终对照neon8、neon16、sve16-fused、sve32-fused、sve32-fused-private。private略改善insert但略恶化lookup，需端到端决定。
- 正在从同一最终源码重建全部5种sysroot/micro/wheel。assemble_sysroot读取Cargo artifact JSON，旧sysroot库目录保留为lib.previous-*，避免混入陈旧rlib。

## 剩余
1. 各最终wheel创建venv并原附件smoke；冻结源码/版本/stdlib/扩展hash证据。
2. 原有lookup、扩展、insert各10轮5变体微测（轮换顺序，CPU240/NUMA3）。
3. 原附件端到端5变体×10实验块，顺序平衡，CPU240,242,...254/NUMA3；A/A校准，保留3x10warmup+30samples和完整IDs。
4. 逐JSON重算统计，取回结果与源码，报告收益/退化和范围；不沿用第一轮应用侧接入基线。


## 宽组实验已完成（2026-09-24）
仅修改 std 内部 hashbrown 0.17.1，固定 Rust 1.98.1；NEON8/NEON16/SVE16-fused/SVE32-fused/SVE32-fused-private 的微测、库测试、原版 tokenizer 端到端及独立审查完成。15120 次正式调用通过校验，本地重算全部配对统计及区间一致。高占用率 miss 微测最多减少 64.75% 耗时，最终 SVE 端到端耗时增加约 2.4%～5.1%，建议当前负载保留 NEON8。完整报告：wide/wide_sve_results.md。
