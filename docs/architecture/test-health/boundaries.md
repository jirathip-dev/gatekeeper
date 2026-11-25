# Test health: module boundaries and statecharts (#99)

## Where the code goes

```
crates/gatekeeper-model/src/
  health.rs             TestsConfig, Timed, Coverage, When, TestId, Case, Changed, Ceiling, Flaky, Covered, Health

crates/gatekeeper-engine/src/
  check/
    cap.rs              unchanged rules; its file walk and `#[cfg(test)]` cut move to health/count.rs
    test_health.rs      core/test-health: the ratio against the ceiling, the ratchet folder, the flaky entries, the soak globs
    test_sleeps.rs      core/test-sleeps: sleeps and wall-clock waits in test code
  baseline.rs           two more arms beside arch's: core/test-health → health::ratchet::write; a [timed] gate → its slow list
  health/               new, a collected folder
    mod.rs              `after`: the one entry run/gate.rs calls
    count.rs      pure  a path and its text → test and src lines
    ratchet.rs          ci/tests/ratio.d: read, the ceiling, write
    flaky.rs            ci/tests/flaky.d: read, quarantine at a date
    spans.rs      pure  a Rust file's test functions; diff hunks → Changed
    junit.rs      pure  JUnit text → cases
    judge.rs      pure  evidence → rerun set, verdict
    rerun.rs      pure  a rerun set → `{only}` for nextest or vitest
    timed.rs            runs the line again, reads the files, calls the pure ones
    coverage.rs         reads the summary, compares with the entry, reads a Drop and its trailer
    entry.rs            the integration entry's health.json; the leg times, the coverage and the sightings of `ci done`
  run/gate.rs           one call: `health::after`
  ci.rs                 one call: `health::entry::done`; the schedule event selects `when = "nightly"`
  plan.rs               `Skip::Nightly`, `Skip::CiOnly(Coverage)`
```

## Who may call whom

```
 cli ──▶ engine::baseline ──▶ health::ratchet (write) · health::timed (the slow list)
 run/gate.rs ──▶ health::after ──▶ timed ──▶ runner (the only process starts)
                                   │    └──▶ junit · spans · judge · rerun · flaky · count      (pure, or files only)
                                   └▶ coverage ──▶ entry (read)
 check/test_health.rs ──▶ count · ratchet · flaky          check/test_sleeps.rs ──▶ count
 check/cap.rs ──▶ count
 ci.rs ──▶ health::entry (done: write on an integration push, compare on a PR)
```

- `count`, `spans`, `junit`, `judge`, `rerun` are pure: no `fs::`, no `runner::`. An arch ban holds it, as it
  does for the selection rules.
- Only `timed.rs` starts a process, through `runner`. Only `ratchet.rs` writes under `ci/tests/`.
- Nothing in `health/` calls `check/`, `select/`, `cache/` or `docs/`. A `[select]` gate that is also `[timed]`
  hands its rerun files to `select` through the `Picked` value `run/gate.rs` already holds.
- Nothing reads the GitHub API or the environment. The CLI reads the date once; the workflow moves the entry
  folder, as it does today.
- The model crate holds the shapes and no rule.

## Statechart: the ratio ceiling

```
                       baseline core/test-health
  [none] ────────────────────────────────────────────▶ [at c]   c = the ratio rounded up to the grid, ≥ 1.00
   every run fails: "no ceiling"                          │  ▲
                                                          │  │ ratio > c ──▶ the gate fails; nothing is written
                 ratio ≤ c − 0.05: the gate fails         │  │
                 until `baseline` adds a lower file       ▼  │
                                                      [at c′ < c] ──▶ … ──▶ [at 1.00]  (final: flat)
```

No arrow raises it: an edited or deleted file fails the PR that does it.

## Statechart: a flaky test

```
  [healthy] ── integration run: fails, then passes on a rerun at one sha ──▶ [sighted]
  [sighted]     done writes the entry file and flaky.json; drover files `flaky: <test>` and spawns its lane
       │ the lane's first commit lands ci/tests/flaky.d/<slug>.toml (since = the sighting's day)
       ▼
  [quarantined] it runs; its failures are warnings
       │ fixed or deleted, entry file deleted ──▶ [healthy]
       │ today ≥ since + 7 days
       ▼
  [ended]       core/test-health fails every PR
       │ fixed or deleted, entry file deleted ──▶ [healthy]
```

Until its entry lands a sighted test still blocks a PR it fails in: the seven days count from the sighting,
whenever the file lands.

## Statechart: a timed gate's run

```
  [run 1: the line, {only} empty]
       │ no JUnit file, or the line failed and no case did ──▶ [FAIL]
       ▼
  [judge the rerun set]
       │ empty ──▶ [verdict]
       ▼
  [run 2: {only} = the set] ─▶ [run 3] ─▶ [verdict]
  [verdict] ── findings ──▶ [FAIL]   ── none ──▶ [ok], warnings in the note
```

The gate's own `budget_secs` covers all three runs.

## How it is tested

| What | Where | How |
|---|---|---|
| `core/test-health` | `packs/core/fixtures/test-health/<case>/` | selftest: the base passes; each case fails with its `expect` line: `over-ceiling`, `no-ceiling`, `a-step-under`, `ceiling-edited`, `ceiling-raised-by-a-new-file`, `inline-tail-counts-as-test`, `src-override`, `swift-tests-folder`, `ts-spec-files`, `quarantine-ended`, `entry-without-issue`, `quarantine-renewed`, `soak-outside-tests`; passing cases `in-quarantine`, `under-one` |
| `core/test-sleeps` | `packs/core/fixtures/test-sleeps/<case>/` | selftest: a sleep per language, a wall-clock read, a soak file's sleep (passes), a listed one (passes), one more than listed |
| the pure rules | `crates/gatekeeper-engine/tests/health/<case>/case.toml` | one case a folder: JUnit files, a diff, the lists, a date, the trigger in; findings, warnings, the rerun set and `{only}` out. The JUnit files are real output, captured once from the pinned nextest and from vitest |
| a timed gate, end to end | the same folder, cases with a `runner.sh` | a script stands in for the runner: it writes a canned JUnit per call (pass, fail, pass) and records its arguments, so the three runs and `{only}` are asserted through `run::gate` |
| coverage and the entry | the same folder | a summary of each tool, an entry, a `Drop` with and without its trailer |
| the real runners | gatekeeper's own gates | its `local/test` gate becomes `[timed]` on nextest, `local/select-vitest` `[timed]` on vitest, and it gets a coverage gate: every PR to gatekeeper runs them |
| the date | a case's `env` | `GATEKEEPER_TODAY` fixes it |

Every new pack gate has a failing fixture case, or selftest refuses it, as today.
