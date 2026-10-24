# 复现命令（2026-10-24 基线）

## ARM（192.168.41.50，固定 2.9 GHz）

```bash
scp main.rs freq_chain.c 192.168.41.50:~/hashmap-repro/
ssh 192.168.41.50

cd ~/hashmap-repro
gcc -O2 freq_chain.c -o fc
./fc                                  # mul/chase 标定；ARM 侧以 perf 为准，此行仅交叉检查
rustup run 1.98.0 rustc -O main.rs -o hr5

for h in std mix identity; do
  taskset -c 240 ./hr5 --n 78960 --reps 15 --warmup 3 --hasher $h > arm_$h.json
done

for h in std mix; do
  taskset -c 240 perf stat -e cycles,instructions,branch-misses,L1-dcache-load-misses \
    -- taskset -c 240 ./hr5 --n 78960 --reps 10 --warmup 3 --hasher $h >/dev/null 2>p_$h.txt
done
```

## x86（WSL2，i7-12700）

```bash
gcc -O2 freq_chain.c -o /tmp/fc
for c in 0 2 4 8 12 16 18 19; do taskset -c $c /tmp/fc; done   # 逐 CPU 标定

rustc -O main.rs -o /tmp/hr
for h in std mix identity; do
  for r in 1 2; do
    taskset -c 2 /tmp/hr --n 78960 --reps 20 --warmup 3 --hasher $h > ${h}_r${r}.json
  done
done
```

## 备注

- ARM 的 `perf stat` 区域包含进程启动与 drop；阶段比值请用 JSON 里的 `build_ns/query_ns/drop_ns`。
- `--reserve` 现在是独立 flag，可放在末尾；`--warmup` 已真正执行。
- `identity` 的桶布局退化（hash=key → 连续 ctrl），不能用来代表“随机键”的真实探测成本；隔离 SipHash 用 `mix`。
- 真实工作量：78,960 次插入 + 约 51,200 次 get（25,600 tokens × 2），当前 `--stride 1` 是 78,960 次查询、`--stride 3` 是 26,320 次，均与真实模式不同。
