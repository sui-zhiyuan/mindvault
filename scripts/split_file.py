#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# /// script
# requires-python = ">=3.10"
# dependencies = []
# ///
#
# 单文件脚本：上面这段 PEP 723 内联元数据说明它「零第三方依赖」（只用标准库），
# 因此既可以 `python split_file.py …` 直接跑，也可以交给 uv：
#     uv run split_file.py big.iso 100M -c xz
# 若将来引入第三方包，只改上面的 dependencies 列表即可，无需 requirements.txt。
"""split_file.py —— 只依赖 Python 标准库的大文件分片工具（可流式压缩再分片）。

设计目标
    * 拆分：把任意文件切成若干个「不超过指定大小」的分片，内存占用恒定。
    * 压缩：可选先把文件流式压缩（gzip / bz2 / xz），再把压缩流切分——
      等价于 7-Zip 的分卷压缩，但每片是「压缩流的字节切片」，拼回后即为完整
      压缩文件，可由本脚本或系统工具解压。
    * 合并：拼回分片，自动识别 gzip / bz2 / xz 并解压还原。

用法示例
    python split_file.py big.iso 100M               # 每片最大 100 MiB（纯字节切分）
    python split_file.py big.iso -n 5               # 拆成 5 片（与 size 二选一）
    python split_file.py big.iso 100M -c            # 先 gzip 压缩再切分（-c 即 -c gzip）
    python split_file.py big.iso 100M -c xz -l 9 -m # xz 最高级别压缩 + 写 manifest
    python split_file.py big.iso 100M -c -o D:/parts -f
    python split_file.py --join big.iso.gz.part001  # 合并还原（自动解压）
    python split_file.py --join D:/parts --out big.iso   # 目录里只有一组分片时也行
    python split_file.py --join big.iso.gz.part001 --raw   # 只拼接、不解压

命名约定（7-Zip 风格，卷号在最后）
    big.iso        -> big.iso.part001、big.iso.part002 …
    big.iso -c xz  -> big.iso.xz.part001、big.iso.xz.part002 …
    -m 追加一份 big.iso.xz.manifest.json，记录原始名/大小/SHA256/编码，合并时优先采信。
"""

from __future__ import annotations

import argparse
import bz2
import hashlib
import json
import lzma
import math
import os
import re
import sys
import zlib
from pathlib import Path

MANIFEST_VERSION = 1
READ_BUF = 4 * 1024 * 1024        # 拆分时每次读入的字节数
DEC_IN_BUF = 1 * 1024 * 1024      # 合并/解压时每次读入的字节数
DEC_OUT_MAX = 8 * 1024 * 1024     # 单次解压输出上限，防止高压缩比数据顶爆内存

CODECS = ("none", "gzip", "bz2", "xz")
CODEC_EXT = {"gzip": "gz", "bz2": "bz2", "xz": "xz"}
EXT_CODEC = {"gz": "gzip", "gzip": "gzip", "bz2": "bz2", "xz": "xz", "lzma": "xz"}
DEFAULT_LEVEL = {"gzip": 6, "bz2": 9, "xz": 6}
LEVEL_RANGE = {"gzip": (0, 9), "bz2": (1, 9), "xz": (0, 9)}

PART_RE = re.compile(r"^(?P<stem>.+?)\.part(?P<num>\d+)$")

# 各解压器遇到坏数据时抛出的异常类型（zlib / lzma / bz2 各不相同）
DEC_ERRORS = (zlib.error, lzma.LZMAError, EOFError, OSError, ValueError)

EXAMPLES = """\
示例:
  python split_file.py big.iso 100M                每片最大 100 MiB
  python split_file.py big.iso 700M -o /mnt/usb     指定分片输出目录
  python split_file.py big.iso -n 5                拆成 5 片（与 size 二选一）
  python split_file.py big.iso 100M -c             先 gzip 压缩再切分
  python split_file.py big.iso 100M -c xz -l 9 -m  xz 最高压缩 + 写 manifest
  python split_file.py big.iso 100M -c -f --verify 覆盖旧分片并回读校验 SHA256
  python split_file.py --join big.iso.gz.part001   合并还原（自动识别解压）
  python split_file.py --join ./parts --out big.iso 从目录合并到指定文件
  python split_file.py --join big.iso.gz.part001 --raw   只拼接、不解压

大小写法:
  100M / 1.5G / 700m / 512K / 104857600（无单位即字节；K/M/G/T 均按 1024 进制）
"""


