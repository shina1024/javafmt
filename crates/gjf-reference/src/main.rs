use clap::Parser;
use javafmt::format_str;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Instant;

#[derive(Debug, Parser)]
#[command(name = "gjf-reference")]
#[command(about = "Reference comparison tool against google-java-format")]
struct Cli {
    #[arg(long, help = "Path to google-java-format jar file")]
    gjf_jar: Option<PathBuf>,
    #[arg(
        long,
        help = "Write JSON report (checked files, mismatch count, mismatch files)"
    )]
    report: Option<PathBuf>,
    #[arg(long, default_value_t = 1, help = "Number of repeated runs")]
    runs: usize,
    #[arg(
        long,
        default_value_t = 0,
        help = "Fail when mismatches exceed this value"
    )]
    max_mismatches: usize,
    #[arg(
        long,
        help = "Fail when (GJF elapsed / javafmt elapsed) is below this value"
    )]
    min_gjf_over_javafmt: Option<f64>,
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
struct ComparisonReport {
    runs: usize,
    files: usize,
    comparisons: usize,
    mismatches: usize,
    mismatch_files: Vec<String>,
    javafmt_elapsed_us: u128,
    gjf_elapsed_us: u128,
    javafmt_avg_us_per_file: f64,
    gjf_avg_us_per_file: f64,
    gjf_over_javafmt_ratio: f64,
    elapsed_us: u128,
}

#[derive(Debug)]
struct SourceFile {
    path: PathBuf,
    source: String,
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
    if cli.runs == 0 {
        return Err(String::from("--runs must be >= 1"));
    }

    let started = Instant::now();
    let gjf_jar = resolve_gjf_jar(cli.gjf_jar)?;
    let files = collect_java_files(&cli.inputs)?;
    let sources = load_sources(&files)?;

    let mut mismatch_files = BTreeSet::new();
    let mut javafmt_elapsed_ns = 0u128;
    let mut gjf_elapsed_ns = 0u128;

    for run_index in 0..cli.runs {
        for item in &sources {
            let ours_started = Instant::now();
            let ours = format_str(&item.source).output;
            javafmt_elapsed_ns += ours_started.elapsed().as_nanos();

            let gjf_started = Instant::now();
            let reference = format_with_gjf(&gjf_jar, &item.source)
                .map_err(|e| format!("failed to run GJF for {}: {e}", item.path.display()))?;
            gjf_elapsed_ns += gjf_started.elapsed().as_nanos();

            if ours != reference {
                if mismatch_files.insert(item.path.display().to_string()) {
                    println!("mismatch: {}", item.path.display());
                } else {
                    println!("mismatch(run={}): {}", run_index + 1, item.path.display());
                }
            }
        }
    }

    let comparisons = sources.len() * cli.runs;
    let javafmt_elapsed_us = javafmt_elapsed_ns / 1_000;
    let gjf_elapsed_us = gjf_elapsed_ns / 1_000;
    let javafmt_avg_us_per_file = javafmt_elapsed_us as f64 / comparisons as f64;
    let gjf_avg_us_per_file = gjf_elapsed_us as f64 / comparisons as f64;
    let gjf_over_javafmt_ratio = if javafmt_elapsed_ns == 0 {
        1_000_000.0
    } else {
        gjf_elapsed_ns as f64 / javafmt_elapsed_ns as f64
    };

    let report = ComparisonReport {
        runs: cli.runs,
        files: sources.len(),
        comparisons,
        mismatches: mismatch_files.len(),
        mismatch_files: mismatch_files.into_iter().collect::<Vec<_>>(),
        javafmt_elapsed_us,
        gjf_elapsed_us,
        javafmt_avg_us_per_file,
        gjf_avg_us_per_file,
        gjf_over_javafmt_ratio,
        elapsed_us: started.elapsed().as_micros(),
    };

    println!(
        "runs={} files={} mismatches={} javafmt_us={} gjf_us={} ratio={:.3} elapsed_us={}",
        report.runs,
        report.files,
        report.mismatches,
        report.javafmt_elapsed_us,
        report.gjf_elapsed_us,
        report.gjf_over_javafmt_ratio,
        report.elapsed_us
    );

    if let Some(report_path) = cli.report {
        write_report(&report_path, &report)?;
        println!("report: {}", report_path.display());
    }

    if report.mismatches > cli.max_mismatches {
        eprintln!(
            "gate failed: mismatches={} > max_mismatches={}",
            report.mismatches, cli.max_mismatches
        );
        return Ok(ExitCode::from(1));
    }

    if let Some(min_ratio) = cli.min_gjf_over_javafmt {
        if report.gjf_over_javafmt_ratio < min_ratio {
            eprintln!(
                "gate failed: gjf_over_javafmt_ratio={:.3} < min_gjf_over_javafmt={:.3}",
                report.gjf_over_javafmt_ratio, min_ratio
            );
            return Ok(ExitCode::from(1));
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn load_sources(files: &[PathBuf]) -> Result<Vec<SourceFile>, String> {
    let mut sources = Vec::with_capacity(files.len());
    for file in files {
        let source = fs::read_to_string(file)
            .map_err(|e| format!("failed to read {}: {e}", file.display()))?;
        sources.push(SourceFile {
            path: file.clone(),
            source,
        });
    }
    Ok(sources)
}

fn write_report(path: &Path, report: &ComparisonReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    let json = serde_json::to_string_pretty(report).map_err(|e| format!("json: {e}"))?;
    fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))
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
