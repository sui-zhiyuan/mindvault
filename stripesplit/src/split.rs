//! 分割：按字节交错把源文件写成 `n` 个分片，并落一份 `index.toml`。

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::index::{Index, PartEntry, INDEX_FILE};
use crate::{check_fd_capacity, part_len, round_bytes, same_file, ALGORITHM, TOOL};

pub struct SplitOptions {
    /// 分片数量，必须 >= 1。
    pub parts: u64,
    /// 覆盖已存在的分片与索引。
    pub force: bool,
}

/// 把 `src` 按字节交错切成 `opts.parts` 份，输出到 `outdir`，返回写出的索引。
///
/// 任一步失败（磁盘满、写入错误、Ctrl-C 等）都会删掉本次已经创建的分片，
/// 不留下能被误用的半成品。
pub fn split(src: &Path, outdir: &Path, opts: &SplitOptions) -> Result<Index> {
    let meta = fs::metadata(src).with_context(|| format!("无法读取源文件：{}", src.display()))?;
    if !meta.is_file() {
        bail!("源文件不是普通文件：{}", src.display());
    }
    let total = meta.len();
    let n = opts.parts;
    if n == 0 {
        bail!("份数必须大于 0");
    }
    check_fd_capacity(n)?;

    let file_name = src
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .with_context(|| format!("源文件名不合法：{}", src.display()))?;

    fs::create_dir_all(outdir)
        .with_context(|| format!("创建输出目录失败：{}", outdir.display()))?;

    let index_path = outdir.join(INDEX_FILE);
    if same_file(src, &index_path) {
        bail!(
            "索引文件会覆盖源文件，请换一个输出目录：{}",
            index_path.display()
        );
    }

    let width = part_width(n);
    let paths: Vec<PathBuf> = (0..n)
        .map(|i| outdir.join(part_name(file_name, width, i)))
        .collect();

    if !opts.force {
        if index_path.exists() {
            bail!("索引已存在：{}（加 -f/--force 覆盖）", index_path.display());
        }
        if let Some(existing) = paths.iter().find(|path| path.exists()) {
            bail!("分片已存在：{}（加 -f/--force 覆盖）", existing.display());
        }
    }

    // `rollback` 声明在 writers 之前，出错时 writers 先关闭、再删文件。
    let mut rollback = Rollback::new();
    let mut writers = Vec::with_capacity(n as usize);
    for path in &paths {
        let file =
            File::create(path).with_context(|| format!("创建分片失败：{}", path.display()))?;
        rollback.track(path.clone());
        writers.push(file);
    }

    let round = round_bytes(total, n);
    let per_part = round / n as usize;
    let mut buf = vec![0u8; round];
    let mut scratch = vec![0u8; per_part];
    let mut hashers: Vec<Sha256> = (0..n).map(|_| Sha256::new()).collect();
    let mut original = Sha256::new();

    let mut reader =
        File::open(src).with_context(|| format!("打开源文件失败：{}", src.display()))?;
    loop {
        let got = read_some(&mut reader, &mut buf)
            .with_context(|| format!("读取源文件失败：{}", src.display()))?;
        if got == 0 {
            break;
        }
        original.update(&buf[..got]);

        for (j, writer) in writers.iter_mut().enumerate() {
            let mut filled = 0;
            let mut pos = j;
            while pos < got {
                scratch[filled] = buf[pos];
                filled += 1;
                pos += n as usize;
            }
            if filled == 0 {
                continue;
            }
            hashers[j].update(&scratch[..filled]);
            writer
                .write_all(&scratch[..filled])
                .with_context(|| format!("写入分片失败：{}", paths[j].display()))?;
        }
    }
    drop(writers);

    let parts: Vec<PartEntry> = hashers
        .into_iter()
        .enumerate()
        .map(|(j, hasher)| PartEntry {
            name: part_name(file_name, width, j as u64),
            size: part_len(total, n, j as u64),
            sha256: format!("{:x}", hasher.finalize()),
        })
        .collect();

    let index = Index {
        version: crate::INDEX_VERSION,
        tool: TOOL.to_string(),
        algorithm: ALGORITHM.to_string(),
        original_name: file_name.to_string(),
        original_size: total,
        original_sha256: format!("{:x}", original.finalize()),
        part_count: n,
        parts,
    };
    index.validate()?;
    index
        .write(outdir)
        .with_context(|| format!("写入索引失败：{}", index_path.display()))?;

    rollback.commit();
    Ok(index)
}

/// 分片序号位数：至少 4 位，份数更多时自动加宽。
pub(crate) fn part_width(n: u64) -> usize {
    n.saturating_sub(1).to_string().len().max(4)
}

pub(crate) fn part_name(stem: &str, width: usize, i: u64) -> String {
    format!("{stem}.part{i:0width$}")
}

/// 尽量把 `buf` 填满，返回实际读到的字节数；`0` 表示读到文件尾。
fn read_some(reader: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(filled)
}

/// 失败时清理本次创建的分片。
struct Rollback {
    paths: Vec<PathBuf>,
    armed: bool,
}

impl Rollback {
    fn new() -> Self {
        Self {
            paths: Vec::new(),
            armed: true,
        }
    }

    fn track(&mut self, path: PathBuf) {
        self.paths.push(path);
    }

    fn commit(&mut self) {
        self.armed = false;
    }
}

impl Drop for Rollback {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for path in &self.paths {
            let _ = fs::remove_file(path);
        }
    }
}
