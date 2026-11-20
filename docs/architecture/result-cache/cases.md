# The result cache: each decision and the case that proves it (#17)

An engine case is a folder, `crates/gatekeeper-engine/tests/cache/<case>/case.toml`, run by one test that globs
the folder and fails on a count that is off, as `tests/budget/` and `tests/kept/` do. A case is a small repo, a
list of runs, and what each run must report:

```toml
# tests/cache/changed-input-misses/case.toml
gate = { run = "test -f src/a.txt", paths = ["src/**"], versions = ["echo 1"] }
files = { "src/a.txt" = "one", "docs/b.md" = "one" }

[[run]]
trigger = "lane"
expect = "ok"            # a miss: it runs and is recorded

[[run]]
trigger = "lane"
edit = { "docs/b.md" = "two" }
expect = "cached"        # outside its inputs

[[run]]
trigger = "lane"
edit = { "src/a.txt" = "two" }
expect = "ok"            # an input changed: a miss
```

A run may also set `versions` (the probes' outputs), `platform`, `full`, `salt`, `now`, `store` (files to plant
before it) and `expect_store` (the records that must be there after it).

## The key

| Decision | Case |
|---|---|
| a changed input file misses; a changed file outside the inputs hits | `changed-input-misses` |
| a file in the pack's `always` is an input, and puts the gate in scope | `always-file-changes` |
| a change under `.gatekeeper/` misses every gate | `gatekeeper-folder-changes` |
| a new, untracked file inside the inputs misses; an ignored one does not count | `untracked-input-misses` |
| a gate with no `paths` misses on any tracked change | `whole-tree-gate` |
| a changed toolchain version misses | `changed-version-misses` (the `rustc -Vv` probe differs) |
| a changed pinned tool misses | `changed-tool-misses` (the tool's asset sha256 differs) |
| a changed gate file misses, even a changed `about` | `changed-gate-file-misses` |
| another gatekeeper misses | `changed-engine-misses` |
| another platform misses | `changed-platform-misses` |
| a `run` gate with no `setup` and no `versions` has no key | `no-version-not-cached` |
| `cache = false`, `{--base}`, `{--pr-body}` and `[[upload]]` have no key | `opted-out`, `reads-base`, `reads-pr-body`, `keeps-files` |
| per project: a change in a needed project misses, in an unrelated one hits (after G10) | `project-needs-changed`, `project-unrelated-hits` |
| a sharded job hits only on the same set of projects (after G10) | `shard-set-differs` |

## What is stored

| Decision | Case |
|---|---|
| a failure is never stored | `failure-is-not-recorded` |
| a gate over an enforced budget is not stored | `over-budget-is-not-recorded` |
| a hit writes nothing; the proof stays the run that ran it | `hit-writes-nothing` |
| a record older than `max_age_days` misses | `expired-record-misses` |

## Trust

| Decision | Case |
|---|---|
| a pull request cannot write the integration tier | `pull-request-writes-its-own`: after the run, `integration/` is byte for byte what it was |
| with `pull_requests = "read"` it writes nothing | `read-only-pull-request` |
| a pull request does not read another pull request's tier | `other-pull-request-not-read` |
| a fork reads and writes nothing | `fork-is-off` |
| a record whose name is not the hash of its parts is refused | `forged-key-refused` |
| a record with the right name and other parts is refused | `forged-parts-refused` |
| a record whose proof is not its tier's is refused (a pull request's record laid in `integration/`) | `forged-scope-refused` |
| a refused record never fails the gate: it runs | all three above expect `ok` and the note `refused` |
| `mode` gives no pull request `write: Integration` and no integration push a read | a unit test over every `Trigger`, `full` and `PullRequests` value |

## The second net

| Decision | Case |
|---|---|
| a push to the integration branch reads nothing and writes the integration tier | `integration-push-does-not-read` |
| a schedule and any other push read and write nothing | `other-run-is-off` |
| a full run does not read (`--all`, `scope: full`) | `full-run-does-not-read` |
| `--no-cache` reads and writes nothing | `no-cache-flag` |
| a sampled hit is rerun; a pass renews the record | `recheck-passes` |
| a sampled hit that fails now fails the gate, names the proving run, deletes the record | `recheck-differs-fails` |
| `recheck = 1` reruns every hit | `recheck-every-hit` |
| no `[cache]` table: nothing is read or written | `not-configured` |

## Printing and counting

Driven through the binary, in `crates/gatekeeper/tests/cache.rs`:

| Decision | Test |
|---|---|
| a second `check` prints `cached`, the proving run and its time, and counts it in the last line | `a_second_check_prints_cached_with_its_proof` |
| a recheck that differs prints "cached pass does not hold" and exits 1 | `a_cached_pass_that_fails_now_is_loud` |
| `list` prints each gate's key or why it has none | `list_shows_a_key_or_why_not` |
| `ci plan` prints the restore and save prefixes of `mode` | `ci_plan_names_the_tiers_of_its_trigger` |
| `ci done` prints the `proved by` column and the totals, and fails on `differed > 0` | `ci_done_counts_the_cache` |

## Speed

| Decision | Test |
|---|---|
| the 200 ms assert holds with the cache on and every gate a miss, the probes stubbed | `tests/perf.rs`, the existing test with `[cache]` |
| a check whose every gate hits takes under 200 ms | `tests/perf.rs`, a second test |

## Fixtures and selftest

This issue adds no gate, so no pack fixture. `selftest` runs with the cache off; its test asserts that a
selftest after a full hit still runs every fixture.
