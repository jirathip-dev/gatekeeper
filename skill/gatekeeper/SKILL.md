---
name: gatekeeper
description: Read, fix and rerun a failing gatekeeper gate (gatekeeper 1.7.0).
---

# gatekeeper 1.7.0

`./.gatekeeper/run check` is what CI runs and what a lane runs. A failing gate prints `FAIL  <pack>/<gate>`, its log's tail, its `fix:` line, its log path and the command that reruns it alone.

1. Read the gate's entry in its pack file below: what it checks and how to fix it.
2. Fix the cause, never the gate: no pack file is copied into the repo or edited there.
3. Rerun the one gate: `./.gatekeeper/run check --gate <pack>/<gate>`, then `./.gatekeeper/run check`.

`./.gatekeeper/run list` prints the planned gates with their budgets and why one is skipped. A repo's own gates live in `.gatekeeper/gates/<gate>.toml` as `local/<gate>` and print their `fix` the same way.

With `[cache]` in `.gatekeeper/config.toml` a gate whose inputs already passed prints `cached` with the run that proved it.
A rerun of a cached pass that fails now prints `cached pass does not hold`: the gate reads a file it does not declare; add it to the gate's `paths` or set `cache = false`.
`--no-cache` runs and records nothing.
A `projects` gate keeps one record per project, over that project, every project it needs and every file outside all projects that `[scope] ignore` keeps: a change reruns the projects it reaches, and the row says how many ran and how many were cached.
A sharded CI job keeps one record for its whole set of projects and is cached only while that set and its files are unchanged.

The code ratchets only fall: `rust/dead-code`, `rust/complexity`, `ts/complexity` and `ts/knip` keep a list in `.gatekeeper/baseline/<pack>-<gate>.txt`; `core/duplication` and `core/surface` keep a ceiling, one file per lowering in `ci/ratchet/<count>.d/<date>-<n>.toml`, the lowest wins.
`./.gatekeeper/run baseline <pack>/<gate>` drops a list's gone lines, or adds a lower ceiling file; a second run writes nothing.
`core/ratchet` fails a change that adds a list line, or edits, deletes or adds a ceiling file that is not lower, unless a commit carries `Ratchet: <pack/gate or count> <old> → <new> (#issue, q<N>)` with the owner's decision; a change that moves `[pin]` may add list lines.
`[rust.exported]` in `.gatekeeper/config.toml` names files whose `pub` items are used from outside the workspace, each `glob = "reason"`: `rust/dead-code` skips them.
`[duplication.ignore]` names more paths jscpd does not read, each `glob = "reason"`; fixtures, snapshots, vendored folders and the arch file's generated files are skipped already.
`[surface] names` lists sets of the `[docs.names]` list, each set's names one count; `[surface.lines]` maps a count's name to globs whose non-blank lines it counts. No `[surface]` table counts nothing.
No key changes a threshold, a clone's minimum size or a step: those are the packs'.

