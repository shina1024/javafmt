use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkInput {
    pub path: PathBuf,
    pub source: String,
}

pub fn resolve_benchmark_corpus_root(
    workspace_root: &Path,
    explicit: Option<&Path>,
) -> Result<(PathBuf, bool), io::Error> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok((path.to_path_buf(), false));
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("benchmark corpus not found: {}", path.display()),
        ));
    }

    let preferred = workspace_root.join("benches").join("corpus");
    if contains_java_files(&preferred)? {
        return Ok((preferred, false));
    }

    let fallback = workspace_root.join("fixtures").join("java");
    if contains_java_files(&fallback)? {
        return Ok((fallback, true));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "no benchmark corpus found in {} or {}",
            preferred.display(),
            fallback.display()
        ),
    ))
}

pub fn collect_benchmark_inputs(root: &Path) -> Result<Vec<BenchmarkInput>, io::Error> {
    let mut files = Vec::new();
    collect_java_files_rec(root, &mut files)?;
    files.sort();

    let mut inputs = Vec::with_capacity(files.len());
    for path in files {
        inputs.push(BenchmarkInput {
            source: fs::read_to_string(&path)?,
            path,
        });
    }
    Ok(inputs)
}

pub fn parse_positive_usize_var(name: &str, default: usize) -> Result<usize, String> {
    match std::env::var(name) {
        Ok(raw) => parse_positive_usize(name, raw.trim()),
        Err(std::env::VarError::NotPresent) => Ok(default),
        Err(std::env::VarError::NotUnicode(_)) => Err(format!("{name} must be valid UTF-8")),
    }
}

fn parse_positive_usize(name: &str, raw: &str) -> Result<usize, String> {
    let value = raw
        .parse::<usize>()
        .map_err(|_| format!("{name} must be a positive integer, got {raw:?}"))?;
    if value == 0 {
        return Err(format!("{name} must be >= 1"));
    }
    Ok(value)
}

fn contains_java_files(path: &Path) -> Result<bool, io::Error> {
    if !path.exists() {
        return Ok(false);
    }
    if path.is_file() {
        return Ok(is_java_file(path));
    }

    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    for entry in entries {
        if contains_java_files(&entry)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn collect_java_files_rec(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
    if path.is_file() {
        if is_java_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    let mut entries = fs::read_dir(path)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, io::Error>>()?;
    entries.sort();

    for entry in entries {
        collect_java_files_rec(&entry, files)?;
    }
    Ok(())
}

fn is_java_file(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("java"))
}

#[cfg(test)]
mod tests {
    use super::{collect_benchmark_inputs, parse_positive_usize, resolve_benchmark_corpus_root};
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Result<Self, io::Error> {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("javafmt-{prefix}-{}-{unique}", std::process::id()));
            fs::create_dir_all(&path)?;
            Ok(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn uses_bench_corpus_when_java_files_exist() -> Result<(), io::Error> {
        let temp = TempDir::new("bench-root")?;
        let bench_file = temp.path().join("benches").join("corpus").join("A.java");
        fs::create_dir_all(bench_file.parent().expect("bench parent"))?;
        fs::write(&bench_file, "class A {}\n")?;
        fs::create_dir_all(temp.path().join("fixtures").join("java"))?;

        let (root, used_fallback) = resolve_benchmark_corpus_root(temp.path(), None)?;

        assert_eq!(root, temp.path().join("benches").join("corpus"));
        assert!(!used_fallback);
        Ok(())
    }

    #[test]
    fn falls_back_to_fixtures_when_bench_corpus_is_empty() -> Result<(), io::Error> {
        let temp = TempDir::new("fixture-fallback")?;
        fs::create_dir_all(temp.path().join("benches").join("corpus"))?;
        let fixture_file = temp.path().join("fixtures").join("java").join("B.java");
        fs::create_dir_all(fixture_file.parent().expect("fixture parent"))?;
        fs::write(&fixture_file, "class B {}\n")?;

        let (root, used_fallback) = resolve_benchmark_corpus_root(temp.path(), None)?;

        assert_eq!(root, temp.path().join("fixtures").join("java"));
        assert!(used_fallback);
        Ok(())
    }

    #[test]
    fn collects_java_inputs_recursively_in_sorted_order() -> Result<(), io::Error> {
        let temp = TempDir::new("collect-inputs")?;
        let root = temp.path().join("fixtures").join("java");
        fs::create_dir_all(root.join("nested"))?;
        fs::write(root.join("Z.txt"), "skip")?;
        fs::write(root.join("nested").join("B.java"), "class B {}\n")?;
        fs::write(root.join("A.java"), "class A {}\n")?;

        let inputs = collect_benchmark_inputs(&root)?;
        let names = inputs
            .iter()
            .map(|input| {
                input
                    .path
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["A.java", "B.java"]);
        Ok(())
    }

    #[test]
    fn rejects_zero_for_positive_usize() {
        assert_eq!(
            parse_positive_usize("JAVAFMT_BENCH_RUNS", "0"),
            Err(String::from("JAVAFMT_BENCH_RUNS must be >= 1"))
        );
    }
}
