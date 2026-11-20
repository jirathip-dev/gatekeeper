# A result cache: skip a gate whose inputs already passed (#17)

## Summary

Decision: gatekeeper keeps a small record of each gate that passed, named by a fingerprint of what the gate says
it reads: its files, its gate file, its tools' versions, the platform. Same fingerprint again: the gate is skipped
and printed `cached` with the run that proved it. Records sit in a folder on a lane's host and in GitHub's cache in
CI; pushes to the integration branch, full runs and forks never read, and one hit in ten is rerun.
Slices: the inputs and the key; records on a lane's host; CI; per-project records after G10; a measured adoption.
Open questions: may a pull request reuse its own earlier passes? Without it CI saves nothing path scope does not.

## Read in this order

| File | What |
|---|---|
| `usage.md` | a consumer's config, a gate's declaration, a lane's run, the CI summary, the measure against a framework |
| `adr.md` | the question, what the code says first, the options, the eight decisions, the criteria mapped, consequences, the issues, the open question, the `.drover/arch.toml` lines |
| `shapes.md` | the key, the record, which runs read and write, a gate's lookup, the report, the store on disk |
| `boundaries.md` | where the code goes, which module may call which, the statechart of a gate's run with the cache |
| `cases.md` | each decision and the engine case or test that proves it |

## The design on one screen

```
 a gate file                      the key = sha256 of
 ───────────                      ─────────────────────
 paths / projects   ───────────▶  the content of its input files (+ the pack's `always`, .gatekeeper/**)
 setup, versions    ───────────▶  each probe's output: rustc -Vv, node --version …
 [[step]] tool      ───────────▶  the pinned asset of each tool
 the file itself    ───────────▶  its bytes
                                  the platform · the gatekeeper that runs

 check ─▶ key ─▶ look up ─┬─ hit ──────────▶ cached   (the run that proved it)
                          ├─ hit, 1 in 10 ─▶ rerun ─▶ ok, or FAIL "cached pass does not hold"
                          └─ miss ─────────▶ run ───▶ passed ─▶ a record;  failed ─▶ nothing

 tier            written by                             read by
 integration     a push to the integration branch       pull requests from the repo
 pr-<n>          pull request n (the open question)     pull request n
 host            a lane's run                           lanes on that host; never CI
```

| Today | After |
|---|---|
| `check`, a one-line fix, `check`: every gate in scope runs twice | the second run reruns the gates whose inputs the fix touched |
| `drover turn done` reruns what the lane just ran | every gate that passed at that head is `cached` |
| a fix round's push reruns every gate the pull request ever touched | it reruns the gates that push touched |
| a consumer carries a build framework for its task cache | one table in `.gatekeeper/config.toml`, for every language, once measured to match |

## What it never does

- Remember a failure, or a pass that was itself a cache hit.
- Skip on a push to the integration branch, a schedule, a full run or a fork's pull request.
- Let one pull request's record reach another run.
- Cache a gate that has not pinned its tools, reads the base or the PR body, or keeps files.

## What the owner decides

Approve, revise or reject, and answer the one open question (`adr.md`, near the end): whether a pull request may
reuse its own earlier passes. The issue's text says pull requests only read; reading the code showed that then
CI skips exactly what path scope already skips, so I propose a tier each pull request writes and only it reads.
