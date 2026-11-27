# gatekeeper

One pinned gate command for every repo, the same locally and in CI: core, Rust, Swift, TS and Supabase packs;
affected projects, shards and a result cache; signed releases, TestFlight and Supabase deploy workflows. Not a build
orchestrator.

## What it is

A pinned binary behind a short shell wrapper, `.gatekeeper/run`. A developer runs `./.gatekeeper/run check` locally,
`drover turn done` runs the same command as its done check, and CI runs it through the reusable `gates.yml`. The
wrapper reads the `[pin]` table, fetches that release's binary for the host, checks its sha256 and runs it. Nothing
is installed on the host or the runner. Every gate runs even after one fails, so one run shows every failure. Full
logs go to `target/gatekeeper/`.

## Intent

- One gate kit for every repo: the packs hold the lints, bans and checks that each repo used to copy.
- The same command everywhere: a lane, a laptop and CI run one binary at one version over one plan.
- Fix once: a gate fixed or tightened in gatekeeper reaches every repo with its next pin bump.
- Fast, with budgets: only the projects a change can reach, a result cache for passes, a wall-time budget per gate.

## Non-goals

- Not a build orchestrator like Nx or Turborepo. There is no task graph between targets.
- No build-output cache. The result cache records passes and restores no files; a gate that keeps files is never
  cached.
- No generators, no daemon or watch mode, no distributed execution.

## Adopt it

The wrapper, `.gatekeeper/run`, is the same bytes in every repo on one version: copy it from this repo at the tag.
Then `.gatekeeper/config.toml`:

```toml
packs = ["core", "rust"]          # core, rust, swift, ts, supabase
integration = "staging"           # the branch pull requests target

[pin]
version = "1.1.0"
x86_64-unknown-linux-gnu = "5cc4e98a0140ee0e190752bd141d74cf7790c943e7846078fbba4dddeaca85e4"
aarch64-apple-darwin = "61be2d06fb38d761348f289c4b5c8e1c43be4b346c3fc00f22b5019e2fa9d959"

[cache]                           # optional: turns the result cache on
recheck = 1
```

The `ts` pack needs a `[ts]` table (`projects`, `install`); the `swift` pack a `[swift]` table (`sources`, and
`packages` or `projects`). `core/deps` reads `.gatekeeper/deps.toml`: each direct dependency with its reason.

A gate that cannot pass at adoption, even with a baseline, is turned off alone in `[off]`, each with its reason;
the rest of its pack still runs:

```toml
[off]
"rust/clippy" = "pre-adoption pedantic findings; own issue"
"core/deps" = "dependency reasons not written yet; own issue"
```

`check`, `list` and the `gates / done` summary show each one as `off: <reason>`. An entry with an empty reason, a gate
no configured pack has, or `core/pin` is a config error.

`.github/workflows/ci.yml`, the whole file. The workflow is named by its commit; the tag is a comment:

```yaml
name: ci
on:
  push:
    branches: [main, staging]
  pull_request:
    types: [opened, synchronize, reopened, edited]

permissions:
  contents: read

jobs:
  gates:
    uses: jirathip-dev/gatekeeper/.github/workflows/gates.yml@c76c64fa790a1a5e5e608c25139960e74db8b44d # v1.1.0
    secrets:
      private-names: ${{ secrets.PRIVATE_NAMES }}
```

Each secret is passed by name; `core/pin` refuses `secrets: inherit`. Branch protection needs one check,
`gates / done`, whatever the packs. Freeze `ci.yml` so a CI-only step can never come back: in `.drover/arch.toml`,

```toml
frozen = [".github/workflows/ci.yml"]
```

Findings that predate adoption go in a baseline that only shrinks: `./.gatekeeper/run baseline ts/oxlint` writes
`.gatekeeper/baseline/<pack>-<gate>.txt`, here `ts-oxlint.txt`. A listed finding passes, a new one fails, and a fixed one must leave the list.

## Commands

`./.gatekeeper/run <verb>` passes its arguments to the pinned `gatekeeper`.

| Command | What it does |
|---|---|
| `check` | Runs every gate in scope. `--pack <p>`, `--gate <pack/name>`, `--leg <name>`, `--shard <i>/<n>`, `--all` (every project, no cache reads), `--no-cache` (reads and writes nothing), `--base <rev>`, `--pr-body <file>` |
| `list` | Each gate, its budget and its cache key or why it is not cached. `--pack`, `--gate`, `--json` |
| `pin <version>` | Checks that release's signed checksums and rewrites `[pin]`, the `uses:` lines and the wrapper |
| `baseline <pack/name>` | Lists a gate's current findings once; after that it only drops the fixed ones |
| `skill --to <dir>` | Writes the lane skill for the pinned version: each gate's `about`, its fix and how to rerun it |
| `selftest` | Every gate of every pack fails each of its fixtures and passes its clean one. `--pack` |
| `ci plan`, `ci done` | Used by `gates.yml`: the job matrix (`--pack`, `--runners`, `--all`) and the final verdict over the jobs' reports (`--reports`) |
| `supabase status`, `supabase apply --target dev\|prod` | The migration ledger, by the supabase pack's scripts |
| `--version` | The version and the commit it was built from |

