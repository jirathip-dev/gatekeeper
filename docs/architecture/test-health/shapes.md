# Test health: data shapes (#99)

## Config: `[tests]` in `.gatekeeper/config.toml`

```rust
/// `[tests]`: every key has a default, so a repo with no table is checked too.
#[serde(default, deny_unknown_fields)]
struct TestsConfig {
    budget_secs: Secs,      // one test's budget. Default 2.
    test: Vec<Glob>,        // count as test, beside the built-in rules
    src: Vec<Glob>,         // count as src although a rule says test
    exclude: Vec<Glob>,     // count as neither
    soak: Vec<Glob>,        // whole test files that run nightly, never in a PR gate
}
```

Refused when the config is parsed: a `soak` glob that can match a Rust file outside a `tests/` folder (a soak
test is its own test target), `budget_secs = 0`.

## Which lines count

A line counts when its file has a code extension of a configured pack (`.rs`; `.ts` `.tsx` `.mts` `.cts`;
`.swift`), is tracked by git and is not blank. Fixtures in other formats (JSON, snapshots, golden text) count
as neither. Each counted line is test or src:

| Language | Test | Src |
|---|---|---|
| Rust | every line of a file with a `tests` or `benches` path segment, or named `tests.rs`; in any other file, the tail from the first line that starts `#[cfg(test)]` | the rest |
| TypeScript | every line of `*.test.*`, `*.spec.*`, and of a file with a `__tests__`, `tests` or `e2e` path segment | the rest; `*.d.ts` is neither |
| Swift | every line of a file under a folder named `Tests` or ending in `Tests` (SwiftPM's `Tests/`, an Xcode `AppTests`, `AppUITests` target folder) | the rest |

Order: `exclude`, then `src`, then `test`, then the table. The Rust tail rule is the one `core/cap` uses
(`stop_at_cfg_test`), so both checks cut a file at the same line.

```rust
enum Side { Test, Src }
/// Pure: a path and its text in, the two counts out.
fn count(path: &str, text: &str, config: &TestsConfig) -> Option<Lines>;
struct Lines { test: u32, src: u32, test_from: Option<u32> }   // `test_from`: where the test tail starts
```

## The ratio ceiling: `ci/tests/ratio.d/<date>-<hundredths>.toml`

```toml
ceiling = 1.30
```

```rust
/// A ratio in hundredths: 130 is 1.30. Never under 100.
struct Ceiling(u32);
const STEP: u32 = 5;                       // the ratchet's grid: 0.05
fn ceiling(files: &[Ceiling]) -> Option<Ceiling>;          // the lowest file; none: not started
fn fits(lines: Totals) -> Ceiling;                         // the ratio rounded up to the grid, at least 1.00
```

What `core/test-health` holds, from the tree alone: a ceiling exists; `tests × 100 ≤ ceiling × src`;
`ceiling ≤ fits(lines)`. With a base: no file of the folder is edited or deleted, and a new file is under the
base's ceiling. Writer: `baseline core/test-health`, which writes `fits(lines)` when that is the first file or
lower than the ceiling, and nothing otherwise.

## The gate tables

```rust
struct GateFile {
    // …
    timed: Option<Timed>,
    coverage: Option<Coverage>,
    when: When,                 // default `Always`
}
/// `[timed]`: the gate's line runs tests and leaves a JUnit file.
struct Timed { runner: Runner, junit: RepoPath, root: RepoPath /* default "." : what the JUnit's paths are relative to */ }
enum Runner { Nextest, Vitest }
/// `[coverage]`: the gate's line leaves a coverage summary.
struct Coverage { tool: CoverageTool, summary: RepoPath }
enum CoverageTool { LlvmCov, Vitest }    // `data[0].totals.lines` · `total.lines` of json-summary
enum When { Always, Nightly }
enum Capability { Containers, PrivateNames, Coverage }     // `Coverage`: CI only
```

Refused when the gate file is parsed: `[timed]` with `[[step]]` or without `{only}` in `run` (a `[select]`
gate's reruns go through `{files}`, so it needs none); `[coverage]` with shards or without
`needs = ["coverage"]`; `when = "nightly"` with `[timed]` or `[coverage]`.

## A test and its runs

```rust
/// `<classname> <name>` as the JUnit file gives them: `drover::link reconnects_once`,
/// `src/board.test.ts board > renders 30k events`.
struct TestId(String);
struct Case { id: TestId, took: Millis, ended: Ended }
enum Ended { Passed, Failed, Skipped }
/// Pure: one JUnit text in, its cases out. Reads `testsuite`, `testcase`, `failure`, `error`, `skipped`.
fn read(junit: &str) -> Result<Vec<Case>>;

/// A changed piece of test code: a Rust test function, or a whole TS test file.
enum Changed { RustFn { file: RepoPath, name: String }, RustFile(RepoPath), TsFile(RepoPath) }
```

A test is changed when the diff against the base touches a line of it. Rust: its function, from its test
attribute to its closing brace; a change to test code of the same file outside every test function
(`RustFile`: a helper, a fixture) changes every test of that file. A `RustFn` matches the cases whose binary
is the file's (`<crate>` for `src/`, `<crate>::<target>` for `tests/<target>…`) and whose name has the
function's name as a `::` segment. TypeScript: every test of an added or modified test file.

## The verdict of a timed gate

```rust
struct Evidence<'a> {
    runs: &'a [Vec<Case>],          // 1 run, or 3
    changed: &'a [Changed],
    slow: &'a BTreeSet<TestId>,     // the gate's baseline file
    flaky: &'a [Flaky],
    soak: &'a [Glob],
    budget: Secs,
    today: Date,
    trigger: Kind,                  // PullRequest | Integration | Lane
}
struct Verdict { findings: Vec<String>, warnings: Vec<String>, flaky: Vec<Sighting>, stale: Vec<TestId> }
fn rerun_set(first: &[Case], changed: &[Changed], slow: &BTreeSet<TestId>, trigger: Kind) -> BTreeSet<TestId>;
fn verdict(e: &Evidence) -> Verdict;
```

| Run | Rerun set (run twice more) | A finding (the gate fails) | A warning |
|---|---|---|---|
| a PR in CI | changed tests, and tests over budget that are not listed | a test that failed every run; a rerun test whose runs differ (`flaky`); a rerun test over budget at its fastest; a soak file's test that ran | a listed test over budget; a quarantined test's failure |
| a lane | the same | the same, but over budget is a warning: a lane's host is shared | the same |
| a push to integration | tests that failed | a test that failed all three | failed then passed: a `Sighting`; any test over budget |

Over 200 tests in the rerun set reruns the whole line. A quarantined test's result is never a finding. A
listed test is `stale` when it ran in under half the budget: the gate fails until the list shrinks.

## The slow list: `.gatekeeper/baseline/<gate>.txt`

Today's baseline file, one line a test: `over budget: drover::sim the_hunt_finds`. Written once by
`baseline <gate>`, which runs the gate; after that it only shrinks, and that run also drops tests that no
longer exist.

## The flaky list: `ci/tests/flaky.d/<slug>.toml`

```rust
#[serde(deny_unknown_fields)]
struct Flaky { test: TestId, since: Date, sha: String, issue: u32 }
const QUARANTINE_DAYS: u32 = 7;
enum Quarantine { Until(Date), Ended(Date) }
fn quarantine(entry: &Flaky, today: Date) -> Quarantine;
/// What an integration run saw: the done job writes it as `flaky/<slug>.toml` (no `issue` yet) and `flaky.json`.
struct Sighting { test: TestId, sha: String, on: Date, gate: GateId, passed_on_rerun: u8 }
```

`core/test-health` fails on: an entry that does not parse or has no `issue`; `since` after today; a `sha`
that is no commit; `Ended`; with a base, a change that deletes one entry of a test and adds another (a
quarantine is not renewed).

## Coverage

```rust
/// Line coverage in tenths of a percent: 714 is 71.4 %.
struct Tenths(u32);
struct Covered { gate: GateId, lines: Tenths, sha: String }
const TOLERANCE: Tenths = Tenths(1);
/// `ci/tests/coverage.d/<date>-<gate>-<floor>.toml`
struct Drop { gate: GateId, floor: Tenths, why: String }
```

A PR's coverage gate fails when `lines + TOLERANCE < integration's`, unless the change itself adds a `Drop`
for the gate and `lines ≥ floor`. A commit that adds one carries `Coverage: <old> → <new> (#issue, q<N>)`.
With no integration measurement the gate passes and says so.

## The integration entry

The folder a push to the integration branch saves and every later run restores: today it holds one
`<gate>.json` of file times per `[select]` gate.

```rust
/// `<entry>/health.json`
struct Health { schema: u32, sha: String, coverage: Vec<Covered>, legs: BTreeMap<String, Millis> }
```

A leg's time is its jobs' setup plus their gates' times, the slowest shard. `ci done` prints each leg beside
the entry's and marks `WARN` over 20 % and 30 s slower. It never fails on it.

## The typed inputs that are new

```rust
struct CheckCx<'a> { /* … */ tests: &'a TestsConfig, today: Date }
struct RunCtx { /* … */ tests: TestsConfig, today: Date, health: Option<Health> }
```

`today` is read once by the CLI: `GATEKEEPER_TODAY` when set (a fixture case's `env`), else the clock, in UTC.

## The pinned tools

| File | Tool | Status |
|---|---|---|
| `packs/rust/tools/cargo-nextest.toml` | cargo-nextest, per target with its sha256 | new dependency, approved by the owner 2026-11-25 |
| `packs/rust/tools/cargo-llvm-cov.toml` | cargo-llvm-cov, the same | approved |
| none | vitest and its v8 coverage are the consumer's own dev dependencies | approved |