A gate with `[select]` runs only the test files a change can reach, passed to its `run` line in place of `{files}`.
Its row prints `selected N of M test files in Xs:` with a count per rule (closure, always, readers, project-readers, widened), or `selected all M test files:` and why, then `list: target/gatekeeper/<pack>-<gate>.selection.json`.
Why did a test run, or not?
That file names each selected file with every rule that selected it: `closure` (vitest lists its imports reaching a changed file), `always`, `reads` (its code names a changed file's path or basename, or it walks that file's project or a directory above it), `names-project`, `widened` (a deleted file's project or import specifier).
Rerun a failing test with the same command: the same tree and base select the same files.
A selection that cannot be computed fails the gate with its `.select.log`; it never runs fewer tests.
A sharded `[select]` gate writes vitest's JSON report to `{report}`; each CI shard selects the same list and runs its slice, split by the times the last push to the integration branch recorded.
`ci done` fails a gate whose shards selected different lists (rerun all jobs), and a shard no file fell into prints `skip` and passes.
`./.gatekeeper/run replay --gate <pack>/<gate> --runs runs.jsonl --other '<cmd> {base}'` measures it against another selector: each line of `runs.jsonl` is `{"pr", "base", "head", "failed"}`, both select in a scratch worktree of the head, and it prints per run the counts, the files only one chose and each missed failure, then the totals, into `target/gatekeeper/replay/`; it exits 1 when gatekeeper missed a failure.

A repo's own test gate with `[timed]` names its `runner` (`nextest` or `vitest`) and the `junit` file its `run` line leaves; the line carries `{only}`, empty on the first run and the runner's own filter on a rerun.
Each test has `[tests] budget_secs` in `.gatekeeper/config.toml` (default 2): a test over it that is not on the gate's slow list, or is listed and changed, runs twice more and fails at its fastest in CI; on a lane's host it is a warning.
The tests a change touches run three times; runs that differ are a `flaky` finding.
The slow list is the gate's baseline file, `.gatekeeper/baseline/local-<gate>.txt`, written by `./.gatekeeper/run baseline local/<gate>`; it only shrinks, and a listed test under half the budget fails the gate until the same command drops it.
A test that failed, then passed on a rerun on the integration branch gets an entry in `ci/tests/flaky.d/<slug>.toml` (`test`, `since`, `sha`, `issue`): for 7 days its failures are warnings, then `core/test-health` fails until the test is fixed or deleted and the entry with it.
A gate with `[coverage]` (`tool` is `llvm-cov` or `vitest`, `summary` the file its line leaves) gives `needs = ["coverage"]`: CI runs it, and it fails more than 0.1 point under the integration branch's number. A deliberate drop is a file in `ci/tests/coverage.d/<date>-<gate>-<floor>.toml` (`gate`, `floor`, `why`) whose commit carries a `Coverage:` trailer.
`[tests] soak` names whole test files that run only in a gate with `when = "nightly"`, on the workflow's schedule event; a soak file's test in a timed gate's JUnit file fails that gate.
`[tests] test`, `src` and `exclude` override which lines `core/test-health` counts as test and as src.

A lane's check never asserts a time: perf, soak, bench and wall-clock limits are CI's.
A gate that measures time gives `needs = ["quiet-host"]`: a lane's host skips it with `CI only on a lane's host: needs a quiet host`, and CI runs it.
A test inside an ordinary gate reads `GATEKEEPER_HOST`, `lane` or `ci`: on `lane` it asserts the outcome and prints the time, on `ci` it asserts the limit too.
A gate's `run` that reads `CI` or `GITHUB_ACTIONS` to skip itself is a config error: use `needs = ["quiet-host"]`.

## Packs

- [core](packs/core.md): the line cap, the rules file, arch-check, actionlint, the dependency list
- [rust](packs/rust.md): fmt, clippy with the shared lints, cargo-deny, the banned-API fixtures
- [swift](packs/swift.md): the prebuilt anti-slop-swift, warnings as errors under GATEKEEPER_STRICT=1, the raw colour and size lint
- [ts](packs/ts.md): tsc strict, oxlint with the bans and the anti-slop rules, knip's unused-code ratchet
- [supabase](packs/supabase.md): the migration files' names, order and policy rule, and pgTAP on a local stack (CI only on a lane's host)

## Pinned tools

A gate's `run` line names one as `{tool:<name>}`: gatekeeper fetches it by version, checks its sha256 and puts the cached path there, on a lane's host and on a runner alike. A tool with no upstream binary is built once per version by gatekeeper's `tools` workflow; never build it from source in a repo's CI.

- `actionlint` 1.7.12
- `farrier` git-04d7a80, built by gatekeeper for x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin (`bin/farrier`)
- `jscpd` 5.4.3
- `libde265` 1.0.16, built by gatekeeper for x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin (`lib/pkgconfig/libde265.pc`)
- `sqlx-cli` 0.8.6, built by gatekeeper for x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, aarch64-apple-darwin (`bin/sqlx`)
- `cargo-deny` 0.22.0
- `cargo-llvm-cov` 0.9.2
- `cargo-machete` 0.10.1
- `cargo-nextest` 0.9.152
- `supabase` 3.2.0
- `anti-slop-swift` 1.0.0, built by gatekeeper for aarch64-apple-darwin (`bin/anti-slop`)
- `knip` 6.47.0
- `oxlint` 1.94.0