class SplitError(Exception):
    """可预期的用户级错误。"""


# --------------------------------------------------------------------------- #
# 小工具
# --------------------------------------------------------------------------- #
def die(msg: str, code: int = 1) -> None:
    print(f"错误: {msg}", file=sys.stderr)
    raise SystemExit(code)


def parse_size(text: str) -> int:
    """把 '100M' / '1.5G' / '104857600' 解析成字节数。"""
    s = text.strip().lower().replace("_", "").replace(" ", "")
    m = re.fullmatch(r"(\d+(?:\.\d+)?)([kmgt]?)(?:i?b)?", s)
    if not m:
        raise argparse.ArgumentTypeError(
            f"无法解析的大小 {text!r}（示例：100M、1.5G、512K、104857600）"
        )
    num, unit = m.group(1), m.group(2)
    scale = {"": 1, "k": 1024, "m": 1024 ** 2, "g": 1024 ** 3, "t": 1024 ** 4}[unit]
    size = int(float(num) * scale)
    if size <= 0:
        raise argparse.ArgumentTypeError("大小必须大于 0")
    return size


def positive_int(text: str) -> int:
    try:
        value = int(text)
    except ValueError:
        raise argparse.ArgumentTypeError(f"需要一个正整数，收到 {text!r}") from None
    if value <= 0:
        raise argparse.ArgumentTypeError("必须是大于 0 的整数")
    return value


def human(n: int) -> str:
    """人类可读的大小，如 '96.00 MiB'。"""
    value = float(n)
    for unit in ("B", "KiB", "MiB", "GiB", "TiB"):
        if value < 1024 or unit == "TiB":
            return f"{int(value)} {unit}" if unit == "B" else f"{value:.2f} {unit}"
        value /= 1024
    return f"{n} B"


def human_bytes(n: int) -> str:
    return f"{n:,} B ({human(n)})" if n >= 1024 else f"{n:,} B"


def log(args: argparse.Namespace, msg: str) -> None:
    if not getattr(args, "quiet", False):
        print(msg)


def progress(done: int, total: int, label: str = "") -> None:
    """在 stderr 上画一条纯 ASCII 进度条（不污染 stdout）。"""
    width = 26
    ratio = 1.0 if total <= 0 else min(1.0, done / total)
    filled = int(width * ratio)
    bar = "#" * filled + "-" * (width - filled)
    sys.stderr.write(f"\r{label}[{bar}] {ratio * 100:5.1f}%  {human(done)}/{human(total)}")
    sys.stderr.flush()


def clear_progress() -> None:
    sys.stderr.write("\r" + " " * 96 + "\r")
    sys.stderr.flush()


def sniff_codec(head: bytes) -> str | None:
    """按文件头猜压缩格式。"""
    if head[:2] == b"\x1f\x8b":
        return "gzip"
    if head[:3] == b"BZh":
        return "bz2"
    if head[:6] == b"\xfd7zXZ\x00":
        return "xz"
    if head[:3] == b"]\x00\x00":       # .lzma (FORMAT_ALONE)
        return "xz"
    return None


def head_bytes(path: Path, n: int = 64) -> bytes:
    with path.open("rb") as fh:
        return fh.read(n)


def _unlink_quiet(path: Path) -> None:
    """删除半成品，删不掉也不报错。"""
    try:
        path.unlink()
    except OSError:
        pass


