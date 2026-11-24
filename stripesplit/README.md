# stripesplit

按字节交错把文件切成多个分片，并用 `index.toml` 校验合并的 Rust 工具。

## 分割规则

把文件分成 `n` 份时，第 `i` 份由原文件下标为 `i, i+n, i+2n, …` 的字节按顺序组成：

```text
文件:      s0 s1 s2 s3 s4 s5 s6 s7 s8 s9        (n = 3)
第 0 份:   s0    s3    s6    s9
第 1 份:      s1    s4    s7
第 2 份:         s2    s5    s8
```

- 每份大小最多相差 1 字节，合计恰好等于原文件大小。
- 默认每份目标 1 MiB，份数 = `ceil(原文件大小 / 1 MiB)`；也可用 `-s` 指定每份大小，或用 `-n` 直接指定份数。
- 输出目录里除了分片，还有一份 `index.toml`。

## index.toml

```toml
version = 1
tool = "stripesplit"
algorithm = "byte-interleave"
original_name = "big.iso"
original_size = 1000
original_sha256 = "57799de8…"      # 分割前整个文件的 SHA-256
part_count = 10

[[parts]]
name = "big.iso.part0000"
size = 100
sha256 = "18a4642f…"                # 该分片内容的 SHA-256

[[parts]]
name = "big.iso.part0001"
size = 100
sha256 = "8d471a97…"
```

索引自己也是校验对象：合并前会核对版本、算法、份数、各分片大小合计，
以及分片名是否安全（不允许带目录或 `..`）。

## 用法

```bash
# 每份约 1 MiB（默认）
stripesplit split big.iso -o parts

# 每份约 4 MiB
stripesplit split big.iso -o parts -s 4M

# 固定 8 份
stripesplit split big.iso -o parts -n 8

# 目录已有同名分片/索引时需加 -f 覆盖
stripesplit split big.iso -o parts -s 1M -f

# 按 index.toml 合并还原（合并时校验分片与原始 SHA-256）
stripesplit merge parts -o big.iso

# 只校验分片，不写输出
stripesplit verify parts
```

`-s` 接受 `1M`、`512K`、`1.5G`、`104857600` 这类写法，K/M/G/T 按 1024 进制；
`-s` 与 `-n` 二选一，都不给时按 1 MiB 算份数。

`merge` / `verify` 的输入既可以是含 `index.toml` 的目录，也可以直接给 `index.toml`
的路径。分片按索引里记录的名字读取，文件名本身不参与排序。

## 校验

`merge` 分三层校验，任何一层失败都会删掉本次写出的输出文件：

1. 索引自洽（版本、算法、份数、大小合计、文件名安全）；
2. 每个分片的大小与 SHA-256；
3. 还原结果的 SHA-256 与 `original_sha256`。

## 行为与限制

- 流式处理，内存约 70 MiB，与文件大小无关。
- 分割和合并都需要同时打开 `n` 个分片文件。按默认 1 MiB 目标，1 GiB 文件约
  1000 份；工具会先检查 `RLIMIT_NOFILE`，不够时直接报错并提示换更大的 `-s`
  或调高 `ulimit -n`，不会写到一半才发现。
- 空文件会生成 1 个 0 字节分片；份数多于文件字节数时，多出来的分片是空文件。
- 用 `-f` 覆盖旧结果时，索引不再引用的旧分片不会被删除（可在覆盖前手动清空目录）。
- 写索引用「临时文件 + 改名」，不会留下写了一半的 `index.toml`；分割中途失败会
  清理本次创建的分片。
- 这不是加密，也不是纠删码：分片就是原文按字节交错的结果，任何一片缺失或损坏
  都无法还原；单个分片离开其余分片也没有独立含义。
- 与 `scripts/split_file.py` 的区别：那个按「连续块」切分（可先压缩、用
  JSON manifest 记录），适合分卷压缩/搬运；本工具按「字节交错」切分，用 TOML
  索引记录，适合需要把同一份数据均匀铺到多个分片的场景。

## 构建与测试

```bash
cargo build --release
cargo test
```
