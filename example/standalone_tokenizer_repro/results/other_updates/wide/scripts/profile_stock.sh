#!/usr/bin/env bash
set -euo pipefail
W=/root/hashbrown-wide-20260924
P="$W/results/profile-stock-${1:-01}"
mkdir -p "$P"
mkfifo "$P/control" "$P/ack"
export RAYON_NUM_THREADS=1
perf record -D -1 --control="fifo:$P/control,$P/ack" -e cycles:u -F 997 -g --call-graph fp -o "$P/perf.data" -- \
  python3 /root/hashbrown-sve-20260924/scripts/run_with_vl.py 32 \
  numactl --physcpubind=240,242,244,246,248,250,252,254 --membind=3 \
  "$W/venv-stock/bin/python" "$W/scripts/profile_tokenizer.py" "$P/control" "$P/ack"
perf report -i "$P/perf.data" --stdio -g none --no-children --sort dso,symbol --percent-limit 1 > "$P/report.txt"
head -70 "$P/report.txt"