# --------------------------------------------------------------------------- #
# 压缩 / 解压（流式，统一走 compressobj / decompressobj 接口）
# --------------------------------------------------------------------------- #
def new_compressor(codec: str, level: int | None):
    if codec == "gzip":
        return zlib.compressobj(DEFAULT_LEVEL["gzip"] if level is None else level,
                                zlib.DEFLATED, 31)          # 31 = gzip 容器
    if codec == "bz2":
        return bz2.BZ2Compressor(DEFAULT_LEVEL["bz2"] if level is None else level)
    if codec == "xz":
        return lzma.LZMACompressor(preset=DEFAULT_LEVEL["xz"] if level is None else level)
    raise SplitError(f"未知压缩格式: {codec}")


def new_decompressor(codec: str):
    if codec == "gzip":
        return zlib.decompressobj(31)
    if codec == "bz2":
        return bz2.BZ2Decompressor()
    if codec == "xz":
        return lzma.LZMADecompressor(format=lzma.FORMAT_AUTO)
    raise SplitError(f"未知压缩格式: {codec}")


def iter_file(path: Path, buf: int = DEC_IN_BUF):
    with path.open("rb") as fh:
        while True:
            block = fh.read(buf)
            if not block:
                break
            yield block


def iter_parts(parts: list[Path]):
    for part in parts:
        yield from iter_file(part)


def decompress_into(parts: list[Path], codec: str, sink) -> int:
    """把分片串成一个流，按 codec 解压，结果交给 sink；返回解压后字节数。

    支持多成员串联（拼接过的 .gz/.bz2/.xz），并对「分片缺失导致截断」报错。
    """
    total = 0
    dec = new_decompressor(codec)
    empty_calls = 0

    for piece in iter_parts(parts):
        data: bytes | None = piece
        while data is not None:
            if dec.eof:                     # 上一段压缩流已结束，后面是下一个成员
                dec = new_decompressor(codec)
            try:
                out = dec.decompress(data, DEC_OUT_MAX)
            except DEC_ERRORS as exc:
                raise SplitError(f"压缩数据已损坏，或与 {codec} 格式不符：{exc}") from None
            if out:
                sink(out)
                total += len(out)
                empty_calls = 0

            if dec.eof:
                leftover = dec.unused_data
                data = None if leftover.strip(b"\x00") == b"" else leftover
                continue

            tail = getattr(dec, "unconsumed_tail", None)   # zlib 会留下未消费输入
            if tail is not None:
                data = tail or None
            elif not dec.needs_input:                      # bz2 / lzma：先榨干内部缓冲
                empty_calls += 1
                if empty_calls > 4:
                    raise SplitError("解压器停滞，压缩数据可能已损坏")
                data = b""
            else:
                data = None

    flush = getattr(dec, "flush", None)
    if flush is not None:
        try:
            out = flush()
        except DEC_ERRORS as exc:
            raise SplitError(f"压缩数据被截断或不完整（是否缺少分片？）：{exc}") from None
        if out:
            sink(out)
            total += len(out)

    if not dec.eof:
        raise SplitError("压缩数据不完整：分片可能缺失，或该文件本就不是压缩流（可加 --raw）")
    return total


# --------------------------------------------------------------------------- #
# 分片写入器：写满一片就自动换下一片
# --------------------------------------------------------------------------- #
class PartWriter:
    """把连续字节流按 chunk 切成 <prefix>.partNNN。"""

    def __init__(self, outdir: Path, prefix: str, chunk: int, width: int, force: bool):
        self.outdir = outdir
        self.prefix = prefix
        self.chunk = chunk
        self.width = width
        self.force = force
        self.total = 0
        self.parts: list[Path] = []
        self._fh = None
        self._cur = 0
        self._count = 0

    # -- 内部 ------------------------------------------------------------- #
    def _open_next(self) -> None:
        self._count += 1
        path = self.outdir / f"{self.prefix}.part{self._count:0{self.width}d}"
        if path.exists() and not self.force:
            raise SplitError(f"分片已存在：{path.name}（加 -f/--force 覆盖）")
        self._fh = path.open("wb")
        self.parts.append(path)
        self._cur = 0

    def _close_current(self) -> None:
        if self._fh is not None:
            self._fh.close()
            self._fh = None
            self._cur = 0

    # -- 对外 ------------------------------------------------------------- #
    def write(self, data) -> int:
        mv = memoryview(data)
        size = len(mv)
        offset = 0
        while offset < size:
            if self._fh is None:
                self._open_next()
            take = min(self.chunk - self._cur, size - offset)
            self._fh.write(mv[offset:offset + take])
            offset += take
            self._cur += take
            self.total += take
            if self._cur >= self.chunk:
                self._close_current()
        return size

    def ensure_part(self) -> None:
        """空文件也留一个 0 字节分片，保证能还原。"""
        if not self.parts:
            self._open_next()
            self._close_current()

    def close(self) -> None:
        self._close_current()

    def rollback(self) -> None:
        """出错/中断时清掉本次写出的半成品分片。"""
        self._close_current()
        for part in self.parts:
            try:
                part.unlink()
            except OSError:
                pass
        self.parts.clear()


