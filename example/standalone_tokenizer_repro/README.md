# 长文本 tokenizer CPU 性能差异：独立复现包

本包只使用公开 tokenizer 和确定性合成输入，不需要内部服务、业务仓库、账号或私有模型。依赖安装完成后，测试可离线运行。

## 问题现象

在两个测试环境中，对相同输入调用同一公开 fast tokenizer，aarch64 环境的耗时比 x86_64 环境高约 39%–40%。固定 Rayon 线程数为 1 或 8 后差异仍存在。

|配置|x86_64 p50（ms）|aarch64 p50（ms）|增加|
|---|---:|---:|---:|
|默认|27.66|38.60|39.6%|
|RAYON_NUM_THREADS=1|27.89|39.11|40.2%|
|RAYON_NUM_THREADS=8|27.62|38.45|39.2%|

每个环境每种配置在新进程中运行 3 轮，每轮 10 次预热、30 次正式测试；共 540 次正式调用。每种配置的 3 轮均观察到 aarch64 更慢。默认配置调用线程 CPU 时间中位数为 27.649 / 38.593 ms，接近 tokenizer wall time；executor 排队中位数为 0.044 / 0.032 ms。此例差异主要在调用线程执行阶段。

这些是两个具体环境的历史结果，不是所有 x86/ARM CPU 的结论，也未证明是 ISA 问题。Python 小版本不同（3.11.2 / 3.11.9）；CPU 型号、频率、编译选项等未统一。调用线程 CPU 时间不包含其他 native 工作线程。接收方需要在自己的环境重新测量。

## 复现对象

- 公开 tokenizer：Qwen/Qwen2.5-0.5B。
- 固定 revision：060db6499f32faf8b98477b0a26969ef7d8b9987。
- 公开来源：https://huggingface.co/Qwen/Qwen2.5-0.5B/tree/060db6499f32faf8b98477b0a26969ef7d8b9987
- Apache-2.0 许可证随包提供；没有模型权重。
- transformers==4.48.3、tokenizers==0.21.4。
- 输入为 batch=1、58,834 字符、78,960 UTF-8 字节、25,600 tokens。
- 固定种子生成通用中文、英文、数字和 JSON，不读取业务语料。
- 使用 8-worker ThreadPoolExecutor，通过 asyncio.run_in_executor 串行提交；没有人为并发压力。
- 每次调用都校验完整 token IDs，校验在计时区间之外。

## 快速运行

推荐 Python 3.11。在解压目录执行：

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install -r requirements.txt
sha256sum -c SHA256SUMS

# 快速检查三个独立进程、输入一致性及结果输出
python run_variants.py --rounds 1 --warmup 1 --samples 3

# 正式测试：三种线程配置，各 3 轮
python run_variants.py --output-dir ./output_full
```

安装 Python 依赖需要可用的软件源或预先准备的 wheel；包内不包含 Python 解释器或依赖 wheel。tokenizer 和输入已经随包提供，测试本身强制离线加载。无需下载模型权重。

单配置运行：

```bash
RAYON_NUM_THREADS=1 python test_tokenizer.py \
  --tokenizer ./tokenizer --fixture ./fixture.json \
  --output ./single_result.json --variant rayon1
```

默认组须取消 RAYON_NUM_THREADS 和 TOKENIZERS_PARALLELISM；run_variants.py 已处理。不要在同一进程中切换 Rayon 配置。重复运行会覆盖指定输出目录中的同名结果。

## 输出与分析

每个配置输出 JSON 和日志，complete.json 中三个退出码均为 0 才算成功。JSON 包含软件版本、架构、输入 shape/hash、tokenizer 文件 hash、逐次样本及分轮统计。

- tokenizer_ms：tokenizer 调用的 wall time。
- executor_thread_cpu_ms：执行调用的线程 CPU 时间。
- queue_ms：提交到 executor 后，执行函数开始前的时间。
- total_ms：从提交到结果返回主事件循环的时间。

历史结果在 results/，新的测试不会覆盖它们。比较双方相同配置的 p50/p95 和逐轮结果；先核对 shape、token IDs hash、文件 hash 和依赖版本一致。建议统一 Python 小版本并记录 CPU 型号、频率、构建来源和负载，再定位 tokenizers 内部 native 热点。

## 文件说明与脱敏范围

- test_tokenizer.py：独立功能测试和计时实现。
- run_variants.py：三种 Rayon 配置的新进程运行入口；任何失败返回非零退出码。
- make_fixture.py：合成输入生成器，可重新生成 fixture.json。
- tokenizer/：公开上游 tokenizer 资产和许可证。
- source.json：公开资产 revision 与 SHA256。
- results/：去除 CPU affinity 的历史性能样本。
- SHA256SUMS：文件完整性校验。

不包含真实请求、私有 tokenizer、业务代码、服务名、主机地址、内部文件路径、个人账号或访问凭据。保留复现必需的通用配置、软件版本、架构与性能数据。压缩包统一清空属主名称、UID/GID 和时间戳；不包含虚拟环境、缓存或本机运行日志。
