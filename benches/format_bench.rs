use javafmt::bench_support::{
    collect_benchmark_inputs, parse_positive_usize_var, resolve_benchmark_corpus_root,
};
use javafmt::format_str;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("format_bench: {err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let workspace_root = workspace_root();
    let explicit_corpus = std::env::var_os("JAVAFMT_BENCH_INPUT").map(PathBuf::from);
    let (corpus_root, used_fallback) =
        resolve_benchmark_corpus_root(&workspace_root, explicit_corpus.as_deref())
            .map_err(|err| err.to_string())?;
    let inputs = collect_benchmark_inputs(&corpus_root).map_err(|err| err.to_string())?;
    let runs = parse_positive_usize_var("JAVAFMT_BENCH_RUNS", 100)?;
    let warmup_runs = parse_positive_usize_var("JAVAFMT_BENCH_WARMUP_RUNS", 5)?;

    for _ in 0..warmup_runs {
        for input in &inputs {
            let _ = format_str(&input.source);
        }
    }

    let started = Instant::now();
    let mut changed = 0usize;
    for _ in 0..runs {
        for input in &inputs {
            changed += usize::from(format_str(&input.source).changed);
        }
    }
    let elapsed = started.elapsed();

    let files = inputs.len();
    let total_bytes = inputs.iter().map(|input| input.source.len()).sum::<usize>();
    let total_formats = files * runs;
    let total_bench_bytes = total_bytes * runs;
    let elapsed_ns = elapsed.as_nanos();
    let ns_per_file = elapsed_ns as f64 / total_formats as f64;
    let mib_per_s = if elapsed.as_secs_f64() == 0.0 {
        f64::INFINITY
    } else {
        total_bench_bytes as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
    };

    println!(
        "corpus={} fallback={} files={} total_bytes={} runs={} warmup_runs={} total_formats={} elapsed_ms={} ns_per_file={:.0} mib_per_s={:.2} changed={}",
        corpus_root.display(),
        used_fallback,
        files,
        total_bytes,
        runs,
        warmup_runs,
        total_formats,
        elapsed.as_millis(),
        ns_per_file,
        mib_per_s,
        changed
    );
    Ok(())
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}
