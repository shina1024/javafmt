Vendored upstream test resources from `google/google-java-format`.

- Sync command: `powershell -ExecutionPolicy Bypass -File scripts/sync-gjf-upstream-suite.ps1`
- Pinned version source: `tools/gjf/version.txt`
- Current runner: `cargo run -p gjf-reference -- --suite-manifest fixtures/upstream-gjf/<version>/manifest.json --report target/gjf-upstream-suite.json`

The vendored corpus includes:

- `testdata`: full-file formatter input/output pairs
- `testjavadoc`: javadoc-focused formatter input/output pairs
- `testimports`: import-mode assets kept for future dedicated runners