Exit 0 when every gate passed, 1 on any `FAIL`, 2 on a usage error.

## Packs

Budgets are seconds of a gate's own wall time.

| Pack | Platform | Gates (budget) | Total |
|---|---|---|---|
| core | linux | `pin` (1), `rules` (5), `arch` (45), `hygiene` (5), `pr-body` (2), `actionlint` (5), `deps` (5), `docs` (10) | 78 |
| rust | linux | `fmt` (10), `clippy` (180), `bans` (30), `deny` (20) | 240 |
| swift | macOS | `anti-slop` (30), `warnings` (360), `tokens` (10, any platform) | 400 |
| ts | linux | `tsc` (90), `oxlint` (30), `knip` (60) | 180 |
| supabase | linux | `migrations` (10), `pgtap` (300, needs containers: CI only on a lane's host) | 310 |

What each gate runs and the fixture cases that prove it fails: `docs/architecture/gatekeeper/packs.md`.

## The docs gate

`core/docs` is on in every repo with the core pack, with no config. It reads `README.md`, `docs/**/*.md`,
`skill/**/*.md`, `CLAUDE.md`, `AGENTS.md` and `.drover/*.md`. A backticked path must exist (a missing one under a
folder git ignores passes), a markdown link must resolve, and a name a set claims must be in that set. A design
record (`docs/architecture/**`, `docs/design/**`) gets the link check only. Agent text (`skill/**/*.md`) must match a
`generated` glob of `.drover/arch.toml`. A finding has no line number, so `baseline core/docs` lists old hits and an
edit above one changes nothing. No key removes a default path; the way out is `[off]` with a reason. Every key of
`[docs]` in `.gatekeeper/config.toml` is optional:

| Key | Meaning |
|---|---|
| `also` | more docs, beside the defaults |
| `skip` | backticked spans that are not read |
| `watch` | source files whose added or removed keys need a changed doc line (or `no doc: <why>` in the PR body) |
| `generated` | more agent text that must be listed as generated |
| `[docs.records]` | `glob = "reason"`: more records, links only |
| `[docs.written]` | `glob = "reason"`: agent text written by hand |
| `[docs.names]` | `run` (a command) or `file` (a generated file) printing `<set> TAB <name> [TAB <description>]` lines, and `[docs.names.sets]`: each set's prefixes, as `"core/{name}"` |

An excuse that matches no file is a finding. In a set whose lines carry descriptions, a name with none and two names
with one description are findings. This repo's own sets come from `scripts/names.sh`: its verbs and each pack's
gates.

## Speed

**Affected projects.** With `[scope]`, a pull request runs a `projects` gate over the projects its change can reach
and every project that needs them. `graphs` names the workspace graphs to read (`cargo`, `pnpm`, `swiftpm`);
`ignore` lists files that widen nothing. A file outside every project, a missing base, `--all`, a push or a
schedule widens the run to everything.

```toml
[scope]
graphs = ["pnpm"]
ignore = ["docs/**", "**/*.md"]
```

**Legs and shards.** A gate runs in its pack's job unless it names a `leg`; `[legs.<name>]` (`platform`, `setup`,
`after`) is an extra CI job, and `after` waits for one leg and receives its kept files. `shards = { hosted = 4,
self-hosted = 2 }` splits a gate into that many CI jobs (`{shard}` in its `run`); `gates / done` fails when a
planned shard has no report. A lane's host runs it as one shard.

**Result cache.** `[cache]` turns it on. A gate's key is the sha256 of its input files (those `paths` matches, or
its `projects` and the projects they reach, plus `.gatekeeper/**`; with neither, every tracked file), its toolchain
versions (`setup`, `versions`), its pinned tools, its gate file, the platform and the gatekeeper version. A hit
skips the gate and prints `cached` with the run that proved it. A failure is never remembered. Keys:

- `recheck` (default 10): one hit in that many is rerun; a rerun that fails deletes the record and fails loudly.
  Start at 1 to measure.
- `max_age_days` (default 14): an older record is refused.
- `pull_requests` (default `"own"`): a pull request reads the integration branch's records and its own and writes
  its own; `"read"` writes nothing.

A push to the integration branch reads nothing and writes the integration records. A fork's pull request, any other
push, a schedule and `--no-cache` read and write nothing; `--all` reads nothing. A lane uses a folder on its host.
A `projects` gate run unsharded keeps one record per project, so a change reruns only the projects it reaches; a
sharded job keeps one record for its whole set. Never cached: `cache = false`, a gate that reads the base or the
PR body, keeps files, or has a `run` step with no `setup` or `versions`.

**Budgets.** A gate over its `budget_secs` fails only on a GitHub-hosted runner with a warm cache; elsewhere it is
marked `slow` and passes. A repo may override one in `[budgets]` (`"rust/clippy" = 240`).

## Repo-local gates

Each `.gatekeeper/gates/<name>.toml` is gate `local/<name>`, in `check` and in CI from that commit:

