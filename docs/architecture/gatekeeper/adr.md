# ADR: gatekeeper, one gate kit for every fleet repo (#1)

## Summary

Decision: gatekeeper is one small program and four shared workflows. A repo lists its packs in one file; the program
runs every gate the same way on a lane's host and in CI, and becomes the fleet's done check. The shared rules ship
inside the program, so consumer repos keep no copies; each pins an exact version and may be stricter, never looser.
Slices: the engine with the core and rust packs, gating itself; releases and pinning; the skill; hygiene and PR body
(after drover's new verbs); the ts, swift and supabase packs; TestFlight; then one adoption per repo, smallest first.
Open questions: the private-names list as a CI secret; database tests CI-only; one install per host; a 4,000-line cap.

## Status

Proposed, 2026-11-20. Nothing is built. Issues are filed after approval.

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
| B. A commit sha in `uses:` only | No. It pins the workflow, not the program a lane runs. |
| C. An exact tag, with the version and checksums in the repo's config | **Yes.** One pin, read by the lane's host and by the runner. |

**4. Where a repo's own gates live**

| Option | Verdict |
|---|---|
| A. Extra jobs in the repo's `ci.yml` | No. CI-only steps are how a lane meets a red CI it could not have seen. |
| B. One file per gate in `.gatekeeper/gates/` | **Yes.** They run in the same command and the same job. |

**5. How a gate's time is checked**

| Option | Verdict |
|---|---|
| A. `timeout-minutes` on jobs | No. It includes setup, kills without a number, and is invisible locally. |
| B. The program times each gate against a budget in the gate's file | **Yes.** Same number everywhere, a table in the summary, a fail in CI. |

## Decision

1. **The workflow contract.** `gates.yml` takes two inputs, `packs` (narrows the configured packs) and `runners`
   (a label per platform), and one secret, `private-names`. Everything else comes from `.gatekeeper/config.toml`,
   because a lane's host cannot read workflow inputs. Three jobs: `plan`, one `pack` job per planned pack, `done`.
   `gates / done` is the one required check. Shapes: `shapes.md`.
2. **Versioning and pinning.** Tags `vMAJOR.MINOR.PATCH`, never moved. MAJOR: the config or a workflow input
   changed shape. MINOR: a gate was added or made stricter. PATCH: a fix. The consumer's `[pin]` holds the version
   and the sha256 of each platform's binary; `core/pin` fails when a `uses:` ref differs. `gatekeeper pin <version>`
   writes both. A release reaches a repo only through a bump PR, where the new gates run first.
3. **The one local entry point.** `gatekeeper check` plans from the config, the packs and the changed files, runs
   every planned gate, and reports each with its time. CI calls the same verb, once per pack. The pinned binary is
   fetched and verified by the host's `gatekeeper` (a lane) or the bootstrap (a runner) from the same `[pin]`. It
   becomes the done check through drover: where a repo has `.gatekeeper/config.toml`, drover's default done check
   is `gatekeeper check`, in place of today's arch-check and fmt defaults (D4).
4. **Shared config, and a repo's extras.** By reference and floor checks, no synced copies (option 2C). A repo
   adds extras in three ways: stricter settings in its own files; its own gates, one file each in
   `.gatekeeper/gates/`; and a shrink-only baseline for findings that predate adoption. Nothing loosens a floor: a
   floor that is wrong is changed in gatekeeper, for everyone.
5. **Module layout, and what moves where.** Three crates (model, engine, cli) and `packs/<pack>/` with one file per
   gate, one folder per fixture case (`boundaries.md`). drover concepts stay drover's and are called as verbs
   through a pinned drover: the line cap, arch-check, the rules file, hygiene and the PR body. gatekeeper holds
   the rest. The full table is `inventory.md`.
6. **Wall-time budgets.** Each gate file carries `budget_secs`; a pack's budget is the sum (core 68 s, rust 150 s,
   ts 180 s, swift 400 s, supabase 10 s plus 300 s for pgTAP). The program measures each gate's own wall time and
   writes `report.json`; `done` prints the table and fails a gate over budget. The budget is enforced in CI on a
   warm cache, waived and reported on a cold one, and only reported on a lane's host. A repo may set a gate's
   budget in `[budgets]`, in a diff the reviewer sees. Per gate: `packs.md`.
7. **gatekeeper gates itself.** Its `ci.yml` calls `./.github/workflows/gates.yml` at the same commit, and its pin
   is `checkout`, so each PR is gated by the gatekeeper that PR builds. It runs the core and rust packs with no
   pedantic lint allowed, plus two local gates: its tests, and `selftest`, which runs every gate of every pack
   against its fixtures and fails a gate that has no failing case. Its releases are cut by its own `release.yml`.
8. **Adoption order and issues.** Below.

## The agent-speed criteria, mapped

