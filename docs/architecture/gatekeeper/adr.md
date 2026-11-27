# ADR: gatekeeper, one gate kit for every fleet repo (#1)

## Summary

Decision: gatekeeper is one small program and four shared workflows. A repo lists its packs in one file; the program
runs every gate the same way on a lane's host and in CI, and is the fleet's done check. Repos keep no copies of the
rules: each pins one signed release, and a short script in the repo fetches it on any host, with nothing installed.
Slices: the engine with the core and rust packs, gating itself; signed releases and pinning; the skill; hygiene and
PR body; the ts, swift and supabase packs; TestFlight; shards and affected projects; then one adoption per repo.
Open questions: may gatekeeper tag its own releases by version (v1.2.0) where the release contract says build tags?

## Status

Accepted 2026-11-20 by the owner, after one revision (`README.md` maps each review point to its place). Nothing
is built. The issues below are filed now.

## Question

Every fleet repo hand-copies the same gates and the copies drift. How do the repos share one set of gates so that
a fix lands once, a lane sees locally exactly what CI will see, and each repo gets smaller?

## Where it stands (survey, 2026-11-20)

- The Rust ban set is the same 13 lints everywhere, written by hand in each `Cargo.toml`. `clippy.toml` exists six
  times. The 13 banned-API fixtures exist four times, their runner in four drifted versions.
- The line cap is five shell scripts that count different things. drover's `cap-check` counts only Rust.
- CI installs drover three ways: a pinned binary (seconds), a source build on every run (228 s in shed), or drover's
  reusable arch-check workflow at a floating branch.
- No repo has one command that is exactly CI. `just check` comes close, but the workflow files repeat each step by
  hand, with their own tool versions, and add steps `just check` lacks.
- No repo checks CI wall time. Perf budgets are hard asserts in tests, with a table in the job summary.
- The private-names list sits inside two of the repos.
- One large monorepo runs a 2,000-line workflow: a scope job that picks the affected projects, unit tests in 4
  shards (2 on its self-hosted runner), and database jobs that restore a snapshot an earlier job made.

## Options

**1. What gatekeeper is**

| Option | Verdict |
|---|---|
| A. Reusable workflows and shell scripts only | No. YAML cannot run on a lane's host, so local and CI stay two lists. |
| B. A composite action per gate, plus a script that mirrors them | No. Two implementations of every gate is today's drift again. |
| C. A program that owns every gate; the workflow only provisions and calls it | **Yes.** One implementation, two callers. |

**2. How shared config reaches a consumer**

| Option | Verdict |
|---|---|
| A. Synced copies plus a drift check | No. Every repo still holds every file; a bump touches them all; lanes can edit them. |
| B. A git submodule or subtree | No. Same copies, worse merges. |
| C. By reference, with floor checks | **Yes.** The program carries the files and hands them to the tools. A file a tool must find in the repo (`Cargo.toml` lints, `deny.toml`, `tsconfig.json`, `Package.swift`) stays hand-written and is checked against a floor. |

**3. How a consumer pins**

| Option | Verdict |
|---|---|
| A. A floating major tag (`@v1`) | No. A gatekeeper release could turn every repo red at once, with no PR to see it in. |
| B. An exact tag in `uses:` | No. A tag can be moved; the workflow a repo runs must be named by its content. |
| C. A commit sha in `uses:` with the tag as a comment, and the version and checksums in the repo's config | **Yes.** `gatekeeper pin` writes both; `core/pin` fails when they name different releases. |

**4. Where a repo's own gates live**

| Option | Verdict |
|---|---|
| A. Extra jobs in the repo's `ci.yml` | No. CI-only steps are how a lane meets a red CI it could not have seen. |
| B. One file per gate in `.gatekeeper/gates/` | **Yes.** They run in the same command and the same job. |

**5. How a gate's time is checked**

| Option | Verdict |
|---|---|
| A. `timeout-minutes` on jobs | No. It includes setup, kills without a number, and is invisible locally. |
| B. The program times each gate against a budget in the gate's file | **Yes.** Same number everywhere, a table in the summary, a fail on a GitHub-hosted runner. |