```toml
about = "unit tests of the projects this change can reach"
fix = "fix the failing test the log names"
run = "pnpm vitest run --shard={shard} {projects}"
budget_secs = 300
projects = ["apps/*", "packages/*"]    # or paths = ["web/**"]: what it reads
versions = ["node --version"]          # part of the cache key

[[upload]]                             # kept files, pass or fail
name = "coverage"
paths = ["target/coverage/*.json"]
```

Other keys: `[[step]]` instead of `run`, `setup` (`rust`, `node`), `needs` (`containers`, `private-names`),
`platform`, `leg` (whose `[legs.<name>]` may set `after`), `shards` and `cache`. Kept files land in
`target/gatekeeper/files/<name>/` and, in CI, in the job's `gates-<leg>` artifact.

## Versioning and pinning

Releases are tagged `vMAJOR.MINOR.PATCH`. Each one carries the binary per target, `SHA256SUMS` and its minisign
signature `SHA256SUMS.minisig`, checked against `release.pub`. `[pin]` holds the version and each target's sha256;
the wrapper refuses a binary that differs. To bump:

```
$ ./.gatekeeper/run pin 1.6.0
```

It checks the signature, rewrites `[pin]`, the `uses: …gates.yml@<sha> # v<version>` lines and the wrapper. Then
run `check`: a stricter gate can turn the repo red only in that one PR. `core/pin` fails a `uses:` line whose sha
is not the pinned build's commit or whose comment is not its version, and an edited wrapper.

Upgrading from v1.1.1 or older: that release's `pin` cannot pin, so run the new release's own binary once at the
repo root (`t` is `aarch64-apple-darwin` or `x86_64-unknown-linux-gnu`):

```
$ t=aarch64-apple-darwin
$ base=https://github.com/jirathip-dev/gatekeeper/releases/download/v1.1.2
$ curl -sSfLO "$base/gatekeeper-$t" -sSfLO "$base/SHA256SUMS"
$ grep " gatekeeper-$t\$" SHA256SUMS | shasum -a 256 -c
$ chmod +x "gatekeeper-$t"
$ ./gatekeeper-$t pin 1.1.2
$ rm "gatekeeper-$t" SHA256SUMS
```

After that `./.gatekeeper/run pin` works as usual.

Upgrading to v1.3.0: `rust/bans` now also reads each crate's own `[lints]` table (one that is not
`workspace = true`) against the floor, so a crate that set its own weaker lints turns the bump PR red. Restore the
floor there, or, for a crate that needs `unsafe`, name it in `[rust] unsafe_crates` (see the rust pack's
`rust/bans` rules).

Upgrading to v1.4.0: `core/pr-body`, `core/cap`, `core/rules`, `core/hygiene` and `core/arch` run inside
gatekeeper, and the core pack no longer pins the drover tool. A config whose `[tools]` names `drover`, or a local
gate whose `run` line uses `{tool:drover}`, is refused: drop the line. A private repo adds an `[off]` line for
`core/hygiene`.

Upgrading to v1.5.0: `core/docs` is on with no config. Run `./.gatekeeper/run baseline core/docs` and commit
the list of old hits it writes (no file when there is no hit). A `[docs]` table in `.drover/arch.toml` is
refused: move `watch` and `skip` to `[docs]` in `.gatekeeper/config.toml` and drop `paths` (an entry outside the
defaults goes to `also`). A skill page that is not generated is listed in `generated` or named in `[docs.written]`.

Upgrading to v1.6.0: the line cap is removed. `core/cap` is gone and nothing reads `ci/cap.toml`, `ci/cap.d/` or a
`Cap:` trailer; a repo that still has them passes, and may delete them. An `[off]` line for `core/cap` names a gate
no pack has, so drop it. File size is watched by the `.drover/arch.toml` size rules.

## Workflows

All are `workflow_call`, called by commit with the tag as a comment.

| Workflow | What it does | Inputs |
|---|---|---|
| `gates.yml` | Plans the jobs, runs `check` per pack, leg and shard, then `gates / done` | `packs`, `runners`, `scope` (`auto`, `full`); secret `private-names` |
| `release.yml` | Builds, signs and publishes a binary release with `SHA256SUMS.minisig` | `bin`, `sha`, `branch`, `targets`, `build-env`, `version-check`, `allow-partial`, `prune-days`, `environment`, `key`, `tag`, `publish` |
| `testflight.yml` | Archives an Xcode project and uploads it to TestFlight | `project`, `scheme`, `platforms`, `upload`, `sign`, `xcode`, `runner`, `prepare`, `environment` |
| `supabase-deploy.yml` | Runs `supabase/migrations`, then applies migrations to dev or prod | `target`, `environment`; secret `SUPABASE_ACCESS_TOKEN` |

## Docs

- `docs/architecture/gatekeeper/`: the design (`adr.md`), the callers (`usage.md`), the packs (`packs.md`), the
  types (`shapes.md`) and the module boundaries (`boundaries.md`).
- `docs/architecture/result-cache/`: the result cache's design, usage and cases.
- The lane skill: `./.gatekeeper/run skill --to <dir>` writes it for the pinned version; this repo's copy is
  `skill/gatekeeper/`.