# --------------------------------------------------------------------------- #
# 分片发现
# --------------------------------------------------------------------------- #
def find_parts(directory: Path, prefix: str) -> list[Path]:
    pattern = re.compile(re.escape(prefix) + r"\.part(\d+)$")
    found: list[tuple[int, Path]] = []
    with os.scandir(directory) as entries:
        for entry in entries:
            if not entry.is_file():
                continue
            m = pattern.match(entry.name)
            if m:
                found.append((int(m.group(1)), Path(entry.path)))
    found.sort(key=lambda item: item[0])
    return [path for _, path in found]


def collect_parts(target: Path) -> tuple[str, Path, list[Path]]:
    """定位一组分片，返回 (分片名主干, 所在目录, 按序号排好的分片列表)。"""
    if target.is_dir():
        groups: dict[str, list[tuple[int, Path]]] = {}
        with os.scandir(target) as entries:
            for entry in entries:
                if not entry.is_file():
                    continue
                m = PART_RE.match(entry.name)
                if m:
                    groups.setdefault(m.group("stem"), []).append(
                        (int(m.group("num")), Path(entry.path))
                    )
        if not groups:
            raise SplitError(f"目录中没有 .partNNN 分片：{target}")
        if len(groups) > 1:
            names = "、".join(sorted(groups))
            raise SplitError(f"目录里有多组分片，请直接指定某一组的分片：{names}")
        stem, items = next(iter(groups.items()))
        base = target
    else:
        m = PART_RE.match(target.name)
        if not m:
            raise SplitError(f"文件名不像分片（应形如 name.part001）：{target.name}")
        stem = m.group("stem")
        base = target.parent
        pattern = re.compile(re.escape(stem) + r"\.part(\d+)$")
        items = []
        with os.scandir(base) as entries:
            for entry in entries:
                if not entry.is_file():
                    continue
                mm = pattern.match(entry.name)
                if mm:
                    items.append((int(mm.group(1)), Path(entry.path)))

    items.sort(key=lambda item: item[0])
    nums = [num for num, _ in items]
    if nums != list(range(1, len(nums) + 1)):
        missing = sorted(set(range(1, max(nums) + 1)) - set(nums))
        raise SplitError("分片序号不连续，缺少：" + "、".join(f"{n:03d}" for n in missing))
    return stem, base, [path for _, path in items]


# --------------------------------------------------------------------------- #
# manifest
# --------------------------------------------------------------------------- #
def manifest_path(base: Path, stem: str) -> Path:
    return base / f"{stem}.manifest.json"


