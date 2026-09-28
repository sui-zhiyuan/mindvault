# 原版 tokenizer + 标准库 hashbrown 宽组 SVE

用户已批准：保持tokenizer与ahash源代码不变，改Rust1.98.1标准库内部hashbrown0.17.1，尽量发挥SVE。旧0.15.3实验保留。

## 实施顺序
1. 构建未修改的1.98.1定制标准库并以独立std::HashMap程序验证；编译器固定原发行版，stdlib内部bootstrap构建配置显式记录。
2. hashbrown0.17.1先写宽组/表布局差分测试，NEON8基线先验证，再实现NEON16、SVE16、SVE32。
3. SVE热路径融合load/tag/empty扫描，优先紧凑predicate mask；必要时比较无store-forwarding的稀疏mask。逻辑WIDTH固定16/32，VL不能改变已分配表布局。
4. 小表、零大小类型、删除、rehash、迭代与页边界全部过关后，跑原有微测及高占用率/较大工作集补充。
5. 用干净的tokenizers0.21.4源码，通过各自定制sysroot重新构建wheel；验证源文件未改、标准库路径命中、完整token IDs。
6. 先A/A，再10对独立进程端到端配对；保留原附件warmup、samples、Rayon配置。记录时间、CI、探测/内存变化及退化。

## 约束
- 功能改动仅在hashbrown；标准库和项目的构建配置可变化，tokenizer/ahash代码保持上游原样。
- 各后端使用相同Rust1.98.1/LLVM22.1.8、相同优化设置、相同依赖锁。
- SVE32必须正确处理或显式约束VL>=32；不允许按线程动态改变表宽。
- 不以旧轮应用侧容器接入数据作为新轮NEON基线。
- 新产物与旧实验分开：本地wide/，远端/root/hashbrown-wide-20260924。

## 状态
- 标准库基线构建：完成。
- 宽组实现：完成，独立审查通过。
- 性能实验：完成，微测和 15120 次端到端正式调用校验通过，见 wide_sve_results.md。
