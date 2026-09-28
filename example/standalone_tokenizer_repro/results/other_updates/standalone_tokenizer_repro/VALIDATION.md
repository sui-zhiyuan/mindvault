# 打包验证

独立虚拟环境中安装 requirements.txt，使用 Python 3.12.8、transformers 4.48.3、tokenizers 0.21.4，对三种 Rayon 配置各执行 1 轮、1 次预热、3 次正式调用。三组功能测试、逐次 token IDs 校验与运行入口均通过，退出码均为 0。

这是运行入口与功能的 smoke test，不用于替换 README 中两个原始环境的性能结果。原始性能环境分别是 Python 3.11.2 / 3.11.9。正式跨环境对比建议统一 Python 版本。

包内文件经过内部标识检查；公开 tokenizer 文件与 source.json 中的上游文件 hash 一致。归档不包含符号链接、绝对成员路径、CPU affinity、虚拟环境或本机 smoke test 输出。归档属主、组名、UID/GID、mtime 以及 gzip mtime 均已规范化。
