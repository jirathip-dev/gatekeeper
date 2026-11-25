# Test health: how the callers read (#99)

Written first. Everything else in this folder exists to make these read this way.

## A repo's config

```toml
# .gatekeeper/config.toml: every key optional; with no table the defaults apply
[tests]
budget_secs = 2                                    # one test's time budget
soak = ["crates/*/tests/perf.rs", "crates/*/tests/soak.rs", "web/**/*.soak.test.ts"]
test = ["crates/testkit/**"]                       # count as test, beside the built-in rules
src = ["crates/server/src/tests/**"]               # count as src although the rules say test
exclude = ["web/src/generated/**"]                 # count as neither
```

## The repo's test gate

```toml
# .gatekeeper/gates/test.toml
about = "every workspace test, each within the per-test budget"
fix = "fix the test the log names"
budget_secs = 600
setup = ["rust"]
run = "{tool:cargo-nextest} nextest run --workspace --locked --profile ci {only}"

[timed]
runner = "nextest"                                 # or "vitest"
junit = "target/nextest/ci/junit.xml"              # where the run line leaves its JUnit file
```

`{only}` is empty on the first run. On a rerun it is the runner's own filter for the tests to run again.

## The coverage gate and a nightly gate

```toml
# .gatekeeper/gates/coverage.toml
about = "line coverage is not under the integration branch's"
fix = "cover the lines the change added, or delete the dead code"
budget_secs = 900
needs = ["coverage"]                               # CI only; a lane's host skips it
leg = "coverage"
run = "{tool:cargo-llvm-cov} llvm-cov nextest --workspace --locked --json --summary-only --output-path target/cov.json"

[coverage]
tool = "llvm-cov"                                  # or "vitest"
summary = "target/cov.json"
```

```toml
# .gatekeeper/gates/soak.toml
about = "the perf and timing tests, nightly"
fix = "the log names the test and its number"
budget_secs = 1800
when = "nightly"                                   # pull requests, pushes and lanes skip it
run = "{tool:cargo-nextest} nextest run --locked --release -E 'binary(perf) | binary(soak)' --run-ignored all"
```

## A lane's run

```
$ ./.gatekeeper/run check
ok    core/test-health     0.2s /    5s
      tests 61,204 / src 47,310 lines: ratio 1.30, ceiling 1.30 · 2 quarantined
FAIL  core/test-sleeps     0.1s /    5s
      crates/core/tests/drive.rs: 1 × thread::sleep
FAIL  local/test          48.1s /  600s
      1,912 tests · 3 changed, run 3 times · 4 over 2 s listed
      flaky: drover::drive retries_after_a_timeout passed 2 of 3 runs
      over 2 s: drover::board renders_30k_events took 3.4 s at its fastest of 3 (changed in this change)
      warn: over 2 s, listed: drover::sim the_hunt_finds 41.0 s
      warn: quarantined until 2026-12-02 (#1771): drover::link reconnects_once failed
skip  local/coverage                        CI only on a lane's host: needs the integration branch's coverage
skip  local/soak                            runs in the nightly soak
```

Over the ceiling:

```
FAIL  core/test-health     0.2s /    5s
      tests 61,950 / src 47,310 lines: ratio 1.31 is over the ceiling 1.30 (ci/tests/ratio.d/2026-12-01-130.toml)
```

Under it by a whole step: the ratchet turns, in the same PR.

```
FAIL  core/test-health     0.2s /    5s
      ratio 1.24 is a step under the ceiling 1.30: lower it (./.gatekeeper/run baseline core/test-health)
$ ./.gatekeeper/run baseline core/test-health
ci/tests/ratio.d/2026-12-09-125.toml: ceiling 1.25 (was 1.30)
```

## The three baseline writes at adoption

```
$ ./.gatekeeper/run baseline core/test-health     # the start: ci/tests/ratio.d/<date>-<ceiling>.toml
$ ./.gatekeeper/run baseline core/test-sleeps     # .gatekeeper/baseline/core-test-sleeps.txt
$ ./.gatekeeper/run baseline local/test           # runs the gate once: .gatekeeper/baseline/local-test.txt, the tests over budget
```

After that each one only shrinks, by the same command.

## The CI summary (`ci done`)

```
| job | host | setup | gate | took | budget | | note | proved by |
| …
legs: rust 6m12s (integration 5m02s) WARN +23% · core 0m21s (0m20s) · coverage 9m40s (9m31s)
coverage: local/coverage 71.4 % (integration 71.6 %, within 0.1: no) FAIL
```

On a push to the integration branch the same job also writes what the next pull requests compare against, and
reports flakes:

```
flaky at 4cf52e4: drover::link reconnects_once failed, then passed on rerun 2
  list it: ci/tests/flaky.d/drover-link-reconnects-once.toml (in the artifact gates-done, under flaky/)
```

## A flaky test's list entry

```toml
# ci/tests/flaky.d/drover-link-reconnects-once.toml: written by gatekeeper, landed by the issue's lane
test = "drover::link reconnects_once"
since = 2026-11-25
sha = "4cf52e4bf0c1…"
issue = 1771
```

Seven days later, in every run of every PR:

```
FAIL  core/test-health
      ci/tests/flaky.d/drover-link-reconnects-once.toml: quarantine ended 2026-12-02 (#1771): fix or delete the test, then delete this file
```

## A deliberate coverage drop

```toml
# ci/tests/coverage.d/2026-12-10-local-coverage-68.toml: counts only in the PR that adds it
gate = "local/coverage"
floor = 68.0
why = "owner decision q3410: the test diet deletes 14k lines of duplicate tests"
```

Its commit carries `Coverage: 71.6 → 68.0 (#1760, q3410)`, as a cap raise carries `Cap:`.

## The repo's workflow

```yaml
on:
  pull_request:
  push: { branches: [staging] }
  schedule: [{ cron: "0 18 * * *" }]      # the nightly soak: gates.yml runs only the `when = "nightly"` gates
jobs:
  gates:
    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@v1.6.0
```

## The engine, where a gate ends (`run/gate.rs`)

```rust
let result = steps_result.and_then(|()| health::after(gate, cx, &mut log, deadline, &mut listed));
```

```rust
// health/timed.rs: the one function that starts processes; everything it calls is pure
pub(crate) fn run(gate: &Gate, timed: &Timed, cx: &RunCtx, log: &mut File, deadline: Instant) -> Result<Verdict> {
    let first = junit::read(&text_of(&timed.junit)?)?;
    let changed = spans::changed(&diff_hunks(cx)?, &test_code(cx)?);
    let again = judge::rerun_set(&first, &changed, &slow_list, cx.trigger());
    let reruns = [1, 2].map(|_| run_only(gate, &rerun::only(timed.runner, &again), cx, log, deadline));
    Ok(judge::verdict(&Evidence { runs: &[first, reruns…], changed: &changed, slow: &slow_list,
                                  flaky: &flaky::read(&cx.root)?, soak: &cx.tests.soak, today: cx.today,
                                  budget: cx.tests.budget, host: cx.host }))
}
```
