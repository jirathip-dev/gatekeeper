# ADR: test health checks: ratio, coverage, per-test budget, flaky quarantine (#99)

Note: the line cap this ADR names (`core/cap`, `ci/cap.toml`, `ci/cap.d/`, the `Cap:` trailer) was removed on 2026-11-27 (#133).

## Summary

Decision: two new core gates read the tree alone: the test-to-src ratio under a ceiling that only falls, and a lint for sleeps in tests. A repo's own test gate becomes "timed": gatekeeper reads its JUnit file, fails a new or changed test over 2 s, and runs changed tests three times. Flaky tests are one small file each, dated, blocking after 7 days. Coverage is compared with the integration branch's last run. Perf tests are whole files named in config and run by a nightly gate.
Slices: gatekeeper: a cap raise, the ratio gate, the sleep lint, timed gates, flaky list, coverage, nightly gates, release 1.6.0. Then one adoption per repo, drover and shed first, and one drover issue to file flaky issues.
Answered: (1) cargo-nextest is approved as a pinned tool of the rust pack; (2) one cap raise for gatekeeper, 10,500 to 11,700, is approved as G0; (3) drover files a flaky test's issue (D1), and the gates workflow stays read-only.

Status: approved by the owner 2026-11-25 (q3675), with the amendment below. Read on gatekeeper staging 06e8d38 (1.5.0), drover staging
4cf52e4bf, shed 7a794152. Read `README.md` first.

## Question

How does gatekeeper measure and gate test size, test time, flakiness and coverage in every repo, with the same
result on a lane and in CI where that is possible, without a file every PR edits, and without running any
suite more than it runs today except for the tests a change touches?

## Where it stands

- `core/cap` counts src lines by globs in `ci/cap.toml`, with a `stop_at_cfg_test` cut for Rust. Raises are one
  file each in `ci/cap.d/`; a commit that edits the cap carries a `Cap:` trailer.
- `baseline.rs` keeps a list of a gate's known findings in `.gatekeeper/baseline/<gate>.txt`. It only shrinks,
  and a listed finding that is gone fails the gate until the list shrinks.
- No pack runs tests. Each repo has its own test gate with a `run` line. A gate has one wall-time budget.
- A `[select]` gate already reads vitest's JSON report for per-file times, and a push to the integration branch
  already saves one entry (an Actions cache folder) that every later run restores to cut shards by time.
- Stable libtest prints no per-test time and has no machine format. drover's CI already runs cargo-nextest;
  shed runs `cargo test` with its own per-crate time script (`scripts/budget.py`).
- drover's CI runs its perf and soak tests in every PR (`--test perf`, `--test soak -- --ignored`). drover's
  `soak` verb (#1648) is something else: it measures lane spawns on hosts.
- drover is not on gatekeeper yet (drover#1587). shed pins 1.1.1.
- gatekeeper counts 9,753 lines against a cap of 10,500.

## Options

### 1. Where the ratio ceiling lives and how it falls

| Option | Verdict |
|---|---|
| A. No stored ceiling: a PR may not raise the ratio above its base's, floored at 1x | No. Zero headroom: in drover and shed a regression test with a small fix fails until the same PR deletes tests. Nothing to check with no base (a push, the merged tree). |
| B. One number in a config file, lowered by hand | No. Nothing turns the ratchet, and one line every lowering PR edits. |
| C. One file per lowering in a folder, the lowest wins, on a 0.05 grid; a ratio a whole step under the ceiling fails until a lower file is added | **Yes.** `ci/cap.d` mirrored. Two PRs that lower it never conflict. Headroom is under 5 % of src. |

### 2. Per-test times in Rust

| Option | Verdict |
|---|---|
| A. Parse stable libtest output | No. It has no times. |
| B. libtest's JSON format | No. Unstable: it needs nightly or a bootstrap flag. |
| C. cargo-nextest's JUnit file | **Yes** (open question 1, answered: approved as a pinned tool). Times per test, a filter to rerun exact tests, prebuilt binaries to pin. drover uses it already. |

### 3. What "a changed test" is

| Option | Verdict |
|---|---|
| A. Every test of a changed file | No for Rust: inline tests share a file with src, so most PRs would rerun most tests. Yes for TypeScript, where a test file is only tests. |
| B. Ask the runner what changed | No. nextest has no such list, and vitest's `--changed` follows imports, which is selection, not "this test's code changed". |
| C. Rust: the diff touches the test function's lines, or test code of its file outside any test; TypeScript: the test file changed | **Yes.** A line scan, pure, no parser dependency. |

### 4. Where the flaky list lives

| Option | Verdict |
|---|---|
| A. In the integration entry (an Actions cache folder), written by integration runs | No. A lane never sees it, so a quarantined test still blocks `turn done`. The cache can be evicted, and no reviewer sees the list change. |
| B. Open issues with a label | No. gatekeeper reads no API. |
| C. One file per test in the repo, with its date; the gate computes the quarantine from the tree and the date | **Yes.** The same on a lane and in CI. The file is written by gatekeeper and landed by the issue's lane. |

### 5. The coverage baseline

| Option | Verdict |
|---|---|
| A. A floor in the repo on a grid, rising like the ceiling falls | No. A fine grid adds a file to most PRs; a coarse one lets coverage drop inside a step. The owner said: measured on integration. |
| B. Measure base and head in the PR job | No. Twice the slowest job. |
| C. The integration run's number, kept in the integration entry; a PR is compared with it, 0.1 point of tolerance | **Yes.** The entry and its plumbing exist. A PR never writes it. |

### 6. How a perf or timing test is marked

| Option | Verdict |
|---|---|
| A. An attribute or a name prefix per test | No. gatekeeper would need each runner's way to select by it, and the sleep lint could not tell a soak test's code from the next test's. |
| B. Whole files named by globs in `[tests] soak`, run by a gate marked `when = "nightly"` | **Yes.** In Rust a soak file is its own test target, which is how drover's `perf.rs` and `soak.rs` already are. |

## Decision

The owner's six decisions, each with what this ADR settles.

1. **Ratio** (owner 1, 2). `core/test-health` counts test and src lines by the rules in `shapes.md`: a Rust
   file's `#[cfg(test)]` tail and `tests/` folders, `*.test.*` and `*.spec.*`, Swift `Tests` folders; blank lines
   and non-code fixtures do not count; `[tests] test`, `src` and `exclude` override. Over the ceiling is a
   finding. So inline tests count as test, which the issue's rough numbers did not do: drover measures about
   1.5, not 1.3.
2. **The ceiling** (owner 2, option 1C) is the lowest file in `ci/tests/ratio.d/`. `baseline core/test-health`
   writes the first one at the measured ratio rounded up to 0.05, never under 1.00: a repo under 1x starts and
   stays at 1.00. A ratio a whole step under the ceiling fails until the same command adds a lower file. A PR
   that edits or deletes a file of the folder fails. No one may raise it and no trailer exists for it.
3. **Leg times** (owner 1). `ci done` prints each leg's time beside the integration branch's last and marks
   `WARN` over 20 % and 30 s slower. It never fails.
4. **Timed gates** (owner 4, 5; options 2C, 3C). A gate with a `[timed]` table names its runner and where its
   line leaves a JUnit file. gatekeeper reads every test's time and result from it. One small reader of JUnit
   text serves nextest and vitest; no XML library is added. libtest alone is not supported.
5. **Per-test budget** (owner 4). `[tests] budget_secs`, default 2. A test over it that is not on the gate's
   slow list, or is on it and changed, is run twice more and fails the PR if its fastest run is still over.
   The slow list is the gate's ordinary baseline file, written at adoption and only shrinking; its tests are
   warnings. In CI this is enforced on every runner; on a lane's host it is a warning.
6. **Three runs** (owner 5). In a PR and on a lane, the changed tests run twice more through `{only}`. Runs
   that differ are a `flaky` finding.
7. **The flaky list** (owner 5, option 4C). On a push to the integration branch a failed test runs twice
   more; failed then passed is a sighting, the run passes, and `ci done` writes the entry file and
   `flaky.json`. The entry, `ci/tests/flaky.d/<slug>.toml`, carries the test, the day, the sha and the issue.
   For 7 days from that day the test runs and its failures are warnings. After that `core/test-health` fails
   every PR until the test is fixed or deleted and the file with it. An entry with no issue is a finding.
8. **Coverage** (owner 3, option 5C). A gate with a `[coverage]` table leaves a summary from cargo-llvm-cov or
   vitest. CI only, in its own leg. A PR more than 0.1 point under the integration branch's number fails. A
   deliberate drop is a file in `ci/tests/coverage.d/` with its reason, counted only in the PR that adds it,
   its commit carrying a `Coverage:` trailer with the owner's decision, as a cap raise does.
9. **The sleep lint** (owner 4). `core/test-sleeps` flags sleeps and wall-clock reads in test lines, per
   language, from a pattern file in the pack. Existing hits go to its baseline; soak files are exempt.
10. **The nightly soak** (owner 4, option 6B). A perf or timing test is moved into a file matching
    `[tests] soak`. Such a file is exempt from the budget and the lint; if one of its tests shows up in a timed
    gate's JUnit file, that gate fails. A gate with `when = "nightly"` is skipped on PRs, pushes and lanes and
    runs on the workflow's schedule event.
11. **Rollout** (owner 6). The two core gates are on for a repo from its bump to 1.6.0, so the bump PR writes
    the three baselines. The adoption issue also adds the tables to the repo's own gates. drover#1760 adopts
    this and builds no cap of its own; its delete slices turn the ratchet.

Who writes what:

| File | Writer | Moves |
|---|---|---|
| `ci/tests/ratio.d/*.toml` | `baseline core/test-health`, in any PR | down only |
| `.gatekeeper/baseline/<test gate>.txt` (slow list) | `baseline <gate>` | shrinks only |
| `.gatekeeper/baseline/core-test-sleeps.txt` | `baseline core/test-sleeps` | shrinks only |
| `ci/tests/flaky.d/*.toml` | gatekeeper's done job writes the file; the issue's lane lands it and later deletes it | added on a sighting, deleted with the fix |
| the integration entry's `health.json` (coverage, leg times) | `ci done` on a push to integration, never a PR | replaced each push |
| `ci/tests/coverage.d/*.toml` | by hand, on an owner decision | a record; spent after its PR |

## Consequences

- A repo over 1x cannot add tests faster than src. With the grid there is at most 5 % of src in headroom, and
  the ratchet takes it back as the ratio falls.
- A PR that lowers the ratio past a grid line must commit one generated file. So must the adoption PR.
- Rust repos run their tests through cargo-nextest. shed's `scripts/budget.py` goes at its adoption. nextest
  runs no doctests; a repo that has them keeps a gate for them.
- Changing a helper in a test file reruns every test of that file. Changing a helper in another file reruns
  none: the rule reads the test's own file only.
- In TypeScript, touching a test file that holds a listed slow test fails the PR until that test is fixed.
- Until a sighted test's entry lands, it can still block a PR it fails in. A flake first seen in a PR, in a
  test the PR did not touch, is an ordinary failure.
- Nothing stops a later PR from listing the same test again after its entry is deleted, except review; the
  gate only refuses a renewal inside one PR.
- Coverage is the one check a lane cannot run. With no integration measurement yet (the first run, an evicted
  entry) the PR's coverage gate passes and says so.
- CI gains one job per PR (coverage) and one scheduled run per night. PR jobs lose the perf and soak tests.
- A red nightly run is a red scheduled workflow. With the drover slice it also becomes an issue.
- Swift gets the ratio and the lint only: no runner is read for it. `swift test` writes the same JUnit shape,
  so it can join `Runner` later.
- gatekeeper gains about 1,650 lines: the cap goes from 10,500 to 11,700 in G0 (open question 2).

## Slices

Gatekeeper. G2 to G6 add files to collected folders; `run/gate.rs` and `ci.rs` get their one call each in G1.

| # | Issue | Blocked on | AC |
|---|---|---|---|
| G0 | cap: raise gatekeeper to 11,700 (filed by the orchestrator) | none (approved) | the raise file with its reason; nothing else |
| G1 | `core/test-health`: counting, the ceiling folder, `baseline core/test-health`; `[tests]` and the date as typed values; leg times in `ci done` | G0 | the selftest cases of `boundaries.md` for the ratio pass; `core/cap` prints what it printed (no `expect` file of it edited); a second `baseline` run writes nothing; `ci done` prints a `legs:` line |
| G2 | `core/test-sleeps` with its pattern file | G1 | a failing case per language; a soak file's sleep passes; `baseline core/test-sleeps` lists and then only shrinks |
| G3 | timed gates: `[timed]`, the JUnit reader, the budget and the slow list, changed tests, three runs; cargo-nextest pinned | G1 | the pure cases pass on real nextest and vitest JUnit files; the stand-in runner case shows three runs and the `{only}` text; `baseline <timed gate>` writes the slow list; a `[timed]` gate without `{only}` is refused |
| G4 | flaky: entries and the quarantine in `core/test-health` and the verdict; the integration rerun; `flaky.json` and the entry file from `ci done` | G3 | cases: in quarantine passes with a warning; day 8 fails `core/test-health`; no issue fails; failed-then-passed on integration passes and is written; failed three times fails |
| G5 | coverage: `[coverage]`, `health.json` in the integration entry, drop files and their trailer; cargo-llvm-cov pinned; the workflow gives a coverage job its LLVM tools | G1 | cases for both tools' summaries, under and within tolerance, a drop with and without its trailer, no entry; a lane skips the gate with its reason |
| G6 | nightly gates: `when = "nightly"`, the schedule event, a soak test in a timed gate's JUnit fails | G3, G5 (both edit `gates.yml`) | a PR plan and a lane skip the gate with its reason; a schedule plan runs only such gates; the soak case fails with its line |
| G7 | gatekeeper adopts it: its test gates timed, a coverage gate, its ceiling; the skill topics; release 1.6.0 | G2 to G6 | its own PR shows all gates green on real runners; `git grep` finds the new keys in the skill |

Adoption, one issue per repo after G7. drover and shed first.

| # | Issue | Blocked on | AC |
|---|---|---|---|
| A1 | drover adopts test health | G7, drover#1587 | pin 1.6.0; the start ceiling by the `baseline` verb; the test gates `[timed]` on nextest, one per shard layout as today; slow list and sleeps baseline written; `perf.rs`, `soak.rs` and the timing tests in `[tests] soak` and a nightly gate; a coverage gate; drover#1760's cap slice is closed as done by this |
| A2 | shed adopts test health | G7 | the same; its test gates move to nextest; `scripts/budget.py` and its fixture are deleted; `timing.toml` and the bench gates become nightly or stay, said in the PR |
| A3… | each other fleet repo, one each | G7 | pin 1.6.0; the start ceiling (1.00); sleeps baseline; `[timed]` and coverage on its vitest gates where it has them |
| D1 | drover: file `flaky: <test>` from an integration run's `flaky.json` with the entry file in the brief, queue it on top and spawn its lane; file `soak: <gate>` for a red nightly gate | G4, G6 | one issue per test, never twice; the lane's first commit is the entry file with the issue's number |

## Open questions, answered

The owner's amendment of 2026-11-25:

1. **cargo-nextest.** Approved as a pinned tool of the rust pack. It lands in G3.
2. **The cap.** One raise, 10,500 to 11,700, approved as G0, filed by the orchestrator.
3. **Who files the flaky issue.** drover (D1), which files every other fleet issue and can queue and spawn at
   once. The gates workflow stays read-only.

## `.drover/arch.toml` lines

Gatekeeper, landed with this ADR (#99), so G1 adds none:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/src/health"
glob = "*.rs"

[[collect]]
dir = "crates/gatekeeper-engine/tests/health"
glob = "*/case.toml"

[[ban]]
pattern = "runner::"
paths = ["crates/gatekeeper-engine/src/health/**"]
except = ["crates/gatekeeper-engine/src/health/timed.rs"]
reason = "one module of test health starts processes"

[[ban]]
pattern = "fs::"
paths = ["crates/gatekeeper-engine/src/health/count.rs", "crates/gatekeeper-engine/src/health/spans.rs", "crates/gatekeeper-engine/src/health/junit.rs", "crates/gatekeeper-engine/src/health/judge.rs", "crates/gatekeeper-engine/src/health/rerun.rs"]
reason = "the test health rules are pure: evidence in, a verdict out"
```

The two new check files and the fixture cases need no line: their folders are collected and `check/mod.rs` is
generated by `sh scripts/gen.sh`. No file joins `frozen`.

Each adopting repo, in its adoption PR:

```toml
[[collect]]
dir = "ci/tests"
glob = "*.d/*.toml"
```

The three folders are one file per entry by design, so no list of ceilings, flaky tests or coverage drops is
ever a file two PRs edit.