**6. How a host gets the program**

| Option | Verdict |
|---|---|
| A. One install per host, on PATH | No. A host change for every host and every new one; a lane on a fresh host is red before it starts. |
| B. drover fetches it for the done check | No. Two fetchers to keep alike (drover's and the runner's), and no gate on a host without drover. |
| C. A wrapper script in the repo, the same file on a lane's host and a runner | **Yes.** It reads the pin, fetches the binary once into a cache, checks its sha256 and runs it. It is the one file copied into a consumer: `pin` writes it and `core/pin` compares it byte for byte. |

**7. How a monorepo's sharded, scoped CI fits**

| Option | Verdict |
|---|---|
| A. Its own workflow stays beside `gates.yml` for good | No. Two lists again, and its lanes keep a CI they cannot run. |
| B. gatekeeper generates a workflow per repo | No. A generated file per consumer to keep in step with every release. |
| C. The plan itself knows legs, shards and affected projects | **Yes**, as its own slice (G10). Until it lands, A: that repo is not frozen. |

## Decision

1. **The workflow contract.** `gates.yml` takes three inputs, `packs` (narrows the configured packs), `runners`
   (a label per platform) and `scope` (`auto` or `full`), and one secret, `private-names`. Everything else comes
   from `.gatekeeper/config.toml`, because a lane's host cannot read workflow inputs. Jobs: `plan`, one job per
   planned leg (a pack, or a leg the repo names) and shard, `done`. `gates / done` is the one required check. A
   caller names the workflow by commit sha with the tag as a comment, and passes each secret by name;
   `secrets: inherit` fails `core/pin`. Shapes: `shapes.md`.
2. **Versioning and pinning.** Tags `vMAJOR.MINOR.PATCH`, never moved. MAJOR: the config or a workflow input
   changed shape. MINOR: a gate was added or made stricter. PATCH: a fix. The consumer's `[pin]` holds the version
   and the sha256 of each platform's binary. `gatekeeper pin <version>` checks the release's signature, then
   writes `[pin]`, every `uses: …@<sha> # v<version>` line and the wrapper. `core/pin` checks that the sha in
   `uses:` is the commit the running binary was built from. A release reaches a repo only through a bump PR, where
   the new gates run first.
3. **The one local entry point.** `./.gatekeeper/run check` plans from the config, the packs and the changed
   files, runs every planned gate, and reports each with its time. CI calls the same file, once per leg. The
   wrapper fetches the pinned binary the same way on a lane's host and on a runner (option 6C); no host has
   gatekeeper installed. It becomes the done check through drover: where a repo has `.gatekeeper/run`, drover's
   default done check is `.gatekeeper/run check`, in place of today's arch-check and fmt defaults (D4).
4. **Shared config, and a repo's extras.** By reference and floor checks, no synced copies (option 2C). A repo
   adds extras in three ways: stricter settings in its own files; its own gates, one file each in
   `.gatekeeper/gates/`; and a shrink-only baseline for findings that predate adoption. Nothing loosens a floor: a
   floor that is wrong is changed in gatekeeper, for everyone. A gate may name files to keep (screenshots, a
   report): CI uploads them with the job's logs whether the gate passed or failed, and a lane sees their paths.
5. **Module layout, and what moves where.** Three crates (model, engine, cli) and `packs/<pack>/` with one file per
   gate, one folder per fixture case (`boundaries.md`). drover concepts stay drover's and are called as verbs
   through a pinned drover: the line cap, arch-check, the rules file, hygiene and the PR body. gatekeeper holds
   the rest. The full table is `inventory.md`.
6. **Wall-time budgets.** Each gate file carries `budget_secs`; a pack's budget is the sum (core 68 s, rust 240 s,
   ts 180 s, swift 400 s, supabase 10 s plus 300 s for pgTAP). The program measures each gate's own wall time and
   each job's setup time (job start to the first gate) and writes `report.json`; `done` prints both in one table.
   A gate over budget fails only on a GitHub-hosted runner with a warm cache. A self-hosted runner, a cold cache
   and a lane's host report the same numbers and never fail on them. Setup time is reported, not budgeted. A repo
   may set a gate's budget in `[budgets]`, in a diff the reviewer sees. Per gate: `packs.md`.
