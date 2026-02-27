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
    run_with_mode(cli.write, &cli.files)
}

fn run_with_mode(write: bool, inputs: &[PathBuf]) -> Result<ExitCode, io::Error> {
    let files = collect_java_files(inputs)?;
    let mut has_diff = false;

    for file in &files {
        let source = fs::read_to_string(file)?;
        let result = format_str(&source);
        if !result.changed {
            continue;
        }

        has_diff = true;
        if write {
            fs::write(file, result.output)?;
            println!("formatted {}", file.display());
        } else {
            println!("{}", file.display());
        }
    }

    if !write && has_diff {
        return Ok(ExitCode::from(1));
    }
    Ok(ExitCode::SUCCESS)
}

fn collect_java_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, io::Error> {
    let mut files = Vec::new();
    for input in inputs {
        collect_java_files_rec(input, &mut files)?;
    }
    files.sort();
    files.dedup();

    if files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no Java files found in inputs",
        ));
    }
    Ok(files)
}

fn collect_java_files_rec(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
    let metadata = fs::metadata(path)?;
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, io::Error>>()?;
        entries.sort();
        for entry in entries {
            collect_java_files_rec(&entry, files)?;
        }
        return Ok(());
    }

    if is_java_file(path) {
        files.push(path.to_path_buf());
    }
    Ok(())
}

fn is_java_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "java")
}

#[cfg(test)]
mod tests {
    use super::run_with_mode;
    use std::fs;
    use std::path::PathBuf;
    use std::process::ExitCode;

    #[test]
    fn check_returns_non_zero_when_diff_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("A.java");
        fs::write(&file, "class A {}").expect("write");

        let result = run_with_mode(false, &[PathBuf::from(temp.path())]).expect("run");
        assert_eq!(result, ExitCode::from(1));
        let content = fs::read_to_string(&file).expect("read");
        assert_eq!(content, "class A {}");
    }

    #[test]
    fn write_mode_formats_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let file = temp.path().join("B.java");
        fs::write(&file, "class B {}").expect("write");

        let result = run_with_mode(true, &[PathBuf::from(temp.path())]).expect("run");
        assert_eq!(result, ExitCode::SUCCESS);
        let content = fs::read_to_string(&file).expect("read");
        assert_eq!(content, "class B {}\n");
    }
}
