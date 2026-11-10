# xorchain

流式、可逆的前缀 XOR 编解码工具。

## 格式

输入按最多 8 字节分块，记为 `S0 ... Sn`：

```text
输出 = R0 | (R0 xor S0) | (R0 xor S0 xor S1) | ...
```

- `R0` 是每次编码新生成的 8 字节随机值。
- 最后一块按实际长度输出，不填充。
- 输出长度恒等于 `8 + 输入长度`。
- 空输入编码后只有 8 字节的 `R0`。

解码：

```text
S0 = C0 xor R0
S1 = C1 xor C0
S2 = C2 xor C1
...
```

其中 `C0` 是 `R0` 之后的第一个输出块。

## 用法

```bash
# 文件
xorchain encode -i input.bin -o output.bin
xorchain decode -i output.bin -o restored.bin

# stdin / stdout
cat input.bin | xorchain encode > output.bin
xorchain decode < output.bin > restored.bin
```

`-i` 和 `-o` 的默认值都是 `-`，表示标准输入 / 标准输出。

## 构建与测试

```bash
cargo build --release
cargo test
```

## 安全说明

这不是加密。`R0` 明文存放在输出里，而且整个变换是线性的，任何人都能还原原始内容。它适合做格式变换或简单混淆，不提供保密性。如果需要保密，请使用 AES-256-GCM 或 ChaCha20-Poly1305 之类的认证加密。
