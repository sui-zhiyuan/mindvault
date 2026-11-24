//! `stripesplit` 命令行入口。

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use sha2::{Digest, Sha256};

use stripesplit::{
    human, merge, parse_size, part_count_for_size, split, Index, MergeOptions, SplitOptions,
    DEFAULT_PART_SIZE, INDEX_FILE,
};

#[derive(Debug, Parser)]
#[command(
    name = "stripesplit",
    version,
    about = "按字节交错分割文件，并用 TOML 索引校验合并",
    after_help = "示例:\n  \
        stripesplit split big.iso -o parts            # 每份约 1 MiB\n  \
        stripesplit split big.iso -o parts -s 4M      # 每份约 4 MiB\n  \
        stripesplit split big.iso -o parts -n 8       # 固定 8 份\n  \
        stripesplit merge parts -o big.iso            # 按 index.toml 还原并校验\n  \
        stripesplit verify parts                      # 只校验分片"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// 把文件按字节交错切成 n 份，并在输出目录写 index.toml
    Split {
        /// 要分割的文件
        input: PathBuf,
        /// 输出目录（分片与 index.toml 都写在这里）
        #[arg(short = 'o', long = "outdir", value_name = "DIR")]
        outdir: PathBuf,
        /// 每份目标大小，如 1M、4M、512K、104857600（默认 1M，与 -n 二选一）
        #[arg(
            short = 's',
            long = "size",
            value_name = "SIZE",
            value_parser = size_arg,
            conflicts_with = "parts"
        )]
        size: Option<u64>,
        /// 份数（与 -s 二选一）
        #[arg(short = 'n', long = "parts", value_name = "N", value_parser = count_arg)]
        parts: Option<u64>,
        /// 覆盖已存在的分片与索引
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// 按 index.toml 合并还原，并校验分片与原始文件的 SHA-256
    Merge {
        /// 含 index.toml 的目录，或 index.toml 本身
        input: PathBuf,
        /// 还原出的文件
        #[arg(short = 'o', long = "out", value_name = "FILE")]
        output: PathBuf,
        /// 覆盖已存在的输出文件
        #[arg(short = 'f', long = "force")]
        force: bool,
    },
    /// 只校验分片的大小与 SHA-256，不写输出
    Verify {
        /// 含 index.toml 的目录，或 index.toml 本身
        input: PathBuf,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("错误: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Split {
            input,
            outdir,
            size,
            parts,
            force,
        } => {
            let total = std::fs::metadata(&input)
                .with_context(|| format!("无法读取源文件：{}", input.display()))?
                .len();
            let n = match (parts, size) {
                (Some(n), _) => n,
                (None, Some(size)) => part_count_for_size(total, size),
                (None, None) => part_count_for_size(total, DEFAULT_PART_SIZE),
            };
            if n > total {
                eprintln!("提示: 份数 {n} 大于文件字节数 {total}，部分分片会是空文件");
            }

            let started = Instant::now();
            let index = split(&input, &outdir, &SplitOptions { parts: n, force })?;
            print_split(&index, &outdir, started.elapsed());
            Ok(())
        }

        Command::Merge {
            input,
            output,
            force,
        } => {
            let (index, dir) = Index::load(&input)?;
            let started = Instant::now();
            merge(&index, &dir, &output, &MergeOptions { force })?;
            print_merge(&index, &dir, &output, started.elapsed());
            Ok(())
        }

        Command::Verify { input } => {
            let (index, dir) = Index::load(&input)?;
            verify(&index, &dir)
        }
    }
}

fn print_split(index: &Index, outdir: &Path, elapsed: Duration) {
    println!(
        "源文件 : {}  {}  sha256 {}",
        index.original_name,
        human(index.original_size),
        index.original_sha256
    );
    println!(
        "分割   : {} 份，字节交错（第 i 份取第 i, i+n, i+2n … 个字节）",
        index.part_count
    );
    println!("输出   : {}", outdir.display());

    let width = index
        .parts
        .iter()
        .map(|part| part.name.len())
        .max()
        .unwrap_or(0);
    if index.parts.len() <= 20 {
        for part in &index.parts {
            println!(
                "  {:<width$}  {:>10}  {}",
                part.name,
                human(part.size),
                part.sha256
            );
        }
    } else {
        for part in index.parts.iter().take(3) {
            println!(
                "  {:<width$}  {:>10}  {}",
                part.name,
                human(part.size),
                part.sha256
            );
        }
        println!("  …… 共 {} 个分片", index.parts.len());
        let last = index.parts.last().expect("已确认非空");
        println!(
            "  {:<width$}  {:>10}  {}",
            last.name,
            human(last.size),
            last.sha256
        );
    }

    println!("索引   : {}", outdir.join(INDEX_FILE).display());
    println!("用时   : {:.2}s", elapsed.as_secs_f64());
}

fn print_merge(index: &Index, dir: &Path, out: &Path, elapsed: Duration) {
    println!("索引   : {}", dir.join(INDEX_FILE).display());
    println!(
        "原始   : {}  {}",
        index.original_name,
        human(index.original_size)
    );
    println!(
        "分片   : {} 个（大小与 SHA-256 全部匹配）",
        index.part_count
    );
    println!("输出   : {}  {}", out.display(), human(index.original_size));
    println!("校验   : 通过（原始 SHA-256 匹配）");
    println!("用时   : {:.2}s", elapsed.as_secs_f64());
}

fn verify(index: &Index, dir: &Path) -> Result<()> {
    println!(
        "索引   : {}（{} 个分片，原始 {}）",
        dir.join(INDEX_FILE).display(),
        index.part_count,
        human(index.original_size)
    );

    let mut failures = 0;
    let mut buf = vec![0u8; 1 << 20];
    for part in &index.parts {
        let path = dir.join(&part.name);
        match verify_part(&path, &mut buf) {
            Ok((size, digest)) => {
                if size != part.size {
                    eprintln!(
                        "失败: {} 大小不符，索引 {} 字节，实际 {size} 字节",
                        part.name, part.size
                    );
                    failures += 1;
                } else if digest != part.sha256 {
                    eprintln!("失败: {} 的 SHA-256 与索引记录不一致", part.name);
                    failures += 1;
                }
            }
            Err(err) => {
                eprintln!("失败: {} 无法读取：{err:#}", part.name);
                failures += 1;
            }
        }
    }

    if failures > 0 {
        bail!("{failures} 个分片校验未通过");
    }
    println!("结果   : 全部 {} 个分片校验通过", index.part_count);
    Ok(())
}

fn verify_part(path: &Path, buf: &mut [u8]) -> Result<(u64, String)> {
    let mut file = File::open(path).with_context(|| format!("打开分片失败：{}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    loop {
        let got = file
            .read(buf)
            .with_context(|| format!("读取分片失败：{}", path.display()))?;
        if got == 0 {
            break;
        }
        hasher.update(&buf[..got]);
        size += got as u64;
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

fn size_arg(text: &str) -> Result<u64, String> {
    parse_size(text)
}

fn count_arg(text: &str) -> Result<u64, String> {
    let n: u64 = text
        .parse()
        .map_err(|_| format!("需要一个正整数，收到 {text:?}"))?;
    if n == 0 {
        return Err("份数必须大于 0".to_string());
    }
    Ok(n)
}
