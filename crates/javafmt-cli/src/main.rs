use clap::{ArgGroup, Parser};
use javafmt::format_str;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(name = "javafmt")]
#[command(about = "Fast google-java-format compatible formatter (work in progress)")]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["check", "write"]),
))]
struct Cli {
    #[arg(long, help = "Check if files would be reformatted")]
    check: bool,
    #[arg(short = 'w', long = "write", help = "Write result to source files")]
    write: bool,
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("javafmt: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, io::Error> {
    let cli = Cli::parse();
    let mut has_diff = false;

    for file in &cli.files {
        let source = fs::read_to_string(file)?;
        let result = format_str(&source);
        if !result.changed {
            continue;
        }

        has_diff = true;
        if cli.write {
            fs::write(file, result.output)?;
            println!("formatted {}", file.display());
        } else if cli.check {
            println!("{}", file.display());
        }
    }

    if cli.check && has_diff {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

#[allow(dead_code)]
fn _is_java_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "java")
}