7. **gatekeeper gates itself.** Its `ci.yml` calls `./.github/workflows/gates.yml` at the same commit, and its pin
   is `checkout`, so each PR is gated by the gatekeeper that PR builds. It runs the core and rust packs with no
   pedantic lint allowed, plus two local gates: its tests, and `selftest`, which runs every gate of every pack
   against its fixtures and fails a gate that has no failing case. The one job beside its gates is a dry run of
   `release.yml` that publishes nothing: a workflow is proven only by running it. Its line cap starts at 4,000.
8. **Releases.** `release.yml` follows farrier's release contract: it builds each target, signs and notarizes
   the macOS ones, writes `SHA256SUMS`, signs that list, and runs farrier's release checker in each build job and
   again on the whole folder. Only then does it publish, every asset in one call. A release the checker would
   reject never exists. **farrier releases first**, with its own workflow, because `release.yml` needs a released
   checker. gatekeeper's v1.0.0 is the first release cut by `release.yml`; cooee, shed and drover follow in
   farrier's order, each before it adopts farrier's updater.
9. **A large monorepo.** Slice G10 adds three things to the plan. *Legs*: a repo names extra jobs, and a leg may
   run after one other and receive its kept files (a database snapshot). *Shards*: a gate splits into N jobs in CI
   and runs as one on a lane's host, over the same tests. *Affected projects*: the engine reads the workspace
   graph and the changed files and gives a gate the projects the change can reach; a file outside every project
   widens the run to everything. The selection engine and its tests live in gatekeeper. Until G10, that repo
   calls `gates.yml` beside its own workflow, which stays unfrozen.
10. **Adoption order and issues.** Below.

## Decided by the owner (review, 2026-11-20)

- The private-names list is an Actions secret in each public repo, written by `drover hygiene push`.
- The pgTAP gate is CI-only on a lane's host, and printed as such.
- No per-host install (decision 3). gatekeeper's line cap starts at 4,000.

## The agent-speed criteria, mapped

| Criterion | Mechanism | What enforces it |
|---|---|---|
| 1. One gate, same everywhere | `.gatekeeper/run check` is CI's only step and drover's default done check; one wrapper fetches one pinned binary and one pinned set of tools on both sides; the plan is one pure function | the arch-check ban on gate commands in `gates.yml`; each consumer freezes its `ci.yml`, so a new CI-only step fails; `core/pin` |
| 2. A skill | `skill/gatekeeper/`, generated from each gate's `about` and `fix`; `gatekeeper skill --to <dir>` | arch-check `generated`: a gate changed without `just gen` fails |
| 3. Fix once | no pack file is copied into a consumer; each adoption PR deletes the copies in `inventory.md` | the adoption issue's AC lists the files to delete; rev checks the diff |
| 4. Smaller blast radius | `ci.yml` falls to about 14 lines and the justfile's gate recipes to one; a repo's gates are one file each | `[[collect]] dir = ".gatekeeper/gates"` in each consumer |
| 5. Fast | a budget per gate, timed by the program, failed on a GitHub-hosted runner; setup time shown per job; pinned tools download in seconds, none is built from source; a gate no changed file reaches is skipped | `gatekeeper ci done`; the engine's own 200 ms perf assert |

## Consequences

- A gate fix is one PR in gatekeeper, a release, then a one-command bump PR per repo. Bumps are small but there
  are N of them; an automatic bump PR can come later.
- gatekeeper pins one drover build and one farrier build. A new drover verb reaches the gates only after a drover
  release and a gatekeeper release. drover's own repo runs the core pack with the drover it builds.
