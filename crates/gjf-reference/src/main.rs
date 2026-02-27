use clap::Parser;
use javafmt::format_str;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

#[derive(Debug, Parser)]
#[command(name = "gjf-reference")]
#[command(about = "Reference comparison tool against google-java-format")]
struct Cli {
    #[arg(long, help = "Path to google-java-format jar file")]
    gjf_jar: PathBuf,
    #[arg(required = true)]
    files: Vec<PathBuf>,
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
    let mut mismatch = false;

    for file in &cli.files {
        let source = fs::read_to_string(file)
            .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
        let ours = format_str(&source).output;
        let reference = format_with_gjf(&cli.gjf_jar, &source)
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

fn format_with_gjf(gjf_jar: &PathBuf, input: &str) -> Result<String, String> {
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
