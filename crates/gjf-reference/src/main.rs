use clap::Parser;
use javafmt::format_str;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

#[derive(Debug, Parser)]
#[command(name = "gjf-reference")]
#[command(about = "Reference comparison tool against google-java-format")]
struct Cli {
    #[arg(long, help = "Path to google-java-format jar file")]
    gjf_jar: Option<PathBuf>,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("gjf-reference: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    let gjf_jar = resolve_gjf_jar(cli.gjf_jar)?;
    let files = collect_java_files(&cli.inputs)?;
    let mut mismatch = false;

    for file in &files {
        let source = fs::read_to_string(file)
            .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
        let ours = format_str(&source).output;
        let reference = format_with_gjf(&gjf_jar, &source)
            .map_err(|e| format!("failed to run GJF for {}: {e}", file.display()))?;
        if ours != reference {
            mismatch = true;
            println!("mismatch: {}", file.display());
        }
    }

    if mismatch {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn resolve_gjf_jar(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path);
        }
        return Err(format!("GJF jar not found: {}", path.display()));
    }

    let version_file = PathBuf::from("tools/gjf/version.txt");
    let version = fs::read_to_string(&version_file)
        .map_err(|e| format!("failed to read {}: {e}", version_file.display()))?;
    let version = version.trim();
    if version.is_empty() || version == "latest" {
        return Err(String::from(
            "tools/gjf/version.txt must contain a resolved version; run scripts/update-gjf.ps1",
        ));
    }

    let jar = PathBuf::from(format!(
        "tools/gjf/google-java-format-{version}-all-deps.jar"
    ));
    if jar.exists() {
        Ok(jar)
    } else {
        Err(format!("GJF jar not found: {}", jar.display()))
    }
}

fn collect_java_files(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        collect_java_files_rec(input, &mut files)?;
    }
    files.sort();
    files.dedup();

    if files.is_empty() {
        return Err(String::from("no Java files found in inputs"));
    }
    Ok(files)
}

fn collect_java_files_rec(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        entries.sort();
        for entry in entries {
            collect_java_files_rec(&entry, files)?;
        }
        return Ok(());
    }

    if path.extension().is_some_and(|ext| ext == "java") {
        files.push(path.to_path_buf());
    }
    Ok(())
}

fn format_with_gjf(gjf_jar: &Path, input: &str) -> Result<String, String> {
    let mut child = Command::new("java")
        .arg("-jar")
        .arg(gjf_jar)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| String::from("missing stdin"))?;
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("stdin write failed: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("wait failed: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("process failed: {stderr}"));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("non-utf8 output: {e}"))
}