| Criterion | Mechanism | What enforces it |
|---|---|---|
| 1. One gate, same everywhere | `gatekeeper check` is CI's only step and drover's default done check; one pinned binary and one pinned set of tools on both sides; the plan is one pure function | the arch-check ban on gate commands in `gates.yml`; each consumer freezes its `ci.yml`, so a new CI-only step fails; `core/pin` |
| 2. A skill | `skill/gatekeeper/`, generated from each gate's `about` and `fix`; `gatekeeper skill --to <dir>` | arch-check `generated`: a gate changed without `just gen` fails |
| 3. Fix once | no pack file is copied into a consumer; each adoption PR deletes the copies in `inventory.md` | the adoption issue's AC lists the files to delete; rev checks the diff |
| 4. Smaller blast radius | `ci.yml` falls to about 14 lines and the justfile's gate recipes to one; a repo's gates are one file each | `[[collect]] dir = ".gatekeeper/gates"` in each consumer |
| 5. Fast | a budget per gate, timed by the program, failed in CI; pinned tools download in seconds, none is built from source; a pack no changed file touches is skipped | `gatekeeper ci done`; the engine's own 200 ms perf assert |

## Consequences

- A gate fix is one PR in gatekeeper, a release, then a one-command bump PR per repo. Bumps are small but there
  are N of them; an automatic bump PR can come later.
- gatekeeper pins one drover build. A new drover verb reaches the gates only after a drover release and a
  gatekeeper release. drover's own repo runs the core pack with the drover it builds.
- Lanes need `gatekeeper` on each host's PATH once. After that, versions fetch themselves into a cache.
- A check on a host of the wrong platform fails for an in-scope gate instead of skipping it. Swift issues must be
  queued to a macOS host.
- A repo's other pull-request workflows fold into `.gatekeeper/gates/` on adoption. Scheduled, pages and deploy
  workflows stay.
- Budgets make slowness a failure. A repo that grows past a budget raises it in its own config, in the open.
- The supabase scripts stay Node, so gatekeeper holds some JavaScript and runs the ts pack on itself from G8.
- Production stays behind each repo's GitHub environments. The deploy, release and TestFlight workflows change
  who holds the YAML, not who may run it.
- Secrets reach the three shared workflows through the caller's environment. The first adopter proves it with a
  dry run; a caller in another organisation may have to name each secret.

## Issues, in order

gatekeeper:

| # | Issue | Needs |
|---|---|---|
| G1 | The engine (`check`, plan, report, budgets, fixtures, `selftest`), the core pack without hygiene and PR body, the rust pack, `gates.yml`, CI on itself | none |
| G2 | `release.yml` and release signing; gatekeeper's own releases; `gatekeeper pin`, the pinned-version fetch, the install script | G1 |
| G3 | The skill and `gatekeeper skill --to` | G1 |
| G4 | Path scope in the plan; shrink-only baselines | G1 |
| G5 | `core/hygiene` and `core/pr-body` | D2, D3 |
| G6 | The ts pack | G4 |
| G7 | The swift pack, with the prebuilt anti-slop tool | G2, G4 |
| G8 | The supabase pack: the migration check, the pgTAP runner, `supabase-deploy.yml`, apply and status | G4 |
| G9 | `testflight.yml` | G2 |

drover:

| # | Issue |
|---|---|
| D1 | `cap-check` counts the globs and exclusions named in `ci/cap.toml` (Rust, TS, Swift) and reads `ci/cap.d/` |
| D2 | `hygiene check` (tree, commit messages, PR body; exit 1; never prints a name) and `hygiene push` (the CI secret from the fleet's list) |
| D3 | `pr-body`: the done gate's PR-body rule as a verb CI can call; `lesson-check` also checks the rules file's format |
| D4 | The default done check is `gatekeeper check` where `.gatekeeper/config.toml` exists; `doctor` reports a host without `gatekeeper` |

Adoptions, one issue in each repo, each carrying the five criteria and its list of files to delete:

| Order | Repo | Why here | Needs |
|---|---|---|---|
| 1 | gatekeeper | it is gated from its first commit | G1 |
| 2 | swag | the smallest: Rust only, 55 lines of CI; proves the released binary and the pin | G2 |
| 3 | blocks | the strictest gate list and the most local gates; the source of the TS and token fixtures; its Swift half follows with G7 | G4, G6, D1 |
| 4 | shed | drops the 228 s drover build; first with hygiene and the shared release | G5, D1 |
| 5 | drover | the busiest repo, so it moves once the packs are proven; retires its `arch-check.yml` after cooee | G5 |
| 6 | cooee | the widest: Rust, Swift, its own gate runner, release, TestFlight; split into four issues | G5, G7, G9, D1 |
| 7 | beacon | Rust in a subfolder, Swift, the second TestFlight caller | G7, G9, D1 |
| 8 | the app repos | ts and supabase gates first; the deploy workflow last, each with the owner's approval | G6, G8 |

## Open questions for the owner

1. **The private-names list in CI.** It has to be a GitHub Actions secret in each public repo, written by
   `drover hygiene push` from the fleet's list. OK to store it there? The other choice is no hygiene gate in CI.
2. **Database tests.** The pgTAP gate needs a container and a lane's hook refuses containers. I made it CI-only
   on a lane's host, printed as such. The other choice is a host tag that allows containers for those lanes.
3. **One install per host.** Lanes need `gatekeeper` on PATH on every fleet host. That is a host change, so it is
   yours: one install command per host after G2.
4. **The line cap.** I propose 4,000 non-test Rust lines for gatekeeper to start.

## `.drover/arch.toml` lines

gatekeeper has no central list to freeze. Its three would-be lists never exist: gates and fixtures are globbed
from `packs/`, and the two Rust module lists and the skill are generated. G1 adds, in gatekeeper:

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
fails and is pointed at `.gatekeeper/gates/`; a pin bump edits a line and passes.
