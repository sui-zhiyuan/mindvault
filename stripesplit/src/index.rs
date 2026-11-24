//! `index.toml` 的数据结构、读写与校验。

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{ALGORITHM, TOOL};

/// 索引文件名，固定放在输出目录里。
pub const INDEX_FILE: &str = "index.toml";

pub const INDEX_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartEntry {
    /// 分片文件名（不含目录）。
    pub name: String,
    /// 分片字节数。
    pub size: u64,
    /// 分片内容的 SHA-256（小写十六进制）。
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    #[serde(default = "tool_name")]
    pub tool: String,
    /// 分割算法标记，目前只有 `byte-interleave`。
    pub algorithm: String,
    /// 分割前的文件名（仅记录，合并时不会据此覆盖文件）。
    pub original_name: String,
    /// 分割前的字节数。
    pub original_size: u64,
    /// 分割前内容的 SHA-256（小写十六进制）。
    pub original_sha256: String,
    /// 分片数量。
    pub part_count: u64,
    pub parts: Vec<PartEntry>,
}

fn tool_name() -> String {
    TOOL.to_string()
}

impl Index {
    /// 从「输出目录」或「index.toml 路径」加载索引，并返回索引所在目录。
    pub fn load(input: &Path) -> Result<(Index, PathBuf)> {
        let path = if input.is_dir() {
            input.join(INDEX_FILE)
        } else {
            input.to_path_buf()
        };
        let text = fs::read_to_string(&path)
            .with_context(|| format!("读取索引失败：{}", path.display()))?;
        let index: Index = toml::from_str(&text).with_context(|| {
            format!(
                "解析索引失败（不是合法的 index.toml？）：{}",
                path.display()
            )
        })?;
        index.validate()?;

        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Ok((index, dir))
    }

    /// 原子地写入 `<dir>/index.toml`（先写临时文件再改名）。
    pub fn write(&self, dir: &Path) -> Result<PathBuf> {
        let text = toml::to_string_pretty(self).context("序列化 index.toml 失败")?;
        let path = dir.join(INDEX_FILE);
        let tmp = dir.join(format!(".{INDEX_FILE}.tmp"));

        fs::write(&tmp, text).with_context(|| format!("写入临时索引失败：{}", tmp.display()))?;
        #[cfg(windows)]
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("无法覆盖旧索引：{}", path.display()))?;
        }
        fs::rename(&tmp, &path).with_context(|| format!("替换索引失败：{}", path.display()))?;
        Ok(path)
    }

    pub fn part_path(&self, dir: &Path, i: usize) -> PathBuf {
        dir.join(&self.parts[i].name)
    }

    /// 校验索引自身是否自洽（版本、算法、数量、大小合计、文件名与摘要格式）。
    pub fn validate(&self) -> Result<()> {
        if self.version != INDEX_VERSION {
            bail!(
                "不支持的索引版本 {}（当前只支持 {}）",
                self.version,
                INDEX_VERSION
            );
        }
        if self.algorithm != ALGORITHM {
            bail!(
                "不支持的分割算法 {:?}（当前只支持 {:?}）",
                self.algorithm,
                ALGORITHM
            );
        }
        if self.part_count != self.parts.len() as u64 {
            bail!(
                "part_count = {} 与 parts 数量 {} 不一致",
                self.part_count,
                self.parts.len()
            );
        }
        if !is_sha256(&self.original_sha256) {
            bail!("original_sha256 不是 64 位十六进制字符串");
        }
        if self.parts.is_empty() {
            if self.original_size == 0 {
                return Ok(());
            }
            bail!(
                "索引里没有任何分片，但 original_size = {}",
                self.original_size
            );
        }

        let mut seen = HashSet::with_capacity(self.parts.len());
        let mut sum: u64 = 0;
        for entry in &self.parts {
            if !is_plain_name(&entry.name) {
                bail!("分片名不合法（不允许目录分隔符或 ..）：{:?}", entry.name);
            }
            if !seen.insert(entry.name.as_str()) {
                bail!("分片名重复：{:?}", entry.name);
            }
            if !is_sha256(&entry.sha256) {
                bail!("分片 {:?} 的 sha256 不是 64 位十六进制字符串", entry.name);
            }
            sum = sum
                .checked_add(entry.size)
                .with_context(|| format!("分片大小合计溢出：{:?}", entry.name))?;
        }
        if sum != self.original_size {
            bail!(
                "分片大小合计 {sum} 与 original_size {} 不一致",
                self.original_size
            );
        }
        Ok(())
    }
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_plain_name(name: &str) -> bool {
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if name.contains(['/', '\\']) {
        return false;
    }
    Path::new(name).components().count() == 1
}
