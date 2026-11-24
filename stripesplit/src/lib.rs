//! stripesplit —— 按字节交错把文件切成多个分片，并用 TOML 索引校验合并。
//!
//! 分割规则：把文件分成 `n` 份时，第 `i` 份由原文件中下标为
//! `i, i+n, i+2n, …` 的字节按出现顺序组成（`0 <= i < n`）。
//! 因此 `n` 份的大小最多相差 1 字节，全部拼回即为原文件。
//!
//! 每次分割都会在输出目录写一个 [`INDEX_FILE`]，记录原始文件的大小与
//! SHA-256、分片数量，以及每个分片的文件名、大小与 SHA-256。
//! [`merge`] 先按索引核对分片，再交错还原，最后用原始 SHA-256 收尾。

use std::path::Path;

pub mod index;
pub mod merge;
pub mod split;

pub use index::{Index, PartEntry, INDEX_FILE, INDEX_VERSION};
pub use merge::{merge, MergeOptions};
pub use split::{split, SplitOptions};

pub const TOOL: &str = "stripesplit";
pub const ALGORITHM: &str = "byte-interleave";

/// 不指定 `-s`/`-n` 时，每个分片的目标大小：1 MiB。
pub const DEFAULT_PART_SIZE: u64 = 1 << 20;

/// 单轮读取的总预算：分片越多、单轮越小，总内存仍控制在这个量级。
const ROUND_BUDGET: u64 = 64 << 20;

/// 单个分片在两轮之间允许积攒的最大字节数，避免 `n = 1` 时缓冲区翻倍。
const MAX_PER_PART: u64 = 8 << 20;

/// 每轮处理的原始字节数：`n` 的整数倍，且尽量贴着预算，不无谓地放大。
pub fn round_bytes(total: u64, n: u64) -> usize {
    let n = n.max(1);
    let ceil = total.div_ceil(n).max(1);
    let per_part = (ROUND_BUDGET / n).clamp(1, MAX_PER_PART).min(ceil);
    (per_part * n) as usize
}

/// 第 `i` 个分片包含的字节数，即原文件中下标模 `n` 余 `i` 的字节个数。
pub fn part_len(total: u64, n: u64, i: u64) -> u64 {
    if i >= total {
        0
    } else {
        (total - i).div_ceil(n)
    }
}

/// 按目标分片大小推算份数（至少 1 份）。
pub fn part_count_for_size(total: u64, target: u64) -> u64 {
    total.div_ceil(target.max(1)).max(1)
}

/// 人类可读的大小。
pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}

/// 解析 `1M` / `1.5G` / `512K` / `104857600` 这类大小写法（K/M/G/T 按 1024 进制）。
pub fn parse_size(text: &str) -> Result<u64, String> {
    let raw = text.trim().to_ascii_lowercase().replace(['_', ' '], "");
    if raw.is_empty() {
        return Err("大小不能为空".to_string());
    }
    let digits_end = raw
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(raw.len());
    let (number, mut unit) = raw.split_at(digits_end);
    for suffix in ["ib", "b"] {
        if let Some(rest) = unit.strip_suffix(suffix) {
            unit = rest;
            break;
        }
    }
    let scale: u64 = match unit {
        "" => 1,
        "k" => 1 << 10,
        "m" => 1 << 20,
        "g" => 1 << 30,
        "t" => 1 << 40,
        _ => return Err(format!("无法识别的大小单位：{text:?}（可用 K/M/G/T）")),
    };
    let value: f64 = number
        .parse()
        .map_err(|_| format!("无法解析的大小：{text:?}（示例：1M、512K、104857600）"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("大小必须大于 0：{text:?}"));
    }
    let bytes = (value * scale as f64).round();
    if bytes < 1.0 {
        return Err(format!("大小必须大于 0：{text:?}"));
    }
    Ok(bytes as u64)
}

/// 两个路径是否指向同一个已存在的文件（不存在的路径返回 `false`）。
pub fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// 当前进程可用的文件描述符上限（Unix）。
#[cfg(unix)]
pub fn fd_limit() -> Option<u64> {
    let mut limit = std::mem::MaybeUninit::<libc::rlimit>::uninit();
    // SAFETY: getrlimit 只写入指向的结构体，指针有效且对齐。
    let ok = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, limit.as_mut_ptr()) } == 0;
    if !ok {
        return None;
    }
    // SAFETY: 调用成功即已初始化。
    let limit = unsafe { limit.assume_init() };
    // rlim_t 在 64 位系统上就是 u64，32 位系统上需要扩展。
    #[allow(clippy::unnecessary_cast)]
    let current = limit.rlim_cur as u64;
    Some(current)
}

/// 提前确认能同时打开 `n` 个分片文件，避免写到一半才因 EMFILE 失败。
pub fn check_fd_capacity(n: u64) -> anyhow::Result<()> {
    #[cfg(unix)]
    if let Some(limit) = fd_limit() {
        if n.saturating_add(32) > limit {
            anyhow::bail!(
                "需要同时打开 {n} 个分片文件，但当前文件描述符上限只有 {limit}；\
                 请用更大的 -s（更少的份数）或先调高 `ulimit -n`"
            );
        }
    }
    #[cfg(not(unix))]
    let _ = n;
    Ok(())
}
