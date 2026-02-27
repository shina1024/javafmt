use javafmt::format_str;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[test]
fn fixture_files_are_idempotent() -> Result<(), io::Error> {
    for file in collect_fixture_files()? {
        let source = fs::read_to_string(&file)?;
        let formatted = format_str(&source).output;
        let formatted_again = format_str(&formatted).output;
        assert_eq!(
            formatted,
            formatted_again,
            "idempotence failed: {}",
            file.display()
        );
    }
    Ok(())
}

#[test]
fn fixture_files_match_current_output() -> Result<(), io::Error> {
    for file in collect_fixture_files()? {
        let source = fs::read_to_string(&file)?;
        let formatted = format_str(&source).output;
        assert_eq!(
            source,
            formatted,
            "fixture is not formatted yet: {}",
            file.display()
        );
    }
    Ok(())
}

fn collect_fixture_files() -> Result<Vec<PathBuf>, io::Error> {
    let root = workspace_root().join("fixtures").join("java");
    let mut files = Vec::new();
    collect_java_files_rec(&root, &mut files)?;
    files.sort();
    Ok(files)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
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

    if path.extension().is_some_and(|ext| ext == "java") {
        files.push(path.to_path_buf());
    }
    Ok(())
}
