# javafmt Implementation Plan

## 1. Product Goal

Build `javafmt`, a fast Java formatter that is compatible with `google-java-format` (GJF) output, with ergonomics similar to `gofmt`/`oxfmt`:

- deterministic output
- zero style configuration for core mode
- simple CLI (`stdin`, files, `-w`, `--check`)
- high throughput on large repositories

## 2. Compatibility Contract

Compatibility must be explicit. We define three levels:

1. **Byte-equivalent mode (target)**  
   For supported Java syntax, output bytes exactly match GJF for the same input and line width.
2. **Token-equivalent mode (fallback during development)**  
   Whitespace differs, but token stream remains identical.
3. **Unsupported input behavior**  
   Parse errors and unsupported syntax produce stable, documented errors.

Initial implementation should target full-file formatting first. Line-range formatting can be added after core compatibility is stable.

## 3. Non-Goals (Initial Phase)

- custom style profiles
- IDE incremental formatting protocol
- semantic refactoring
- semantic import cleanup or code cleanup beyond GJF-compatible formatting behavior

## 4. System Architecture

Formatter pipeline:

1. **Input Normalization**
   - detect line endings (`LF`, `CRLF`)
   - preserve trailing newline policy
   - create source map for byte/line conversion
2. **Lexer**
   - fast tokenization over UTF-8 bytes
   - preserve comments and whitespace anchors
3. **Parser / Concrete Syntax Tree (CST)**
   - parse Java grammar for targeted language versions
   - retain enough structure to reproduce comment attachment and break decisions
4. **Attachment Phase**
   - attach comments to nearest stable syntax nodes
   - classify comments (`leading`, `trailing`, `dangling`)
5. **Format IR Builder**
   - transform CST into a compact doc IR (groups, indents, soft/hard breaks)
   - model GJF behavior for wrapping and continuation indent
6. **Line-Break Solver / Printer**
   - choose breakpoints under max column constraint
   - stable deterministic tie-breaking
7. **Emitter**
   - emit formatted text with selected line ending mode
   - validate token-equivalence in debug/test mode

### 4.1 Dual-Path Execution Model

- **Fast path (default):** token-driven single-pass formatting for unambiguous/common syntax.
- **Structured path (fallback):** parser-assisted formatting for ambiguous/high-risk syntax constructs.
- **Dispatch rule:** choose fast path first, switch to structured path only for constructs known to cause ambiguity.
- **Constraint:** fallback must reuse existing lex/parse artifacts and must not re-lex/re-parse full files repeatedly.

## 5. Design Principles for Speed

- single-pass or near-single-pass stages in hot paths
- no semantic/type resolution in the formatting hot path (syntax + comments only)
- arena allocation for short-lived syntax objects
- compact token representation (integers/enums over heap-heavy objects)
- minimize string allocations (slice-based references)
- pre-allocate buffers from file-size heuristics where possible
- avoid regex in hot loops
- parallelize across files, not within a single file formatting operation
- keep parallelism bounded by memory budget and preserve deterministic output order
- warm-cache friendly data layout
- hard complexity target: O(n) with respect to input bytes/tokens for the common path
- no per-file unbounded backtracking or repeated full-token rescans in hot loops
- compatibility fixes must preserve the fast-path eligibility rate for common code patterns

## 6. GJF Compatibility Strategy

Use differential testing against a pinned GJF version:

- format with `javafmt` and GJF
- compare exact bytes
- classify mismatches by syntax category
- maintain a known-differences list only temporarily

Latest-follow policy for GJF:

- treat latest stable GJF as target behavior
- update pinned reference version on a regular cadence (or immediately when needed)
- record the exact pinned version in CI to keep test results reproducible
- current pinned reference version: `1.34.1` (`tools/gjf/version.txt`)

Prioritize compatibility in this order:

1. whitespace and indentation in blocks/statements
2. method chains and argument wrapping
3. annotations and modifiers
4. lambdas, generics, and complex expressions
5. comment placement edge cases

## 7. Testing Strategy

### 7.1 Golden Tests

- input fixture -> expected output fixture
- category-based directories for syntax constructs
- stable snapshot updates with explicit review

### 7.2 Differential Tests

- run against GJF on curated corpus
- report mismatch summary and sampled diffs
- emit machine-readable JSON reports for CI trend tracking
- publish reference reports as CI artifacts for baseline comparisons
- include per-run timing fields for `javafmt` and GJF in reference reports
- fail CI when mismatch rate exceeds threshold

### 7.3 Property/Safety Tests

- idempotence: `fmt(fmt(x)) == fmt(x)`
- token preservation: token stream unchanged (excluding whitespace/comments policy)
- panic-free fuzzing for parser/formatter

### 7.4 Performance Benchmarks

- small/medium/large Java files
- cold and warm runs
- files/sec and MB/sec metrics
- memory peak tracking
- repeatable reference timing collection (`scripts/bench-reference.ps1`)
- gate-oriented reference checks with repeated runs (`gjf-reference --runs`)

## 8. CLI Plan (gofmt-like UX)

Initial v0 commands (minimal UI):

- `javafmt -w <file...>` (format and write in place)
- `javafmt --check <file...>` (non-zero exit on changes)

Future options (after core stability):

- `javafmt <file...>` (stdout mode)
- `javafmt -` (stdin -> stdout)
- `--diff`
- `--line-width` (if compatibility mode allows controlled variance)
- `--threads`

## 9. Phased Delivery Plan

### Phase 0: Spec and Harness

Deliverables:

- compatibility contract document (this file)
- pinned GJF reference formatter setup for tests
- fixture layout and benchmark harness skeleton

