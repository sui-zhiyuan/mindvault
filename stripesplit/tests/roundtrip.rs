use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use sha2::{Digest, Sha256};
use stripesplit::{
    merge, part_count_for_size, part_len, split, Index, MergeOptions, SplitOptions, INDEX_FILE,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("时钟早于 UNIX_EPOCH")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "stripesplit-test-{}-{tag}-{nanos}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("创建临时目录失败");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// 可复现的伪随机数据，避免引入 rand 依赖。
fn payload(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..len {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        out.push((state >> 33) as u8);
    }
    out
}

fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn do_split(dir: &Path, name: &str, data: &[u8], parts: u64) -> (PathBuf, PathBuf, Index) {
    let src = dir.join(name);
    fs::write(&src, data).expect("写源文件失败");
    let outdir = dir.join("parts");
    let index = split(
        &src,
        &outdir,
        &SplitOptions {
            parts,
            force: false,
        },
    )
    .expect("分割失败");
    (src, outdir, index)
}

#[test]
fn roundtrip_matrix() {
    let cases: [(usize, u64); 10] = [
        (0, 1),
        (1, 1),
        (1, 4),
        (3, 8),
        (10, 3),
        (10, 4),
        (100, 100),
        (1000, 7),
        // 100_000 字节 / 500 份：每轮 134 字节/份，需要多轮，覆盖循环边界。
        (100_000, 500),
        (1_048_576 + 13, 3),
    ];

    for (len, n) in cases {
        let dir = TempDir::new(&format!("rt-{len}-{n}"));
        let data = payload(len);
        let (src, outdir, index) = do_split(dir.path(), "input.bin", &data, n);

        assert_eq!(index.original_size, len as u64);
        assert_eq!(index.original_sha256, sha256(&data));
        assert_eq!(index.part_count, n);
        assert_eq!(index.parts.len() as u64, n);

        let mut sum = 0u64;
        for (j, part) in index.parts.iter().enumerate() {
            let expected_len = part_len(len as u64, n, j as u64);
            assert_eq!(part.size, expected_len, "len={len} n={n} part={j}");

            let bytes = fs::read(outdir.join(&part.name)).expect("读分片失败");
            assert_eq!(bytes.len() as u64, expected_len);
            assert_eq!(part.sha256, sha256(&bytes), "len={len} n={n} part={j}");

            let expected: Vec<u8> = data.iter().skip(j).step_by(n as usize).copied().collect();
            assert_eq!(bytes, expected, "len={len} n={n} part={j} 内容不符");
            sum += expected_len;
        }
        assert_eq!(sum, len as u64);

        let restored = dir.path().join("restored.bin");
        merge(&index, &outdir, &restored, &MergeOptions { force: false }).expect("合并失败");
        assert_eq!(fs::read(&restored).expect("读输出失败"), data);
        assert_eq!(fs::read(&src).expect("读源文件失败"), data);
    }
}

#[test]
fn index_toml_contains_required_fields() {
    let dir = TempDir::new("index-fields");
    let data = payload(64);
    let (_, outdir, _) = do_split(dir.path(), "a.bin", &data, 4);

    let text = fs::read_to_string(outdir.join(INDEX_FILE)).expect("读索引失败");
    for needle in [
        "version = 1",
        "algorithm = \"byte-interleave\"",
        "original_name = \"a.bin\"",
        "original_size = 64",
        "part_count = 4",
        "[[parts]]",
        "name = \"part0001\"",
        "size = 16",
        "sha256 = \"",
    ] {
        assert!(text.contains(needle), "索引缺少 {needle:?}:\n{text}");
    }

    let (loaded, loaded_dir) = Index::load(&outdir).expect("加载索引失败");
    assert_eq!(loaded_dir, outdir);
    assert_eq!(
        loaded,
        Index::load(&outdir.join(INDEX_FILE))
            .expect("按文件加载失败")
            .0
    );
}

#[test]
fn merge_detects_corrupt_part() {
    let dir = TempDir::new("corrupt");
    let data = payload(4096);
    let (_, outdir, index) = do_split(dir.path(), "a.bin", &data, 4);

    let victim = outdir.join(&index.parts[1].name);
    let mut bytes = fs::read(&victim).expect("读分片失败");
    bytes[0] ^= 0xff;
    fs::write(&victim, &bytes).expect("写分片失败");

    let out = dir.path().join("out.bin");
    let err = merge(&index, &outdir, &out, &MergeOptions { force: false })
        .expect_err("损坏的分片不应合并成功");
    assert!(
        format!("{err:#}").contains("校验失败"),
        "错误信息应指出校验失败：{err:#}"
    );
    assert!(!out.exists(), "失败时应删除输出文件");
}

#[test]
fn merge_detects_truncated_part() {
    let dir = TempDir::new("truncated");
    let data = payload(4096);
    let (_, outdir, index) = do_split(dir.path(), "a.bin", &data, 4);

    let victim = outdir.join(&index.parts[2].name);
    let mut bytes = fs::read(&victim).expect("读分片失败");
    bytes.pop();
    fs::write(&victim, &bytes).expect("写分片失败");

    let out = dir.path().join("out.bin");
    let err = merge(&index, &outdir, &out, &MergeOptions { force: false })
        .expect_err("缺字节的分片不应合并成功");
    assert!(
        format!("{err:#}").contains("大小不符"),
        "错误信息应指出大小不符：{err:#}"
    );
    assert!(!out.exists());
}

#[test]
fn merge_detects_missing_part() {
    let dir = TempDir::new("missing");
    let data = payload(1024);
    let (_, outdir, index) = do_split(dir.path(), "a.bin", &data, 3);
    fs::remove_file(outdir.join(&index.parts[1].name)).expect("删分片失败");

    let err = merge(
        &index,
        &outdir,
        &dir.path().join("out.bin"),
        &MergeOptions { force: false },
    )
    .expect_err("缺分片不应合并成功");
    assert!(format!("{err:#}").contains("找不到分片"), "{err:#}");
}

#[test]
fn split_and_merge_respect_existing_files() {
    let dir = TempDir::new("force");
    let data = payload(1000);
    let (src, outdir, _) = do_split(dir.path(), "a.bin", &data, 2);

    let err = split(
        &src,
        &outdir,
        &SplitOptions {
            parts: 2,
            force: false,
        },
    )
    .expect_err("重复分割应失败");
    assert!(format!("{err:#}").contains("已存在"), "{err:#}");

    let again = split(
        &src,
        &outdir,
        &SplitOptions {
            parts: 2,
            force: true,
        },
    )
    .expect("加 force 后应可覆盖");
    assert_eq!(again.original_sha256, sha256(&data));

    let out = dir.path().join("out.bin");
    fs::write(&out, b"occupied").expect("写占位文件失败");
    let err = merge(&again, &outdir, &out, &MergeOptions { force: false })
        .expect_err("输出已存在且未加 force 应失败");
    assert!(format!("{err:#}").contains("已存在"), "{err:#}");

    merge(&again, &outdir, &out, &MergeOptions { force: true }).expect("加 force 后应可覆盖");
    assert_eq!(fs::read(&out).expect("读输出失败").len(), data.len());
}

#[test]
fn index_rejects_unsafe_part_names() {
    let dir = TempDir::new("unsafe");
    let path = dir.path().join(INDEX_FILE);
    let text = format!(
        "version = 1\ntool = \"stripesplit\"\nalgorithm = \"byte-interleave\"\n\
         original_name = \"a.bin\"\noriginal_size = 4\n\
         original_sha256 = \"{}\"\npart_count = 1\n\
         [[parts]]\nname = \"../evil\"\nsize = 4\nsha256 = \"{}\"\n",
        "0".repeat(64),
        "0".repeat(64)
    );
    fs::write(&path, text).expect("写索引失败");
    let err = Index::load(&path).expect_err("不安全的文件名应被拒绝");
    assert!(format!("{err:#}").contains("不合法"), "{err:#}");
}

#[test]
fn part_count_for_size_never_exceeds_target() {
    for total in [0u64, 1, 999, 1000, 1_000_001, 10_485_760] {
        for target in [1u64, 100, 1_048_576, 4_194_304] {
            let n = part_count_for_size(total, target);
            assert!(n >= 1);
            for i in 0..n.min(50) {
                assert!(part_len(total, n, i) <= target.max(1));
            }
            let sum: u64 = (0..n).map(|i| part_len(total, n, i)).sum();
            assert_eq!(sum, total, "total={total} target={target} n={n}");
        }
    }
}

#[test]
fn cli_split_verify_merge() {
    let dir = TempDir::new("cli");
    let data = payload(300_000);
    let src = dir.path().join("big.bin");
    fs::write(&src, &data).expect("写源文件失败");
    let outdir = dir.path().join("parts");
    let restored = dir.path().join("restored.bin");
    let exe = env!("CARGO_BIN_EXE_stripesplit");

    let split = Command::new(exe)
        .arg("split")
        .arg(&src)
        .arg("-o")
        .arg(&outdir)
        .arg("-s")
        .arg("100K")
        .output()
        .expect("运行 split 失败");
    assert!(
        split.status.success(),
        "split 失败：{}",
        String::from_utf8_lossy(&split.stderr)
    );
    let stdout = String::from_utf8_lossy(&split.stdout);
    assert!(stdout.contains("分割"), "stdout:\n{stdout}");
    assert!(outdir.join(INDEX_FILE).is_file());

    let verify = Command::new(exe)
        .arg("verify")
        .arg(&outdir)
        .output()
        .expect("运行 verify 失败");
    assert!(
        verify.status.success(),
        "verify 失败：{}",
        String::from_utf8_lossy(&verify.stderr)
    );

    let merge = Command::new(exe)
        .arg("merge")
        .arg(&outdir)
        .arg("-o")
        .arg(&restored)
        .output()
        .expect("运行 merge 失败");
    assert!(
        merge.status.success(),
        "merge 失败：{}",
        String::from_utf8_lossy(&merge.stderr)
    );
    assert_eq!(fs::read(&restored).expect("读输出失败"), data);

    let bad = Command::new(exe)
        .arg("merge")
        .arg(&outdir)
        .arg("-o")
        .arg(&restored)
        .output()
        .expect("运行 merge 失败");
    assert!(!bad.status.success(), "输出已存在时应失败");
}

#[test]
fn defaults_to_one_mib_parts() {
    let dir = TempDir::new("default");
    let data = payload(2 * 1_048_576 + 7);
    let (_, outdir, index) = do_split(
        dir.path(),
        "a.bin",
        &data,
        part_count_for_size(2 * 1_048_576 + 7, 1_048_576),
    );
    assert_eq!(index.part_count, 3);
    assert!(index.parts.iter().all(|p| p.size <= 1_048_576));
    for part in &index.parts {
        assert!(outdir.join(&part.name).is_file());
    }
}

#[test]
fn part_files_use_fixed_part_naming() {
    let dir = TempDir::new("naming");
    let data = payload(10);
    let (_, outdir, index) = do_split(dir.path(), "a.bin", &data, 4);

    let names: Vec<&str> = index.parts.iter().map(|part| part.name.as_str()).collect();
    assert_eq!(names, ["part0001", "part0002", "part0003", "part0004"]);
    for name in &names {
        assert!(outdir.join(name).is_file(), "{name} 不存在");
        assert!(!name.contains("a.bin"), "分片名不应带源文件名：{name}");
    }
}

#[test]
fn refuses_to_overwrite_source_named_like_a_part() {
    let dir = TempDir::new("src-name");
    let src = dir.path().join("part0001");
    fs::write(&src, b"hello world").expect("写源文件失败");

    let err = split(
        &src,
        dir.path(),
        &SplitOptions {
            parts: 3,
            force: true,
        },
    )
    .expect_err("源文件名与分片名冲突时应拒绝");
    assert!(format!("{err:#}").contains("覆盖源文件"), "{err:#}");
    assert_eq!(fs::read(&src).expect("读源文件失败"), b"hello world");
}
