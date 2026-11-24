//! 合并：按索引核对每个分片，再交错还原并校验原始 SHA-256。

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::index::{Index, INDEX_FILE};
use crate::{check_fd_capacity, round_bytes, same_file};

pub struct MergeOptions {
    /// 覆盖已存在的输出文件。
    pub force: bool,
}

/// 用 `index` 把 `dir` 里的分片还原到 `out`。
///
/// 校验分三层：索引自洽（[`Index::validate`]）、每个分片的大小与 SHA-256、
/// 还原结果的 SHA-256。任一层不通过都会删掉本次写出的输出文件。
pub fn merge(index: &Index, dir: &Path, out: &Path, opts: &MergeOptions) -> Result<()> {
    index.validate()?;
    let n = index.parts.len();
    check_fd_capacity(index.part_count)?;

    let index_path = dir.join(INDEX_FILE);
    if same_file(out, &index_path) {
        bail!("输出文件与索引文件是同一个文件：{}", out.display());
    }
    if out.exists() && !opts.force {
        bail!("输出文件已存在：{}（加 -f/--force 覆盖）", out.display());
    }

    let mut paths = Vec::with_capacity(n);
    for entry in &index.parts {
        let path = dir.join(&entry.name);
        let meta =
            fs::metadata(&path).with_context(|| format!("找不到分片：{}", path.display()))?;
        if !meta.is_file() {
            bail!("分片不是普通文件：{}", path.display());
        }
        if meta.len() != entry.size {
            bail!(
                "分片大小不符：{} 期望 {} 字节，实际 {} 字节（分片损坏或被改动过）",
                path.display(),
                entry.size,
                meta.len()
            );
        }
        if same_file(out, &path) {
            bail!("输出文件与分片是同一个文件：{}", path.display());
        }
        paths.push(path);
    }

    let mut output = OutputFile::create(out, opts.force)?;

    if index.parts.is_empty() {
        // 空文件 + 0 个分片（正常分割总是至少写 1 个分片，这里只做兼容）。
        output.flush()?;
        let actual = format!("{:x}", Sha256::new().finalize());
        if actual != index.original_sha256 {
            bail!(
                "原始文件 SHA-256 不匹配：期望 {}，实际 {actual}",
                index.original_sha256
            );
        }
        output.disarm();
        return Ok(());
    }

    let total = index.original_size;
    let round = round_bytes(total, index.part_count);
    let per_part = round / n;
    let mut buf = vec![0u8; round];
    let mut scratch = vec![0u8; per_part];
    let mut hashers: Vec<Sha256> = (0..n).map(|_| Sha256::new()).collect();
    let mut consumed = vec![0u64; n];
    let mut original = Sha256::new();
    let mut readers: Vec<File> = paths
        .iter()
        .map(|path| File::open(path).with_context(|| format!("打开分片失败：{}", path.display())))
        .collect::<Result<_>>()?;

    let mut base = 0u64;
    while base < total {
        let local = base / n as u64;
        let mut max_take: usize = 0;

        for j in 0..n {
            let take = index.parts[j]
                .size
                .saturating_sub(local)
                .min(per_part as u64) as usize;
            max_take = max_take.max(take);
            if take == 0 {
                continue;
            }
            readers[j]
                .read_exact(&mut scratch[..take])
                .with_context(|| format!("读取分片失败：{}", paths[j].display()))?;
            hashers[j].update(&scratch[..take]);
            consumed[j] += take as u64;
            for (t, byte) in scratch[..take].iter().enumerate() {
                buf[t * n + j] = *byte;
            }
        }

        let valid = ((n as u64 * max_take as u64).min(total - base)) as usize;
        original.update(&buf[..valid]);
        output
            .write_all(&buf[..valid])
            .with_context(|| format!("写入输出文件失败：{}", out.display()))?;
        base += valid as u64;
    }
    drop(readers);
    output.flush()?;

    for (j, hasher) in hashers.into_iter().enumerate() {
        if consumed[j] != index.parts[j].size {
            bail!(
                "分片 {} 读取了 {} 字节，索引记录 {} 字节",
                index.parts[j].name,
                consumed[j],
                index.parts[j].size
            );
        }
        let actual = format!("{:x}", hasher.finalize());
        if actual != index.parts[j].sha256 {
            bail!(
                "分片校验失败：{} 的 SHA-256 与索引记录不一致（分片损坏或被改动过）",
                index.parts[j].name
            );
        }
    }

    let actual = format!("{:x}", original.finalize());
    if actual != index.original_sha256 {
        bail!(
            "原始文件 SHA-256 不匹配：索引期望 {}，还原得到 {actual}",
            index.original_sha256
        );
    }

    output.disarm();
    Ok(())
}

/// 输出文件 + 失败自动删除。
struct OutputFile {
    path: PathBuf,
    file: Option<File>,
    armed: bool,
}

impl OutputFile {
    fn create(path: &Path, force: bool) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.write(true);
        if force {
            options.create(true).truncate(true);
        } else {
            options.create_new(true);
        }

        let mut output = Self {
            path: path.to_path_buf(),
            file: None,
            armed: false,
        };
        let file = options
            .open(path)
            .with_context(|| format!("创建输出文件失败：{}", path.display()))?;
        output.file = Some(file);
        output.armed = true;
        Ok(output)
    }

    fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.file.as_mut().expect("输出文件已关闭").write_all(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.as_mut().expect("输出文件已关闭").flush()
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for OutputFile {
    fn drop(&mut self) {
        // 先关文件再删除，Windows 上打开着的文件删不掉。
        self.file = None;
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
