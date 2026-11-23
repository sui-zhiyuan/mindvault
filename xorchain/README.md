# xorchain

流式、可逆的周期密钥前缀 XOR 编解码工具。

## 格式

输出头部为：

```text
n (u32 小端) | K0 K1 ... K(n-1) | 载荷
```

- `n`：密钥周期（字节数，即「xor 程长」），编码时可用 `-n` 指定，解码时从文件头读出。
- `K0..K(n-1)`：每次编码新生成的 `n` 字节随机密钥。
- 载荷：输入按 `n` 字节一组切开，每组第 `j` 个字节落在第 `j` 列；每列单独做前缀 XOR，再与对应的密钥字节 `K(j)` 异或。

设输入字节为 `S0 S1 S2 ...`，输出载荷字节为 `C0 C1 C2 ...`：

```text
Ci = K(i mod n) xor Si xor S(i-n) xor S(i-2n) xor ...
```

即同列（间隔 `n` 字节）的字节按出现顺序依次异或，不同列互不影响。以 `n = 3` 为例，前两组载荷为：

```text
第一组: K0 xor S0,            K1 xor S1,            K2 xor S2
第二组: K0 xor S0 xor S3,     K1 xor S1 xor S4,     K2 xor S2 xor S5
```

- 输出长度恒等于 `4 + n + 输入长度`。
- 空输入编码后只有头部（`4 + n` 字节）。
- 最后一块按实际长度输出，不填充；不完整的组只影响缺列。
- `n = 1` 时退化为对整条流的前缀 XOR。

解码：

```text
Pi = Ci xor K(i mod n)               # 第 i 列的截至当前字节的前缀 XOR
S0 = P0, Si = Pi xor P(i-n)          # 跨组取同列上一个前缀，首字节按 0
```

解码只需 `n` 字节的列状态，仍是流式、O(n) 内存。单字节载荷被破坏只会影响该字节及其后同一列的下一个字节。

## 用法

```bash
# 文件
xorchain encode -i input.bin -o output.bin
xorchain decode -i output.bin -o restored.bin

# 指定周期（例如 4099 以外的质数）
xorchain encode -i input.bin -o output.bin -n 1031

# stdin / stdout
cat input.bin | xorchain encode > output.bin
xorchain decode < output.bin > restored.bin
```

`-i` 和 `-o` 的默认值都是 `-`，表示标准输入 / 标准输出。

`-n` 默认 4099（质数）。选质数是为了让分组边界不与输入里常见的 2 的幂记录长度对齐；上限为 1048576 字节。

## 与旧格式的兼容性

0.2.0 起文件头从「8 字节 R0」改为「`u32 n` + `n` 字节密钥」，旧版本生成的
文件无法被新版本解码，反之亦然。需要保留旧数据时请先用 0.1.x 解码。

## 构建与测试

```bash
cargo build --release
cargo test
```

## 安全说明

这不是加密，不提供保密性，也没有完整性校验。密钥以明文存放在输出里，编码是线性的可逆运算，任何人都能还原原始内容。需要保密时请使用 AES-256-GCM 或 ChaCha20-Poly1305 之类的认证加密。
