use javafmt::format_str;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct SuiteManifest {
    pub version: String,
    pub repository: String,
    pub tag: String,
    pub suites: Vec<SuiteDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct SuiteDefinition {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub cases: Vec<SuiteCase>,
    #[serde(default)]
    pub assets: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SuiteCase {
    pub name: String,
    pub input: String,
    pub expected_output: String,
}

#[derive(Debug, Serialize)]
pub struct SuiteReport {
    pub version: String,
    pub repository: String,
    pub tag: String,
    pub suites: Vec<SuiteSummary>,
    pub executable_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub pending_assets: usize,
    pub pass_rate: f64,
    pub elapsed_us: u128,
}

#[derive(Debug, Serialize)]
pub struct SuiteSummary {
    pub id: String,
    pub kind: String,
    pub executable_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub pending_assets: usize,
    pub pass_rate: f64,
    pub failed_case_names: Vec<String>,
    pub note: Option<String>,
}

pub fn load_suite_manifest(path: &Path) -> Result<SuiteManifest, String> {
    let json =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("failed to parse suite manifest {}: {e}", path.display()))
}

pub fn run_suite(manifest_path: &Path) -> Result<SuiteReport, String> {
    let started = Instant::now();
    let manifest = load_suite_manifest(manifest_path)?;
    let root = manifest_path
        .parent()
        .ok_or_else(|| format!("suite manifest has no parent: {}", manifest_path.display()))?;

    let mut suites = Vec::with_capacity(manifest.suites.len());
    let mut executable_cases = 0usize;
    let mut passed_cases = 0usize;
    let mut failed_cases = 0usize;
    let mut pending_assets = 0usize;

    for suite in manifest.suites {
        if suite.kind == "full_format" {
            let mut failed_case_names = Vec::new();
            let mut passed = 0usize;

            for case in &suite.cases {
                let input = read_suite_file(root, &case.input)?;
                let expected_output = read_suite_file(root, &case.expected_output)?;
                let output = format_str(&input).output;
                if output == expected_output {
                    passed += 1;
                } else {
                    failed_case_names.push(case.name.clone());
                }
            }

            let failed = suite.cases.len().saturating_sub(passed);
            let pass_rate = if suite.cases.is_empty() {
                1.0
            } else {
                passed as f64 / suite.cases.len() as f64
            };

            executable_cases += suite.cases.len();
            passed_cases += passed;
            failed_cases += failed;

            suites.push(SuiteSummary {
                id: suite.id,
                kind: suite.kind,
                executable_cases: suite.cases.len(),
                passed_cases: passed,
                failed_cases: failed,
                pending_assets: 0,
                pass_rate,
                failed_case_names,
                note: suite.note,
            });
        } else {
            pending_assets += suite.assets.len();
            suites.push(SuiteSummary {
                id: suite.id,
                kind: suite.kind,
                executable_cases: 0,
                passed_cases: 0,
                failed_cases: 0,
                pending_assets: suite.assets.len(),
                pass_rate: 1.0,
                failed_case_names: Vec::new(),
                note: suite.note,
            });
        }
    }

    let pass_rate = if executable_cases == 0 {
        1.0
    } else {
        passed_cases as f64 / executable_cases as f64
    };

    Ok(SuiteReport {
        version: manifest.version,
        repository: manifest.repository,
        tag: manifest.tag,
        suites,
        executable_cases,
        passed_cases,
        failed_cases,
        pending_assets,
        pass_rate,
        elapsed_us: started.elapsed().as_micros(),
    })
}

fn read_suite_file(root: &Path, relative_path: &str) -> Result<String, String> {
    let path = root.join(relative_path);
    fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::run_suite;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(prefix: &str) -> Result<Self, String> {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time after unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "gjf-reference-{prefix}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&path).map_err(|e| format!("tempdir {}: {e}", path.display()))?;
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
    fn suite_report_counts_pass_fail_and_pending_assets() {
        let temp = TempDir::new("suite-report").expect("tempdir");
        let root = temp.path();

        fs::create_dir_all(root.join("testdata")).expect("create testdata");
        fs::create_dir_all(root.join("testimports")).expect("create testimports");
        fs::write(root.join("testdata").join("Pass.input"), "class Pass {}\n")
            .expect("write pass input");
        fs::write(root.join("testdata").join("Pass.output"), "class Pass {}\n")
            .expect("write pass output");
        fs::write(root.join("testdata").join("Fail.input"), "class Fail{}")
            .expect("write fail input");
        fs::write(
            root.join("testdata").join("Fail.output"),
            "class Different {}\n",
        )
        .expect("write fail output");

        let manifest = r#"{
  "version": "1.34.1",
  "repository": "google/google-java-format",
  "tag": "v1.34.1",
  "suites": [
    {
      "id": "testdata",
      "kind": "full_format",
      "cases": [
        { "name": "Pass", "input": "testdata/Pass.input", "expected_output": "testdata/Pass.output" },
        { "name": "Fail", "input": "testdata/Fail.input", "expected_output": "testdata/Fail.output" }
      ]
    },
    {
      "id": "testimports",
      "kind": "asset_only",
      "assets": ["testimports/A.input", "testimports/A.imports-only"],
      "note": "pending dedicated import-mode runner"
    }
  ]
}"#;
        fs::write(root.join("manifest.json"), manifest).expect("write manifest");

        let report = run_suite(&root.join("manifest.json")).expect("run suite");

        assert_eq!(report.executable_cases, 2);
        assert_eq!(report.passed_cases, 1);
        assert_eq!(report.failed_cases, 1);
        assert_eq!(report.pending_assets, 2);
        assert!((report.pass_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(report.suites.len(), 2);
        assert_eq!(report.suites[0].failed_case_names, vec!["Fail"]);
        assert_eq!(report.suites[1].pending_assets, 2);
    }
}
