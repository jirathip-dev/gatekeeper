# Per-file test selection: how the callers read (#46)

Read this first. These are the call sites once the slices land; `adr.md` says why, `shapes.md` has the types.
Every name here is new. Paths, shas, counts and run numbers are made up.

## A gate asks for it

```toml
# .gatekeeper/gates/unit.toml
about = "unit tests this change can reach"
fix = "fix the failing test the log names"
budget_secs = 300
setup = ["node"]
shards = { hosted = 4, self-hosted = 2 }
run = "pnpm vitest run --reporter=default --reporter=json --outputFile={report} {files}"

[select]                              # the table turns per-file selection on for this gate
vitest = "vitest.config.ts"           # the one vitest config this gate's run line uses
always = ["tests/guards/**"]          # repo-wide guards: selected on every run
full = ["pnpm-lock.yaml", "tsconfig.base.json"]   # a change to one of these runs every test
```

- `{files}` is the selected test files of this job's shard. They are passed as arguments, one each, never
  spliced into the line. `{report}` is where the line writes vitest's JSON report; gatekeeper reads each file's
  time from it. A sharded `[select]` gate without `{report}` is refused when the gate file is parsed.
- One gate is one vitest run under one config. A repo with three configs writes three gates; they may share a leg.
- `full` is required, even when empty: the repo says in a reviewed line which files no import graph can follow.
  `.gatekeeper/**` is always in it.
- `[select]` needs `[ts]` (its `install` runs before the selection) and `[scope] graphs` (the projects). It
  replaces `projects` on the gate; `paths` still works and is checked first.

Nothing else is configured. The consumer's workflow file does not change.

## A lane's run

```
$ ./.gatekeeper/run check --gate local/unit
gatekeeper 1.4.0 · legs local · base 3f9c2ab8e1d0 · 3 files changed · host lane · cache reads ["host"] writes ["host"]
ok     local/unit          48.2s /  300s
      selected 37 of 2140 test files in 6.1s: closure 21 · always 9 · readers 5 · widened 2
      list: target/gatekeeper/local-unit.selection.json
1 gates passed · setup 0.2s
```

Other first lines a lane can see:

```
      selected all 2140 test files: `pnpm-lock.yaml` changed (full)
      selected all 2140 test files: no rule selected a test
      selected all 2140 test files: --all or scope full
```

`selection.json` names each selected file and every rule that selected it. It is how a lane answers "why did
this test run" and "why did that one not":

```json
{ "gate": "local/unit", "base": "3f9c2ab8e1d0", "universe": 2140, "digest": "a1b2c3d4…",
  "files": {
    "apps/web/src/price.test.ts": ["closure"],
    "packages/tools/src/config.test.ts": [{ "reads": { "basename": "supabase/config.toml" } }],
    "packages/ui/src/button.test.ts": [{ "widened": { "former-project": "packages/ui/src/old.ts" } }],
    "tests/guards/licence.test.ts": [{ "always": "path" }] } }
```

A failing test is rerun with the same command: the same tree and base select the same files.

## CI

`gates.yml` gains three steps and no condition; the consumer's `ci.yml` is untouched.

| Job | New step | Decided by |
|---|---|---|
| `plan` | look up the newest timings entry (no download) and hand its exact name to `ci plan` | `ci plan` puts that name in each sharded `[select]` leg's matrix entry |
| each shard | restore the timings entry by that exact name; `check --timings <dir>` | the matrix entry: an empty name means no step |
| `done` | `ci done --timings-out <dir>`, then save the folder | `ci plan` says whether this run writes: only a push to the integration branch |

Each shard works out the same selection from the same commit, base and timings, then runs its own slice:

```
| job      | host          | setup | gate       | took   | budget |    | note                                    |
| unit 1/4 | github-hosted |  41 s | local/unit | 52.0 s |  300 s | ok | 10 of 37 selected (2140) · select 6.0 s |
| unit 2/4 | github-hosted |  39 s | local/unit | 49.5 s |  300 s | ok |  9 of 37 selected (2140) · select 6.2 s |
| unit 3/4 | github-hosted |  40 s | local/unit | 50.1 s |  300 s | ok |  9 of 37 selected (2140) · select 5.9 s |
| unit 4/4 | github-hosted |  44 s | local/unit | 51.3 s |  300 s | ok |  9 of 37 selected (2140) · select 6.1 s |

- selected by local/unit: 37 of 2140 test files: closure 21 · always 9 · readers 5 · widened 2
```

`done` fails the run when the shards of one gate did not work from the same list:

```
FAIL local/unit: its shards selected different lists (1/4 a1b2c3d4, 3/4 9e8f7a6b); rerun all jobs
```

A shard no file fell into prints `skip`, "no selected test file fell in this shard", and passes.

## Measuring it against a consumer's own selector

The consumer writes the gate file on a branch and one line per pull-request run it wants replayed:

```
# runs.jsonl
{"pr": 4101, "base": "3f9c2ab8…", "head": "88e01d4c…", "failed": []}
{"pr": 4102, "base": "51d0e77a…", "head": "7a1b3c9d…", "failed": ["apps/web/src/price.test.ts"]}
```

`failed` is the test files that failed in that run, from the consumer's own CI reports. A failure that escaped to
the integration branch is one more line: the pull request's merged head, with the tests the full run saw fail.

```
$ ./.gatekeeper/run replay --gate local/unit --runs runs.jsonl --other 'node scripts/select-tests.mjs --base {base}'
replay local/unit · 32 runs of 30 pull requests · 2140 test files
| pr   | head    | changed | gatekeeper          | other      | only gatekeeper | only other | failed | missed: gatekeeper | other |
| 4101 | 88e01d4 |       3 | 37                  | 41         |               2 |          6 |      0 |                  0 |     0 |
| 4102 | 7a1b3c9 |       1 | 12                  | all (2140) |               0 |       2128 |      1 |                  0 |     0 |
| 4107 | 51d0e77 |      14 | all: pnpm-lock.yaml | all (2140) |               0 |          0 |      0 |                  0 |     0 |
totals: gatekeeper 3412 file runs (5.0 %), 2 full · other 9655 (14.1 %), 9 full · missed: gatekeeper 0, other 0
by rule: closure 2204 · always 640 · readers 512 · project readers 18 · widened 38
wrote target/gatekeeper/replay/replay.json and replay.md
```

- For each line `replay` checks the head out into a scratch worktree, runs `[ts] install`, selects against the
  merge base, and runs `--other` in the same worktree. That command prints the test files its selector would
  run, one a line; every file of the suite printed means "all".
- `replay.json` holds the same rows with the lists: each file only one selector chose, and each missed failure
  with the run it failed in. It exits 1 when gatekeeper missed a failure.
- The gate replaces the consumer's selector when, over at least 30 merged pull requests: gatekeeper missed no
  failure, selected no more file runs in total, and went to the full suite no more often. The adoption issue
  quotes the totals line and reads every file in `only other`.
