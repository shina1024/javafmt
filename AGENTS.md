# AGENTS.md

## Project Goal

Build a fast Java formatter (`javafmt`) that is byte-compatible with `google-java-format` (GJF) while keeping `gofmt`-like operational simplicity.

## Current Baseline (2026-03-06)

- Java target: `25` (LTS)
- GJF target: latest stable, currently pinned to `1.34.1` (`tools/gjf/version.txt`)
- Latest successful CI run: `22767398922`
- Latest local reference gate snapshot (`target/gjf-report-local.json`):
  - `runs=3`
  - `files=79`
  - `mismatches=0`
  - `gjf_over_javafmt_ratio=2640.332695829754`

## Required Workflow For Any Change

1. Implement smallest coherent change set with regression tests.
2. Run:
   - `cargo fmt --all`
   - `cargo test --workspace --locked`
   - `cargo run -p gjf-reference -- --runs 3 --max-mismatches 0 --min-gjf-over-javafmt 1.10 --report target/gjf-report-local.json fixtures/java`
3. If any mismatch appears, inspect and either:
   - fix formatter logic, or
   - drop unstable/non-idempotent fixture inputs from gate corpus.
4. Commit, push, and verify GitHub Actions `CI` (`test` + `reference`) are green.

## Compatibility Rules

- Target is byte-equivalent output against pinned GJF.
- Do not add style knobs that diverge from GJF behavior.
- Keep output deterministic and idempotent on project fixtures.
- Only add fixture files that are stable and reproducible.

## Performance Rules

- Preserve fast-path behavior (single-pass, low-allocation heuristics).
- Avoid compatibility patches that add repeated full rescans in hot path.
- Do not accept changes that break the CI speed gate (`--min-gjf-over-javafmt 1.10`).

## Fixture Policy

- Gate corpus is under `fixtures/java`.
- Add new probe cases only after they show stable expected behavior.
- Known non-idempotent GJF edge case exists around specific `do { /*...*/ ... } while (...)` comment layouts; avoid promoting such cases into gate fixtures.

## Import Ordering Policy

- GJF-compatible import ordering is implemented in `crates/javafmt/src/lib.rs` as a post-processing pass:
  - static imports first
  - lexical sort within static and non-static groups
  - one blank line between groups when both exist
- If import block includes comments, skip reordering for safety.

## Dependency Policy

- Any newly added dependency must use the latest stable version at introduction time.
- Keep `Cargo.lock` up to date with dependency changes.

## Commit Convention

- Use concise conventional-style messages, e.g. `feat: ...`, `fix: ...`, `refactor: ...`.
- Each commit should preserve green local gates.
