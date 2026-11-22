# Per-file test selection for TypeScript (#46)

## Summary

Decision: a test gate may ask for per-file selection. gatekeeper then runs only the test files a change can
reach: those whose imports reach a changed file (asked of the repo's own vitest), the always-run guards, tests
that read a changed file by path, and a deleted file's project. A file outside every project no longer runs
everything. Shards cut one list by recorded time; the result cache keeps one record per job.
Slices: the rules and their cases; vitest and the run; shards and timings; the replay report; one adoption each.
Open questions: project readers, the one test or its whole project? May gatekeeper's own tests install vitest?

## Read in this order

| File | What |
|---|---|
| `usage.md` | a gate's `[select]` table, a lane's run, the CI summary, the replay against a consumer's own selector |
| `adr.md` | the question, what the code says first, the options, the eight decisions, the criteria mapped, consequences, the slices, the two open questions, the `.drover/arch.toml` lines |
| `shapes.md` | the gate table, the change, the evidence and the selection, each rule's exact wording, the split, the report, the key, the replay report |
| `boundaries.md` | where the code goes, which module may call which, the statecharts of a selecting gate's run and of a timings entry |
| `cases.md` | each union member, each widening and each full-suite rule with the named case that proves it |

## The design on one screen

```
 the change (base … tree)                     the gate's test files (vitest list)
 ────────────────────────                     ───────────────────────────────────
 not a change · a `full` file · .gatekeeper/  ─────────────────────────────────▶  all
 any changed file    ─▶ closure: vitest list --changed <base>          ┐
                     ─▶ always: under `always` · in no project         │
                     ─▶ readers: its path · its basename · a walker    ├─ union ─┬─ empty ─▶ all
 a touched project   ─▶ project readers: a test that names its root    │         └─ files, each with its reasons
 a deleted non-test  ─▶ widened: its project · its import specifier    ┘

 the list ─▶ split by recorded time ─▶ shard i/n ─▶ the gate's run line, the files as arguments
             (a lane: 1/1, all of it)               └▶ vitest's JSON report ─▶ each file's time

 times:  a push to the integration branch writes one entry · `plan` names the newest · every shard reads that one
 check:  every shard reports the list's digest · `done` fails a gate whose shards disagree
```

| Today | After |
|---|---|
| a workflow file in the change runs every project's tests | it runs the guards and the tests that read it |
| a change in one project runs that project and every project that needs it | it runs the tests whose imports reach the changed files |
| shards are cut by path | shards are cut by the time each file took on the integration branch |
| "why did this test run" has no answer | `selection.json` names every rule that selected it |
| a consumer keeps its own selector and its own proof | one table in a gate file; the rules and their cases are gatekeeper's |

## What it never does

- Run nothing because nothing was selected: an empty selection is the full suite.
- Narrow after a failure: a selection that cannot be computed fails the gate.
- Let shards run different lists unseen: `done` fails the run.
- Let a pull request write the times another run splits by.
- Change a gate that has no `[select]` table.

## What the owner decides

Approve, revise or reject, and answer the two open questions (`adr.md`, near the end): how to read "project
readers", and whether gatekeeper's own repo may install vitest for one test gate. The design holds either way;
each answer changes one rule or one gate.
