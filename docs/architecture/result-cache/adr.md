# ADR: a result cache, skip a gate whose inputs already passed (#17)

## Summary

Decision: gatekeeper keeps a small record of each gate that passed, named by a fingerprint of what the gate says
it reads: its files, its gate file, its tools' versions, the platform. Same fingerprint again: the gate is skipped
and printed `cached` with the run that proved it. Records sit in a folder on a lane's host and in GitHub's cache in
CI; pushes to the integration branch, full runs and forks never read, and one hit in ten is rerun.
Slices: the inputs and the key; records on a lane's host; CI; per-project records after G10; a measured adoption.
Open questions: may a pull request reuse its own earlier passes? Without it CI saves nothing path scope does not.

## Status

Proposed 2026-11-20. Nothing is built. The issues below are filed once the owner approves.

## Question

A gate reruns work that already passed. How does gatekeeper skip a gate whose inputs already passed, for every
language, on a lane's host and in CI, without ever turning a run green that a full run would turn red?

Decided by the owner already: the cache is gatekeeper's own, and a consumer drops a framework's cache only once
this one is measured to match it.

## What the code says first

Path scope (G4) skips a gate when the change against the base touches none of its `paths`. A cache that only
pushes to the integration branch may write holds what the base proved. For a pull request these are the same
set: a gate's inputs equal the base's exactly when the change touched none of them. So that cache alone saves a
pull request nothing; affected projects (G10) are the same comparison per project. The time a cache can add comes
from records the base did not make:

| Where | The run that repeats | Who proved it |
|---|---|---|
| a lane's host | `check`, a fix, `check` again; `drover turn done` at the same head; the next lane on the host | an earlier run on that host |
| CI, a pull request's second push and every fix round | the gates the new push did not touch | that pull request's earlier run |
| CI, a rerun after one leg flaked | every other leg | the same |

The design is built around those three rows. The second and third need the open question answered yes.

## Options

**1. What a record is keyed by**

| Option | Verdict |
|---|---|
| A. The commit sha | No. One changed file misses every gate; a dirty lane tree has no sha. |
| B. A second list of inputs beside `paths` | No. Two declarations drift, and path scope already trusts `paths` to be complete. |
| C. The content of what the gate declares it reads, with its gate file, tool versions, platform and the gatekeeper that runs | **Yes.** `paths` (or `projects`) is the one declaration, for the skip and for the key. |

**2. How an undeclared read is caught**

| Option | Verdict |
|---|---|
| A. Trace the files a gate opens | No. A different mechanism per OS, needing rights a runner and a lane's host do not give. |
| B. Run each gate in a tree holding only its inputs | No, not now. It needs every install step repeated there, and a missing file can pass quietly. |
| C. Wide by default, narrow only by declaration, then rerun a sample of hits and fail on a difference; full runs on the integration branch never read | **Yes.** It catches exactly the case that matters, a result that differs, on every OS. |

**3. What is stored**

| Option | Verdict |
|---|---|
| A. The pass and the gate's kept files | No. Megabytes per record, and a later leg that needs a file would get an old one. |
| B. The pass and the failure | No. A remembered failure hides a fix of something undeclared; a failing gate should rerun. |
| C. The pass only; a gate that keeps files always runs | **Yes.** A record is a few hundred bytes. |

**4. Where records live in CI**

| Option | Verdict |
|---|---|
| A. A bucket or server the consumer runs | No. A service, credentials and a bill per consumer. |
| B. A folder on a self-hosted runner | No. Every pull request's job can write that folder. |
| C. GitHub's cache, for hosted and self-hosted runners alike | **Yes.** It is shared between runners already, and GitHub scopes each entry to the branch or pull request that wrote it. |

**5. How a forged record is stopped**

| Option | Verdict |
|---|---|
| A. Sign each record with a key only the integration branch holds | No. A key pair per consumer and a new dependency, to guard the tier that saves the least. |
| B. Check each record against its proving run through GitHub's API | No. A pull request that forges a record can forge that run's report too. |
| C. Trust where a record lies, not what it says: tiers GitHub keeps apart, and a record that does not fit its tier or its key is refused | **Yes.** |