def read_manifest(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        with path.open("r", encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, ValueError) as exc:
        print(f"警告: 忽略无法解析的 manifest（{path.name}）：{exc}", file=sys.stderr)
        return None
    return data if isinstance(data, dict) else None


def write_manifest(path: Path, payload: dict) -> None:
    with path.open("w", encoding="utf-8") as fh:
        json.dump(payload, fh, ensure_ascii=False, indent=2)
        fh.write("\n")


# --------------------------------------------------------------------------- #
# split
# --------------------------------------------------------------------------- #
def cmd_split(args: argparse.Namespace) -> int:
    src = Path(args.path)
    if not src.is_file():
        die(f"找不到文件：{src}")
    if args.size is not None and args.num_parts is not None:
        die("位置参数 <大小> 与 -n/--num-parts 只能给一个")
    if args.size is None and args.num_parts is None:
        die("请给出每个分片的最大大小（如 100M），或用 -n/--num-parts 指定份数")

    total = src.stat().st_size
    if args.num_parts is not None:
        chunk = max(1, math.ceil(total / args.num_parts)) if total else 1
    else:
        chunk = args.size

    codec = args.compress
    level = args.level
    if level is not None:
        if codec == "none":
            die("-l/--level 只在压缩时有效（配合 -c gzip|bz2|xz）")
        low, high = LEVEL_RANGE[codec]
        if not low <= level <= high:
            die(f"{codec} 的压缩级别需在 {low}-{high} 之间")

    outdir = Path(args.outdir) if args.outdir else src.parent
    prefix = src.name if codec == "none" else f"{src.name}.{CODEC_EXT[codec]}"
    width = max(args.digits, len(str(max(1, math.ceil(total / chunk)))))

    existing = find_parts(outdir, prefix) if outdir.is_dir() else []
    if existing and not args.force:
        die(f"输出目录已有 {len(existing)} 个同名分片（如 {existing[0].name}），"
            f"加 -f/--force 覆盖")
    outdir.mkdir(parents=True, exist_ok=True)

    writer = PartWriter(outdir, prefix, chunk, width, args.force)
    digest = hashlib.sha256()
    level_note = "" if codec == "none" else \
        f" 级别={level if level is not None else DEFAULT_LEVEL[codec]}"
    label = "压缩中 " if codec != "none" else "拆分中 "
    log(args, f"源文件  : {src}  ({human_bytes(total)})")
    log(args, f"分片上限: {human_bytes(chunk)}   压缩: {codec}{level_note}")

    compressor = new_compressor(codec, level) if codec != "none" else None
    try:
        with src.open("rb") as fh:
            done = 0
            while True:
                block = fh.read(READ_BUF)
                if not block:
                    break
                digest.update(block)
                if compressor is None:
                    writer.write(block)
                else:
                    out = compressor.compress(block)
                    if out:
                        writer.write(out)
                done += len(block)
                if not args.quiet:
                    progress(done, total, label)
        if compressor is not None:
            tail = compressor.flush()
            if tail:
                writer.write(tail)
        writer.ensure_part()
    except KeyboardInterrupt:
        writer.rollback()
        if not args.quiet:
            clear_progress()
        die("已中断，半成品分片已清理", 130)
    except Exception as exc:          # 含 OSError / SplitError / 压缩库异常
        writer.rollback()
        if not args.quiet:
            clear_progress()
        die(f"写出分片失败，已清理半成品：{exc}")
    finally:
        writer.close()

    if not args.quiet:
        clear_progress()

    original_hash = digest.hexdigest()
    log(args, f"完成    : {len(writer.parts)} 个分片 -> {outdir}")
    for part in writer.parts:
        log(args, f"  {part.name}  {human_bytes(part.stat().st_size)}")
    log(args, f"原始文件 SHA256: {original_hash}")

    if args.manifest:
        mpath = manifest_path(outdir, prefix)
        write_manifest(mpath, {
            "tool": "split_file.py",
            "version": MANIFEST_VERSION,
            "original_name": src.name,
            "original_size": total,
            "sha256": original_hash,
            "chunk": chunk,
            "codec": codec,
            "codec_level": level if level is not None else (
                DEFAULT_LEVEL[codec] if codec != "none" else None),
            "parts": len(writer.parts),
            "part_prefix": prefix,
        })
        log(args, f"manifest: {mpath}")

    if args.verify:
        log(args, "回读校验中…")
        check = hashlib.sha256()
        try:
            if codec == "none":
                for part in writer.parts:
                    for block in iter_file(part):
                        check.update(block)
            else:
                decompress_into(writer.parts, codec, check.update)
        except SplitError as exc:
            die(f"回读校验失败：{exc}")
        if check.hexdigest() != original_hash:
            die("回读校验失败：分片内容与源文件 SHA256 不一致")
        log(args, "回读校验通过：分片可完整还原")

    if not args.quiet:
        script = Path(sys.argv[0]).name or "split_file.py"
        print(f"\n合并还原:\n  python {script} --join \"{writer.parts[0].name}\"",
              file=sys.stderr)
        if codec != "none":
            print(f"  （分片按序号拼起来就是完整的 {src.name}.{CODEC_EXT[codec]}，"
                  f"也可用 7-Zip / gzip / xz 直接解压）", file=sys.stderr)
    return 0


# --------------------------------------------------------------------------- #
# join
# --------------------------------------------------------------------------- #
def cmd_join(args: argparse.Namespace) -> int:
    target = Path(args.path)
    if not target.exists():
        die(f"找不到分片或目录：{target}")

    try:
        stem, base, parts = collect_parts(target)
    except SplitError as exc:
        die(str(exc))

    mpath = manifest_path(base, stem)
    manifest = read_manifest(mpath)
    manifest_codec = manifest.get("codec") if manifest else None
    if manifest:
        log(args, f"发现 manifest: {mpath.name}"
                  f"（来源 {manifest.get('original_name', '?')}，"
                  f"编码 {manifest_codec or 'none'}）")

    detected = sniff_codec(head_bytes(parts[0]))
    if args.raw:
        codec: str | None = None
    elif manifest_codec and manifest_codec != "none":
        codec = manifest_codec
    elif detected is not None:
        codec = detected
    else:
        codec = None
        ext = stem.rsplit(".", 1)[-1].lower() if "." in stem else ""
        if ext in EXT_CODEC:
            print(f"提示: 分片名以 .{ext} 结尾，但第一片的内容不是有效的 "
                  f"{EXT_CODEC[ext]} 流，将按原样拼接（若分片缺了头几片，"
                  f"请核对后再试）", file=sys.stderr)

    # manifest 是否与本次实际做的事匹配（决定能否用它的原始名与 SHA256）
    manifest_applies = manifest is not None and (
        (codec is None and (manifest_codec in (None, "none"))) or
        (codec is not None and manifest_codec == codec)
    )

    if args.out:
        out = Path(args.out)
    elif manifest_applies and manifest.get("original_name"):
        out = base / str(manifest["original_name"])
    else:
        name = stem
        if codec is not None:
            suffix = "." + CODEC_EXT[codec]
            if name.lower().endswith(suffix):
                name = name[: -len(suffix)]
            elif name.lower().endswith("." + codec):
                name = name[: -(len(codec) + 1)]
        out = base / name

    if out.exists() and not args.force:
        die(f"输出文件已存在：{out}（加 -f/--force 覆盖）")
    if not out.parent.is_dir():
        out.parent.mkdir(parents=True, exist_ok=True)

    if codec is None:
        log(args, f"合并中  : {len(parts)} 个分片 -> {out}（按原样拼接，不解压）")
    else:
        log(args, f"合并中  : {len(parts)} 个分片 -> {out}"
                  f"（检测到 {codec}，合并后自动解压；如不想解压请加 --raw）")

    digest = hashlib.sha256()
    written = 0
    fh = None

    def sink(data: bytes) -> None:
        nonlocal written
        digest.update(data)
        fh.write(data)
        written += len(data)

    try:
        fh = out.open("wb")
        if codec is None:
            for part in parts:
                for block in iter_file(part):
                    sink(block)
        else:
            decompress_into(parts, codec, sink)
        fh.close()
        fh = None
    except KeyboardInterrupt:
        if fh is not None:
            fh.close()
        _unlink_quiet(out)
        die("已中断，未完成的输出文件已删除", 130)
    except Exception as exc:          # 含 OSError / SplitError / 解压库异常
        if fh is not None:
            fh.close()
        _unlink_quiet(out)
        die(f"合并失败，未完成的输出文件已删除：{exc}")

    actual = digest.hexdigest()
    log(args, f"完成    : {out}  {human_bytes(written)}")
    log(args, f"输出 SHA256: {actual}")

    if manifest_applies:
        recorded_size = manifest.get("original_size")
        if recorded_size is not None and int(recorded_size) != written:
            print(f"警告: 大小与 manifest 记录不符"
                  f"（manifest {recorded_size} 字节，实际 {written} 字节）", file=sys.stderr)
        recorded_hash = manifest.get("sha256")
        if recorded_hash and recorded_hash != actual:
            _unlink_quiet(out)
            die(f"校验失败：与 manifest 记录的 SHA256 不一致，已删除输出\n"
                f"  期望 {recorded_hash}\n  实际 {actual}")
        if recorded_hash:
            log(args, "校验通过：与 manifest 记录的 SHA256 一致")
    return 0


# --------------------------------------------------------------------------- #
# CLI
# --------------------------------------------------------------------------- #
def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="split_file.py",
        description="把大文件按大小切成多个分片（可选流式压缩再分片），或把分片合并还原。",
        epilog=EXAMPLES,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("path", help="拆分：要拆分的文件；--join：任一（建议第一个）分片，"
                                     "或只含一组分片的目录")
    parser.add_argument("size", nargs="?", type=parse_size,
                        help="每个分片的最大大小，如 100M、1.5G、104857600")
    parser.add_argument("-n", "--num-parts", type=positive_int, metavar="N",
                        help="改为「拆成 N 份」（拆分模式，与 size 互斥）")
    parser.add_argument("-c", "--compress", nargs="?", const="gzip", default="none",
                        choices=list(CODECS), metavar="CODEC",
                        help="先压缩再切分：gzip（-c 单独使用时的默认）、bz2、xz、none")
    parser.add_argument("-l", "--level", type=int, metavar="N",
                        help="压缩级别：gzip/xz 0-9，bz2 1-9"
                             f"（默认 gzip={DEFAULT_LEVEL['gzip']}、"
                             f"bz2={DEFAULT_LEVEL['bz2']}、xz={DEFAULT_LEVEL['xz']}）")
    parser.add_argument("-o", "--outdir", metavar="DIR",
                        help="拆分：分片输出目录（默认与源文件同目录）")
    parser.add_argument("-d", "--digits", type=int, default=3, metavar="N",
                        help="分片序号位数，默认 3（part001）；份数更多时自动加宽")
    parser.add_argument("-f", "--force", action="store_true",
                        help="覆盖已存在的分片 / 输出文件")
    parser.add_argument("-m", "--manifest", action="store_true",
                        help="额外写 <前缀>.manifest.json，记录原始名、SHA256、编码方式")
    parser.add_argument("--verify", action="store_true",
                        help="拆分后把分片回读一遍做 SHA256 校验（多一倍 I/O）")
    parser.add_argument("-q", "--quiet", action="store_true", help="安静模式，只报错误")

    join = parser.add_argument_group("合并模式")
    join.add_argument("--join", action="store_true",
                      help="合并分片（自动识别 gzip/bz2/xz 并解压）")
    join.add_argument("--out", metavar="FILE", help="--join 的输出文件名")
    join.add_argument("--raw", action="store_true",
                      help="--join 时只按字节拼接，不做任何解压")
    return parser


def main(argv: list[str] | None = None) -> int:
    # 中文输出在非中文控制台上不要直接崩掉
    for stream in (sys.stdout, sys.stderr):
        try:
            stream.reconfigure(errors="replace")   # type: ignore[union-attr]
        except (AttributeError, ValueError):
            pass

    args = build_parser().parse_args(argv)

    if args.join:
        if args.size is not None:
            die("--join 不需要 <大小> 参数")
        if args.num_parts is not None:
            die("--join 与 -n/--num-parts 无关，请去掉")
        if args.compress != "none" or args.level is not None or args.manifest:
            die("--join 会自动识别压缩格式，请去掉 -c/-l/-m")
        if args.outdir is not None:
            die("--join 的输出文件名请用 --out（-o/--outdir 是拆分模式的分片目录）")
        return cmd_join(args)

    if args.out:
        die("--out 只用于 --join")
    if args.raw:
        die("--raw 只用于 --join")
    if args.digits < 1:
        die("-d/--digits 至少为 1")
    return cmd_split(args)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except BrokenPipeError:
        raise SystemExit(0)