- A host needs only `curl`, a sha256 tool and network on a version's first run. After that the cache serves it.
- Each consumer holds one copied file, the wrapper. It changes only when `pin` rewrites it.
- gatekeeper's first release waits for farrier's checker (farrier's slice 3). Until then gatekeeper gates only
  itself, from its checkout. The owner adds the release secrets to gatekeeper's `release` environment before G2.
- A check on a host of the wrong platform fails for an in-scope gate instead of skipping it. Swift issues must be
  queued to a macOS host.
- A sharded gate runs unsharded on a lane's host: the same tests, a longer run. Affected scope keeps it short.
- A repo on a self-hosted runner gets its times reported and never failed. Its budgets bind only if it moves to
  GitHub-hosted runners.
- A repo's other pull-request workflows fold into `.gatekeeper/gates/` on adoption. Scheduled, pages and deploy
  workflows stay.
- Screenshots and other kept files sit in the job's one artifact (`gates-<leg>`, under `files/<name>/`), not in an
  artifact of their own name. A reader that opens `ui-390` today opens that folder.
- The supabase scripts stay Node, so gatekeeper holds some JavaScript and runs the ts pack on itself from G8.
- Production stays behind each repo's GitHub environments. The deploy, release and TestFlight workflows change
  who holds the YAML, not who may run it.
- Each shared workflow declares its secrets by name, under the names the repos' environments use today. The job
  names the environment, whose secret of that name is the one read. The first adopter proves it with a dry run.

## Issues, in order

gatekeeper:

| # | Issue | Needs |
|---|---|---|
| G1 | The engine (`check`, plan, report, budgets with runner kind and setup time, fixtures, `selftest`), the wrapper, the core pack without hygiene and PR body, the rust pack, `gates.yml`, CI on itself | none |
| G2 | `release.yml` under farrier's contract (signing, the signed list, the checker); gatekeeper's own releases; `gatekeeper pin` and `core/pin`'s sha check | G1, farrier's checker |
| G3 | The skill and `gatekeeper skill --to` | G1 |
| G4 | Path scope in the plan; shrink-only baselines; a gate's kept files | G1 |
| G5 | `core/hygiene` and `core/pr-body` | D2, D3 |
| G6 | The ts pack | G4 |
| G7 | The swift pack, with the prebuilt anti-slop tool (a recipe of #2) | G2, G4, #2 |
| G8 | The supabase pack: the migration check, the pgTAP runner, `supabase-deploy.yml`, apply and status | G4 |
| G9 | `testflight.yml` | G2 |
| G10 | Legs, shards and affected projects in the plan; the `scope` input | G4 |
| #2 | Prebuilt tool assets (filed already): recipes, built once per version, fetched like any pinned tool | G1, G2 |

farrier's adoption table calls G2 "G1".

drover:

| # | Issue |
|---|---|
| D1 | `cap-check` counts the globs and exclusions named in the cap file (retired, #127) (Rust, TS, Swift) and reads `ci/cap.d/` |
| D2 | `hygiene check` (tree, commit messages, PR body; exit 1; never prints a name) and `hygiene push` (the CI secret from the fleet's list) |
| D3 | `pr-body`: the done gate's PR-body rule as a verb CI can call; `lesson-check` also checks the rules file's format |
| D4 | The default done check is `.gatekeeper/run check` where that file exists |

Gate adoptions, one issue in each repo, each carrying the five criteria and its list of files to delete:

| Order | Repo | Why here | Needs |
|---|---|---|---|
| 1 | gatekeeper | it is gated from its first commit | G1 |
| 2 | swag | the smallest: Rust only, 55 lines of CI; proves the released binary, the wrapper and the pin | G2 |
| 3 | blocks | the strictest gate list and the most local gates; the source of the TS and token fixtures; its Swift half follows with G7 | G4, G6, D1 |
| 4 | shed | drops the 228 s drover build; first with hygiene | G5, D1 |
| 5 | drover | the busiest repo, so it moves once the packs are proven; retires its `arch-check.yml` after cooee | G5 |
| 6 | cooee | the widest: Rust, Swift, its own gate runner, TestFlight; split into three issues | G5, G7, G9, D1 |
| 7 | beacon | Rust in a subfolder, Swift, the second TestFlight caller | G7, G9, D1 |
| 8 | the smaller app repo | ts and supabase gates first; the deploy workflow last, with the owner's approval | G6, G8 |
| 9 | the app monorepo | first `gates.yml` for the gates that fit, beside its own workflow, unfrozen; then its test and database jobs as gate files, its workflow reduced to the call and frozen | G6, G8; then G10, #2 |

Release adoptions, one small issue in each repo, apart from its gates:

| Order | Repo | What | Needs |
|---|---|---|---|
| 0 | farrier | publishes with its own workflow and checks itself (its slice 3) | none of gatekeeper's |
| 1 | gatekeeper | v1.0.0, the first release through `release.yml` | G2 |
| 2 | cooee, then shed, then drover | each swaps its `release.yml` and signing script for the call, and starts publishing the signed list; its farrier-updater PR comes after | G2 |

## Open questions for the owner

1. **gatekeeper's own tags.** farrier's contract tags a release `build-<sha7>`, one per commit, for daemons that
   follow a branch. Nothing follows gatekeeper's branch: a repo pins a version. I propose gatekeeper's releases
   are tagged `v<version>` and follow the contract in everything else (assets, the signed list, the checker, which
   takes the tag as an argument). `release.yml` gets a `tag` input whose default is the contract's form, and
   farrier's contract gains one line allowing it. The other choice is build tags only, with the version as a
   second, asset-less tag on the same commit.

## `.drover/arch.toml` lines

gatekeeper has no central list to freeze. Its three would-be lists never exist: gates and fixtures are globbed
from `packs/`, and the two Rust module lists and the skill are generated. `.drover/arch.toml` holds these lines
from #1 on, except the first two, which G1 adds with the justfile whose `just gen` they run:

```toml
generated = ["crates/gatekeeper/src/cmd/mod.rs", "crates/gatekeeper-engine/src/check/mod.rs", "skill/gatekeeper/**"]
generate = "just gen"

[[collect]]
dir = "crates/gatekeeper/src/cmd"
glob = "*.rs"

[[collect]]
dir = "crates/gatekeeper-engine/src/check"
glob = "*.rs"

[[collect]]
dir = "packs"
glob = "*/gates/*.toml"

[[collect]]
dir = "packs"
glob = "*/fixtures/**"

[[layer]]
name = "model"
paths = ["crates/gatekeeper-model/src/**"]
mods = ["gatekeeper_model"]
may_use = []

[[layer]]
name = "engine"
paths = ["crates/gatekeeper-engine/src/**"]
mods = ["gatekeeper_engine"]
may_use = ["model"]

[[layer]]
name = "cli"
paths = ["crates/gatekeeper/src/**"]
may_use = ["engine", "model"]

[[ban]]
pattern = "Command::new"
paths = ["crates/*/src/**"]
except = ["crates/gatekeeper-engine/src/runner.rs"]
reason = "start processes through the engine's runner"

[[ban]]
pattern = "cargo "
paths = [".github/workflows/gates.yml"]
reason = "a gate runs through gatekeeper check; the workflow only provisions"

[[ban]]
pattern = "secrets: inherit"
paths = [".github/workflows/**"]
reason = "a workflow names each secret it reads"

[[size]]
paths = ["crates/*/src/**/*.rs"]
max_file_lines = 400
max_fn_lines = 60

[comments]
policy = "strict"
```

Each adoption PR adds, in the consumer, in the same PR that shrinks its `ci.yml`:

```toml
frozen = [".github/workflows/ci.yml"]

[[collect]]
dir = ".gatekeeper/gates"
glob = "*.toml"
```

A repo whose `frozen` list exists already appends the one path. From then on a PR that adds a line to `ci.yml`
fails and is pointed at `.gatekeeper/gates/`; a pin bump edits a line and passes. The app monorepo adds the
`[[collect]]` lines with its first step and the `frozen` line only with its second, after G10.
