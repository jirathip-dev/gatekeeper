# Test health checks (#99)

## Summary

Decision: two new core gates read the tree alone: the test-to-src ratio under a ceiling that only falls, and a lint for sleeps in tests. A repo's own test gate becomes "timed": gatekeeper reads its JUnit file, fails a new or changed test over 2 s, and runs changed tests three times. Flaky tests are one small file each, dated, blocking after 7 days. Coverage is compared with the integration branch's last run. Perf tests are whole files named in config and run by a nightly gate.
Slices: gatekeeper: a cap raise, the ratio gate, the sleep lint, timed gates, flaky list, coverage, nightly gates, release 1.6.0. Then one adoption per repo, drover and shed first, and one drover issue to file flaky issues.
Answered: (1) cargo-nextest is approved as a pinned tool of the rust pack; (2) one cap raise for gatekeeper, 10,500 to 11,700, is approved as G0; (3) drover files a flaky test's issue (D1), and the gates workflow stays read-only.

## Read in this order

| File | What |
|---|---|
| `usage.md` | how it reads: a repo's config, its test, coverage and nightly gates, a lane's run, the CI summary, a flaky entry, the engine's one call |
| `adr.md` | the question, where the code stands, six options tables, the eleven decisions against the owner's six, who writes which file, consequences, the slices, the answered questions, the `arch.toml` lines |
| `shapes.md` | config keys, which lines count per language, the ceiling, the gate tables, a test and its runs, the verdict table, the slow list, the flaky list, coverage, the integration entry, the pinned tools |
| `boundaries.md` | where the code goes beside `check/cap.rs` and `baseline.rs`, who may call whom, three statecharts, how each part is tested |

Read on gatekeeper staging 06e8d38 (1.5.0), drover staging 4cf52e4bf and shed 7a794152, 2026-11-25.

## The design on one screen

```
 the tree alone (lane = CI)                              needs a test run
 ──────────────────────────                              ────────────────
 core/test-health                                        the repo's test gate + [timed]
   test lines / src lines ≤ ceiling                        its line ─▶ JUnit file ─▶ every test's time and result
   ceiling = lowest file in ci/tests/ratio.d/              changed tests (diff ∩ test functions) ─▶ run 3 times
   flaky entries: ci/tests/flaky.d/, 7 days each             runs differ ─▶ flaky: FAIL
 core/test-sleeps                                            fastest run over 2 s ─▶ FAIL
   sleeps and clock reads in test lines                    listed slow test ─▶ warning · quarantined test ─▶ warning
   baseline: only shrinks                                  a soak file's test ran ─▶ FAIL

 CI only                                                 nightly
 ───────                                                 ───────
 a gate + [coverage]                                     a gate with when = "nightly"
   its line ─▶ summary ─▶ line coverage                    skipped on PRs, pushes, lanes
   PR: ≥ integration's − 0.1                               runs the files in [tests] soak
   integration push: writes the number, the leg times,
   and any test that failed and then passed (a sighting)
```

| Owner decision | Where it lands |
|---|---|
| 1. ratio, hard fail; leg times as a warning | `core/test-health`; a `legs:` line in `ci done` |
| 2. start at the current ratio, or 1x | the first file `baseline core/test-health` writes |
| 3. coverage may not drop | a `[coverage]` gate against the integration entry |
| 4. 2 s per test; perf to nightly; a sleep lint | `[tests] budget_secs`, a `[timed]` gate; `[tests] soak` with `when = "nightly"`; `core/test-sleeps` |
| 5. three runs; flaky list, 7 days | `{only}` reruns; `ci/tests/flaky.d/` |
| 6. on with the release | both core gates run from the bump to 1.6.0 |

## What it never does

- Raise a ceiling, or let a PR write the coverage number PRs are compared with.
- Fail an unrelated PR on one slow measurement: a test over budget is run twice more and judged by its fastest.
- Hide a quarantined test: it runs every time and its failure is printed.
- Read the GitHub API, or keep state a lane cannot see, except the coverage number.
- Add a file that every PR edits.

## What the owner decided

Approved 2026-11-25 (q3675), with the three answers above. Four effects worth a look, all
under "Consequences" there: inline Rust tests count as test lines, so drover starts near 1.5 and not 1.3; the
ratchet makes a PR commit a generated file when the ratio crosses a 0.05 line; Rust repos move their test
gates to cargo-nextest; Swift gets the ratio and the lint but no per-test checks yet.