**6. Who writes in CI**

| Option | Verdict |
|---|---|
| A. Every run, one shared tier | No. One pull request could turn another green. |
| B. Only pushes to the integration branch | Safe, and saves a pull request nothing (above). It is `pull_requests = "read"`. |
| C. B, and a pull request writes a tier only it can read | **Yes, proposed.** The open question. |

## Decision

1. **The key.** The sha256 of: the gate's id and gate file; the content of its input files; the output of its
   version probes (its `setup` toolchains', and the commands in its own `versions`); the pinned asset of each tool
   it names; the platform (target, and the runner image or OS version); the gatekeeper that runs, which carries
   the packs' files and the built-in checks. Inputs are `paths` plus the pack's `always` plus `.gatekeeper/**`;
   with no `paths`, every tracked file; with `projects`, one record per project over that project and all it
   needs. Files are hashed from the worktree, so a lane's dirty tree keys as what it is. A gate with nothing to
   pin its tools by, one that reads the base, the PR body, the network, the clock or a secret, and one that keeps
   files has no key and always runs. Shapes: `shapes.md`.
2. **Undeclared reads.** What the key cannot see: an ignored file, an environment variable, the network, the
   clock, anything outside the repo. Three nets. *Declaration*: a gate is narrow only where it says so, a pack's
   `always` adds what every gate of it reads, and `cache = false` is one line. *Recheck*: one hit in ten is rerun
   in the same tree; a failure fails the gate with "cached pass does not hold", names the proving run and deletes
   the record. *Full runs*: every push to the integration branch runs every gate and reads nothing, so a false
   green cannot outlive its merge unseen. Records also age out after 14 days.
3. **What is stored.** A pass that really ran: its key, the key's parts, the run that proved it, its time. Never a
   failure, never a file, never a pass that was itself a hit.
4. **Where.** The engine knows one thing, a folder with a subfolder per tier. On a lane's host it is beside the
   pinned tools, shared by every lane and repo on the host. In CI, `gates.yml` restores it from GitHub's cache and
   saves it back, on hosted and self-hosted runners alike. A consumer configures no place; it adds `[cache]`.
5. **Trust.** Three tiers: the integration branch's, a pull request's own, a host's. In CI GitHub keeps them
   apart: an entry saved by a pull request's run is readable by that pull request only, and one saved on the
   integration branch by every pull request into it. A fork's run reads and writes nothing. The engine refuses a
   record whose name is not the hash of its parts, whose parts are not this run's, or whose proof is not its
   tier's, and then runs the gate. A lane's host folder can turn only a lane's own check green; a merge needs CI,
   which never reads it.
6. **The second net.** Never read: a push (the integration branch writes; others do neither), a schedule,
   `scope: full`, `--all`, `--no-cache`, a fork, `selftest`. The recheck rate is `recheck` (default 10);
   `recheck = 1` reruns every hit, which is how a consumer starts.
7. **Printing and counting.** A row `cached`, the proving run's time in brackets, and the run that proved it. The
   last line and the CI summary count gates cached, time run, time saved, rechecks, rechecks that differed and
   records refused; `report.json` carries the same. `ci done` fails on a recheck that differed.
8. **The measure.** Both caches on the same 50 pull-request runs with `recheck = 1`: zero rechecks that differ,
   and a share of gate time saved at least the framework's; then a trial with the framework's cache off
   (`usage.md`).

The fixture or engine case for each: `cases.md`.

## The agent-speed criteria, mapped

| Criterion | Mechanism | What enforces it |
|---|---|---|
| 1. One gate, same everywhere | the same key function and lookup on a lane's host and in CI; the workflow only moves a folder, and `ci plan` tells it which | the ban on `github.event_name` in `gates.yml`; the `mode` unit test |
| 2. A skill | the generated skill gains the `cached` row, the "does not hold" failure and each gate's `cached: yes` or why not, from the gate files | arch-check `generated`: a gate file changed without `just gen` fails |
| 3. Fix once | no file is copied into a consumer; `always` lives in the pack; a consumer adds one table | `core/pin` (the wrapper); review of the adoption PR |
| 4. Smaller blast radius | `src/cache/` one concern a file; one folder a case; tiers and probes are enums, not lists | the `[[collect]]` line below |
| 5. Fast | a cached gate costs a lookup; inputs are hashed once a run through git, probes run once a run | the two 200 ms asserts in `tests/perf.rs` |

## Consequences

- With `pull_requests = "read"`, CI gains nothing path scope does not give; the lane's host still gains.
- A pull request that adds its own workflow can save a forged entry in its own tier, as it can already fake its
  own check. The cache gives it no way to turn another run green; review and the full run on the merge's push
  answer it. Signed records (option 5A) would not: the pull request's own tier has no key to sign with.
- A persistent self-hosted runner keeps no record folder, yet a pull request's job there can still leave
  anything on the host, as today. The cache does not widen that.
- A gate that reads an ignored file it did not generate from its inputs (a local `.env`) must set
  `cache = false`. The recheck finds the ones that forget, one hit in ten.
- A recheck that differs on a flaky gate reads as an undeclared input. The message names both runs; the gate is
  the thing to fix either way.
- A runner image update or a toolchain update misses everything once. That is the intent.
- A gate that keeps files (screenshots) always runs. If the measure shows that as the gap to a framework, storing
  kept files is its own decision.
- A sharded gate in CI hits only when its whole set of projects is unchanged. Dropping proven projects before
  the shards are cut needs keys in the plan job; it waits for the measure to ask for it.
- `rust/deny` is never cached while it fetches the advisory list.
- The build is about 700 lines of `src/`. gatekeeper's cap is 4,000; if the slices do not fit, the raise is its
  own issue.
- No new dependency: `sha2` and `serde_json` are in the engine already.

## Issues, in order

Filed after approval, each carrying the five criteria and its rows of `cases.md`:

| # | Issue | Needs |
|---|---|---|
| R1 | Declared inputs and the key: a pack's `always` (path scope uses it too), a gate's `versions` and `cache`, the worktree's file hashes, the probes, `Keyed`; `list` prints each gate's key or why it has none. Nothing is skipped yet. | G4 |
| R2 | Records on a lane's host: `[cache]`, the store, `lookup`, the recheck, the age limit, `Outcome::Cached`, the `cached` row and the totals line, `--no-cache`, the skill text, both perf asserts | R1 |
| R3 | CI: `Trigger` and `mode`, the tiers, `ci plan`'s two strings, restore and save in `gates.yml`, the `proved by` column and totals in `ci done`; gatekeeper turns it on for itself with `recheck = 1` | R2 |
| R4 | Per-project records for a `projects` gate, and a sharded job's set record | R2, G10 |
| R5 | One adoption per consumer: `[cache]` with `recheck = 1`, 50 runs, the three numbers, then `recheck = 10`; where a framework caches today, its removal is that repo's own PR after the numbers | R3; R4 for a monorepo |

## Open questions for the owner

1. **May a pull request reuse its own earlier passes?** The issue says pull requests only read. Built that way,
   CI skips only what path scope already skips, and the fix-round saving, the one a framework's cache gives, is
   lost. I propose `pull_requests = "own"` as the default: a pull request writes a tier that GitHub lets no other
   run read, the integration branch stays the only writer of what others read, and the integration push still
   runs everything. `"read"` is the strict reading and is one word in the config.

## `.drover/arch.toml` lines

This design adds no central list. R1 adds, in the PR that creates the cases folder:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/tests/cache"
glob = "*/case.toml"

[[ban]]
pattern = "env::var"
paths = ["crates/gatekeeper-engine/src/**", "crates/gatekeeper-model/src/**"]
reason = "the cli reads the environment once and passes typed values"
```

R3 adds, in the PR that gives `gates.yml` its restore and save steps:

```toml
[[ban]]
pattern = "github.event_name"
paths = [".github/workflows/gates.yml"]
reason = "ci plan decides which runs read and write the result cache; the workflow only moves the folder"
```

A consumer adds nothing: its gates are already one file each under `[[collect]] dir = ".gatekeeper/gates"`.
