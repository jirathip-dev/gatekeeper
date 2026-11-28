# ADR: per-file test selection for TypeScript (#46)

Note: the line cap this ADR names (`core/cap`, `ci/cap.toml`, `ci/cap.d/`, the `Cap:` trailer) was removed on 2026-11-27 (#133).

## Summary

Decision: a test gate may ask for per-file selection. gatekeeper then runs only the test files a change can
reach: those whose imports reach a changed file (asked of the repo's own vitest), the always-run guards, tests
that read a changed file by path or walk its project or a folder above it, and a deleted file's project. A file
outside every project no longer runs everything. Shards cut one list by recorded time; one cache record per job.
Slices: the rules and their cases; vitest and the run; shards and timings; the replay report; one adoption each.
Open questions: none. Revised on the owner's answers: walkers narrowed, project readers per test, vitest allowed.

## Status

Proposed 2026-11-22; revised the same day on the owner's answers, 2026-11-22 (near the end); approved the same
day. Nothing is built. The issues below are filed now.

## Question

Affected selection (G10) works per project and runs everything when one changed file lies outside every project.
A consumer measured that shape of rule on its own repo: 25 of 32 sampled pull-request runs went to the full
suite, most often for a workflow file. It then built a per-file selector that runs far fewer tests. How does
gatekeeper select at least as tightly, prove each rule, and let a consumer measure it against its own selector?

Decided by the owner already (2026-11-20): until it does, such a repo does not freeze its workflow on gatekeeper.

## What the code says first

- `scope::affected` returns every project for one file outside all of them. `[scope] ignore` is the only relief,
  a list a repo would grow by hand.
- `ci plan` never sees the change. It prints the same matrix for every run, and each leg diffs against the base
  itself. A selection belongs where the diff is: in the leg.
- A `run` line goes to `sh -c` as one argument. Linux caps one argument at 128 KB, so a list of thousands of
  test files cannot be spliced into the line.
- `run::changed` keeps names only. The widening rules need to know which files were deleted.
- A gate with no `paths` is keyed on every tracked file, so its record hits only on an unchanged tree.
- `src/` is at 6,202 lines of a 6,700 cap.

## Options

**1. What selects**

| Option | Verdict |
|---|---|
| A. Keep per-project selection, with a longer `ignore` list | No. The list is a hand-kept guess at what no test reads, and a change inside a project still runs all of it. |
| B. A selector command the consumer supplies | No. Every consumer keeps its own selector and its own proof of it; a fix lands once per repo. The selection engine and its tests live in gatekeeper (#1, decision 9). The replay calls such a command once, to be measured against. |
| C. Per-file selection in gatekeeper, for the ts pack | **Yes.** One set of rules, one set of cases, the same on a lane's host and in CI. |

**2. Who computes the import closure**

| Option | Verdict |
|---|---|
| A. An import parser in the engine | No. A second resolver beside vitest's: aliases, plugins and path maps would drift from what a test really loads. |
| B. A script in the pack on vitest's Node API | No, not now. It could name each test's whole graph, but that API changes with each major. |
| C. The repo's own `vitest list --filesOnly --changed <base>` | **Yes.** vitest's resolver, through a documented command, at the version the repo runs its tests with. |

**3. Where the selection is computed in CI**

| Option | Verdict |
|---|---|
| A. In the `plan` job, once | No. `plan` would need Node, the install and the full history, and every leg of every pack would wait for it. |
| B. In a first leg the test leg runs after | No. An `after` leg waits for every first leg to end. |
| C. In each shard: the same commit, base and timings give the same list, and `done` fails when they did not | **Yes.** No leg waits, and the plan stays pure. |

**4. Where recorded times come from**

| Option | Verdict |
|---|---|
| A. vitest's own `--shard` | No. It cuts by path, not by time. |
| B. A timings file committed in the repo | No. The same split everywhere, but a generated file someone must refresh. |
| C. Each shard restores the newest entry from GitHub's cache | No. Two shards of one run can restore different entries; a test would fall in no shard. |
| D. Pushes to the integration branch write; a run's `plan` job names one entry and every shard reads that one | **Yes.** |

**5. What the result cache records**

| Option | Verdict |
|---|---|
| A. One record per project, as a `projects` gate | No. A test that reads another project by path is outside its own project's inputs. |
| B. One record per test file, keyed by the content it imports | No. `vitest list` names the tests a change reaches, not each test's files. |
| C. One record per test file, proved at a commit, held while the selector does not select it since | Not now. It is as sound as the selector; decide it after the replay has measured the selector. |
| D. One record per job: every tracked file, and the list the job ran | **Yes.** Nothing new to trust. |

## Decision

1. **Where it lives.** A `[select]` table on a gate file turns it on for that gate: its vitest config, its
   `always` paths, its `full` files. The rules are engine code under `select/`; the vitest adapter in it is the
   ts pack's part and needs `[ts]`, whose `install` runs first. One gate is one vitest run under one config.
   `projects` stays for every other gate. Shape: `usage.md`.
2. **The rules.** Everything, when the run is not a change or a changed file matches `full` or `.gatekeeper/**`.
   Otherwise the union: the closure vitest lists; tests under `always` and tests in no project; tests whose own
   code holds a changed file's path or its basename as a whole token; tests that walk directories or shell out,
   when they lie in a changed file's project or name a directory above it; tests whose code names a touched
   project; and for a deleted file that was no test, its project's tests and the tests that name it by import
   specifier. Comment-only lines are not read. An empty union is everything. Each selected file carries every
   rule that selected it. Exact wording: `shapes.md`.
3. **Each job selects for itself.** A lane's run and each CI shard run the install, the two vitest lists and the
   rules, inside the gate's time. Each report carries a digest of the list; `done` fails a gate whose shards
   disagree. A selection that cannot be computed fails the gate; it never narrows.
4. **Shards.** The one list is split by each file's recorded time, longest first, each to the lightest shard.
   Times come from vitest's JSON report, which the run line writes to `{report}`. `done` of a push to the
   integration branch merges them and saves one entry; a later run's `plan` job names the newest entry and each
   shard restores exactly that one. The files reach the run line as arguments, in place of `{files}`.
5. **The result cache.** One record per job, keyed as any gate without `paths`, plus the list the job ran. It
   hits where the tree is unchanged: a rerun after another leg flaked, `drover turn done` after `check`. The key's
   schema becomes 2.
6. **What a lane reads.** One line, "selected 37 of 2140 test files" with a count per rule, or "selected all"
   with the reason; and `selection.json`, each file with its reasons. CI's summary carries the same line.
7. **Proof.** One folder a case, each rule and each widening by name (`cases.md`). A case's expectation is the
   whole selection with reasons, so the test a rule is named for is selected by that rule alone. The pure test
   takes the closure from the case; gatekeeper's own gate `select-vitest` asserts a real vitest returns it.
8. **The measure.** `replay` takes the pull-request runs a consumer lists, each with its base, head and the
   tests that failed, selects for each with both selectors in a scratch worktree, and reports per run the
   counts, the files only one chose and the failures each missed, then totals. The gate replaces the consumer's
   selector when, over at least 30 merged pull requests, gatekeeper missed no failure, selected no more in
   total and went full no more often.

## The agent-speed criteria, mapped

| Criterion | Mechanism | What enforces it |
|---|---|---|
| 1. One gate, same everywhere | the same rules pick the same files on a lane's host and in CI; a shard is a slice of that list | `ci done`'s digest check; the ban on `github.event_name` in `gates.yml` |
| 2. A skill | the generated skill gains the selection line, `selection.json` and the "why did this run" recipe | arch-check `generated` |
| 3. Fix once | the rules and their cases are gatekeeper's; a consumer writes one table | the cases; the adoption issue deletes the consumer's selector |
| 4. Smaller blast radius | one file a rule family, one folder a case; reasons are an enum | the `[[collect]]` lines below |
| 5. Fast | a workflow-only change runs the guards, not the suite; shards are even by time | the replay's totals; the summary's per-shard times |

## Consequences

- A `[select]` gate ignores `[scope] ignore` and the outside-every-project rule. A `projects` gate is unchanged.
- Each shard pays the install and two vitest lists. Wall time grows by one selection, runner time by one per
  shard, and a small selection still starts every shard. If the measure shows that hurts, selecting in `plan` is
  its own decision (option 3A).
- The text rules read a test file's own text. A helper that reads a file by path for a test is not seen; the
  replay's missed failures are where that would show, and `always` is the answer for such a test.
- A module deleted in one project and still imported by an unchanged module of another selects neither side's
  tests beyond the deleted file's project. `ts/tsc` fails that change.
- No import graph follows a lockfile. `full` is required so each repo writes that line once, in review.
- A walker runs for a change in its own project or under a directory it names, not on every change. One that
  builds its directory from a variable, or walks the repo's root, names nothing: `always` is for such a test.
- A code line that starts with `*` (a wrapped multiplication, a generator method) is read as a comment.
- Project readers are per test, and match the touched project itself, not the projects that need it: a test
  that reads one of those by path sees no changed file, and one that imports it is in the closure.
- vitest matches a file argument by substring, so `a.test.ts` can also run `a.test.tsx`. More tests, never fewer.
- vitest's own trigger stays: a changed `package.json` or vitest config lists every test of that config.
- Every result record misses once, when the key's schema changes.
- The binary gains no dependency. gatekeeper's repo gains vitest, pinned, in the `package.json` and lockfile of
  one test workspace, `tests/select/_base`; only the gate `select-vitest` installs it. That manifest is outside
  the root one's workspaces, so the deps check does not read it: S2's commit carries `Deps: #46` as the record.
- The build is about 1,200 lines of `src/`, over the 498 lines of headroom. The raise is its own issue, before S1.

## Issues, in order

Filed after approval, each carrying the five criteria and its rows of `cases.md`:

| # | Issue | Needs |
|---|---|---|
| S1 | The rules, pure: `[select]` on a gate file and its refusals, changes with their kind, `rules`, `readers`, the selection cases with the pure test. Nothing runs differently yet. | none |
| S2 | vitest and the run: the adapter, `{files}` as arguments, the selection line and `selection.json`, the list in the result key, the gate `select-vitest`, the skill text. A lane and an unsharded leg select per file. | S1 |
| S3 | Shards: `{report}` and the times, `split`, the timings entry (`done` writes, `plan` names, a shard restores), `done`'s agreement check, the empty shard. | S2 |
| S4 | `replay`: the verb, `replay.json` and `replay.md`. | S2 |
| S5 | One adoption per consumer, in its repo: the gate file, a replay over at least 30 merged pull requests with its totals in the issue, the consumer's selector deleted, then its workflow frozen (#1, adoption 9). | S3, S4 |

## Answered by the owner (the owner's answers, 2026-11-22)

No question is open.

| Asked or proposed | Answer | Where it landed |
|---|---|---|
| Walkers: the first draft ran every test with a walker token on every change | Revise: as the source selector does. A walker is selected when it lies in a changed file's project or names a directory above the changed file; `opendir` and `git grep` are walker tokens; comment-only lines are skipped | `shapes.md`, the rules; the cases `walker-own-project`, `walker-ancestor-dir`, `walker-top-level-dir`, `walker-each-token`, `comment-only-line` |
| Project readers: the one test, or its whole project? | Per test | unchanged; the case `project-reader` now also shows its neighbour is not selected |
| May gatekeeper's own tests install vitest? | Yes, in one test workspace | decision 7; the gate `select-vitest` in S2 |

Two things in the walker rule are this lane's reading, both wider than the answer's words and never narrower.
The ancestor-directory half applies to every changed file, not only one outside every project: a repo whose
root is itself a project has no such file. The token list also takes the source selector's `scandir`, `fdir`
and `execSync`.

## `.drover/arch.toml` lines

This design converts no hot file and adds no central list. S1 adds, in the PR that creates the cases folder:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/tests/select"
glob = "*/case.toml"

[[ban]]
pattern = "runner::"
paths = ["crates/gatekeeper-engine/src/select/rules.rs", "crates/gatekeeper-engine/src/select/readers.rs", "crates/gatekeeper-engine/src/select/split.rs"]
reason = "the selection rules are pure: evidence in, a selection out"

[[ban]]
pattern = "fs::"
paths = ["crates/gatekeeper-engine/src/select/rules.rs", "crates/gatekeeper-engine/src/select/readers.rs", "crates/gatekeeper-engine/src/select/split.rs"]
reason = "the selection rules are pure: evidence in, a selection out"
```

S2 adds, in the PR that creates `select/vitest.rs`:

```toml
[[ban]]
pattern = ".bin/vitest"
paths = ["crates/*/src/**"]
except = ["crates/gatekeeper-engine/src/select/vitest.rs"]
reason = "one module finds and runs the repo's vitest"
```

S3 adds, in the PR that creates the split cases:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/tests/split"
glob = "*/case.toml"
```

A consumer adds nothing: its gates are already one file each under `[[collect]] dir = ".gatekeeper/gates"`.