Exit criteria:

- can run differential comparison tool end-to-end on sample corpus

### Phase 1: Core Parser + Basic Formatter

Deliverables:

- parse major declarations/statements/expressions
- emit deterministic formatted output for common Java
- pass idempotence tests on base fixtures

Exit criteria:

- stable formatting for representative production-style files

### Phase 2: Compatibility Expansion

Deliverables:

- close high-frequency diffs vs GJF
- robust comment attachment and wrapping behavior
- broaden Java syntax coverage

Exit criteria:

- high compatibility on curated corpus (target >= 99% files byte-equivalent)

### Phase 3: Performance Optimization

Deliverables:

- profiling-driven hot-path improvements
- parallel file formatting
- reduced allocations and improved throughput

Exit criteria:

- measurable speedup vs GJF baseline on agreed benchmarks

### Phase 4: Hardening and Release

Deliverables:

- CI quality gates (compatibility + performance + fuzz)
- release process and versioning policy
- documentation for users and contributors

Exit criteria:

- release candidate with stable CLI and known limitations documented

## 10. Observability and Quality Gates

CI should include:

- unit tests
- golden tests
- differential tests against pinned GJF
- benchmark smoke check (regression threshold)
- lint and formatting checks for this repository

Suggested hard gate examples:

- idempotence failures: 0
- crash/panic count: 0
- compatibility rate: >= target for current milestone
- performance regression: <= agreed threshold
- memory regression: <= agreed threshold
- CI reference gate: `mismatches <= 0` on reference corpus
- CI speed gate: `gjf_over_javafmt_ratio >= 1.10` with repeated runs on reference corpus
- CI policy: compatibility improvements that violate speed gate are rejected until optimized

## 11. Risks and Mitigations

1. **Grammar drift with new Java versions**  
   Mitigation: explicit version matrix and staged parser updates.
2. **Comment placement complexity**  
   Mitigation: dedicated attachment model + focused fixtures for edge cases.
3. **Compatibility tail is expensive**  
   Mitigation: rank diffs by frequency/impact and resolve highest leverage first.
4. **Performance regressions during compatibility fixes**  
   Mitigation: enforce benchmark checks in CI with historical baseline.

## 12. Ready-to-Code Checklist

Before major implementation starts, confirm all are true:

- [ ] compatibility target and scope are accepted
- [ ] pinned GJF reference version is decided
- [ ] fixture corpus sources are selected
- [ ] benchmark scenarios and success metrics are agreed
- [ ] phase exit criteria are accepted

When this checklist is complete, implementation can begin with Phase 0 tasks.

## 13. Decision Status Before Phase 0 Completion

Decided:

1. Java language coverage target for v0: Java 25 LTS.
2. GJF policy: track latest stable `google-java-format`.
3. Initial CLI surface: minimal UI (`--check` and `-w` first).

Pending (decide after baseline measurement in Phase 0):

1. Acceptable compatibility floor per milestone (file-level and byte-level).
2. Performance target definition (`x` times faster than GJF on specified corpus/hardware).

## 14. Performance Hypotheses to Validate Early

1. Parse + print only (no semantic analysis) is the primary latency driver reduction.
2. Allocation reduction (arenas, buffer pre-allocation, compact IR) is a top throughput lever.
3. File-level parallel execution with deterministic output order improves repo-scale throughput.
4. Comment attachment quality should be improved without introducing branch-heavy hot loops.
5. Compatibility fixes must be accepted only when benchmark impact stays within budget.

## 15. Repository Layout (Phase 0/1)

```text
javafmt/
  Cargo.toml
  crates/
    javafmt/
      src/
        lib.rs
        lexer/
        parser/
        cst/
        comments/
        ir/
        printer/
        emit/
    javafmt-cli/
      src/main.rs
    gjf-reference/
      src/main.rs
  tests/
    golden/
    differential/
    idempotence/
  fixtures/
    java/
  benches/
    format_bench.rs
    corpus/
  scripts/
    update-gjf.ps1
    run-diff.ps1
  tools/
    gjf/
      version.txt
  docs/
    IMPLEMENTATION_PLAN.md
```

Naming policy:

- use `reference` in module/folder names for GJF comparison tooling

## 16. Current Status Snapshot (2026-03-06)

Implementation is actively in progress and already beyond Phase 0/1:

- current branch head includes recent tooling and hardening improvements through commit `f7ab5db`
- pinned GJF version: `1.34.1` (`tools/gjf/version.txt`)
- latest successful CI run: `22767398922` (both `test` and `reference` jobs succeeded)

Latest local reference gate result (`target/gjf-report-local.json`):

- runs: `3`
- files: `79`
- mismatches: `0`
- `gjf_over_javafmt_ratio`: `2640.332695829754`

Implemented compatibility coverage in recent cycles:

- top-level package/import spacing
- module directive grouping and continuation (`to`, `with`)
- enum constant body/comma behavior
- try-with-resources multiline handling
- annotation handling (`@interface`, named argument wrapping)
- explicit generic-call wrapping and diamond operator spacing
- top-level import ordering for GJF compatibility (`static` first, lexical order in groups)
- CI format gate aligned with local workflow (`cargo fmt --all --check`)
- reusable `cargo bench -p javafmt --bench format_bench` harness with fixture fallback
- token-preservation regression coverage for representative rewrite cases

Known caveats / guardrails:

- some GJF edge inputs are non-idempotent (example: `do { /*x*/ a(); } while (...)` with specific comment layouts)
- such unstable inputs should not be promoted into `fixtures/java` gate corpus
- when import block contains comments, keep original order (skip reorder pass for safety)
