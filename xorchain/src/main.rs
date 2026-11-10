use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

use xorchain::{decode, encode};

const BUFFER_SIZE: usize = 64 * 1024;

#[derive(Debug, Parser)]
#[command(
    name = "xorchain",
    version,
    about = "Streaming reversible prefix-XOR file encoder/decoder"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Encode a file or stream into R0 followed by the prefix-XOR chain.
    Encode {
        /// Input file, or "-" for stdin.
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        input: PathBuf,

        /// Output file, or "-" for stdout.
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        output: PathBuf,
    },

    /// Recover the original file or stream from encoded input.
    Decode {
        /// Input file, or "-" for stdin.
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        input: PathBuf,

        /// Output file, or "-" for stdout.
        #[arg(short, long, value_name = "FILE", default_value = "-")]
        output: PathBuf,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Encode { input, output } => {
            ensure_distinct(&input, &output)?;
            let mut src = open_input(&input)?;
            let mut dst = open_output(&output)?;

            encode(&mut src, &mut dst).context("failed to encode input")?;
            dst.flush().context("failed to flush output")?;
        }
        Command::Decode { input, output } => {
            ensure_distinct(&input, &output)?;
            let mut src = open_input(&input)?;
            let mut dst = open_output(&output)?;

            decode(&mut src, &mut dst).context("failed to decode input")?;
            dst.flush().context("failed to flush output")?;
        }
    }

    Ok(())
}

fn is_stdio(path: &Path) -> bool {
    path == Path::new("-")
}

fn open_input(path: &Path) -> Result<BufReader<Box<dyn Read>>> {
    let reader: Box<dyn Read> = if is_stdio(path) {
        Box::new(io::stdin())
    } else {
        Box::new(
            File::open(path)
                .with_context(|| format!("failed to open input file {}", path.display()))?,
        )
    };

    Ok(BufReader::with_capacity(BUFFER_SIZE, reader))
}

fn open_output(path: &Path) -> Result<BufWriter<Box<dyn Write>>> {
    let writer: Box<dyn Write> = if is_stdio(path) {
        Box::new(io::stdout())
    } else {
        Box::new(
            File::create(path)
                .with_context(|| format!("failed to create output file {}", path.display()))?,
        )
    };

    Ok(BufWriter::with_capacity(BUFFER_SIZE, writer))
}

/// Refuse to truncate the input by opening the same path for output.
fn ensure_distinct(input: &Path, output: &Path) -> Result<()> {
    if is_stdio(input) || is_stdio(output) {
        return Ok(());
    }

    if same_existing_file(input, output) {
        bail!(
            "input and output refer to the same file: {}",
            input.display()
        );
    }

    Ok(())
}

fn same_existing_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if let (Ok(a_meta), Ok(b_meta)) = (fs::metadata(a), fs::metadata(b)) {
            return a_meta.dev() == b_meta.dev() && a_meta.ino() == b_meta.ino();
        }
    }

    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}
