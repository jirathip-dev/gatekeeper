# gatekeeper: data shapes (#1)

## The reusable-workflow contract

```yaml
# .github/workflows/gates.yml
on:
  workflow_call:
    inputs:
      packs:        # a subset of the configured packs, space separated; "" = all. It narrows, never adds.
        type: string
        default: ""
      runners:      # the runner label (or JSON list of labels) per platform
        type: string
        default: '{"linux": "ubuntu-latest", "macos": "macos-latest"}'
      scope:        # "auto": a pull request runs what its change can reach, any other event everything.
        type: string    # "full": everything. (G10)
        default: auto
    secrets:
      private-names:    # the hygiene list, one name a line; a public repo's same-repo PR fails without it
        required: false
```

A caller writes `uses: jirathip-dev/gatekeeper/.github/workflows/<file>@<40-hex commit> # v<version>` and passes
each secret by name. `core/pin` checks the line.

| Job | Runs on | What |
|---|---|---|
| `plan` | linux | `./.gatekeeper/run ci plan`: emits the matrix, one entry per planned leg and shard |
| `leg` (matrix) | the leg's platform | stamps its start; provisions the leg's toolchains and cache; `./.gatekeeper/run check --leg <leg> [--shard i/n]`; uploads one artifact, `gates-<leg>[-<i>]`: `report.json`, the logs, the gates' kept files |
| `leg-after` (matrix, G10) | the leg's platform | the legs that name an `after` leg, once that leg passed; its kept files are restored first |
| `done` | linux | `./.gatekeeper/run ci done`: merges the reports, writes the summary table, fails on any `FAIL`, a planned job with no report, or a gate over budget on a GitHub-hosted runner |

A leg is a pack (`core`, `rust`, …, and `local` for the repo's own gates) unless a gate names another.

The other three workflows. Each declares its secrets by the names the repos' environments use today:

| Workflow | Inputs | Secrets |
|---|---|---|
| `release.yml` | `bin`, `sha`, `branch`, `targets` (default linux x86_64 and macOS arm64), `build-env`, `version-check`, `allow-partial`, `prune-days` (14), `environment` (`release`), `key` (`release.pub`), `tag` (default: the contract's `build-…` form), `publish` (true) | `MACOS_CERT_P12`, `MACOS_CERT_PASSWORD`, `ASC_API_KEY_P8`, `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID`, `RELEASE_SIGNING_KEY` |
| `testflight.yml` | `project`, `scheme`, `platforms`, `upload`, `sign` (`export` or `archive`), `xcode`, `runner`, `prepare` (a script of the caller's, run first), `environment` (`testflight`) | `ASC_KEY_P8`, `ASC_KEY_ID`, `ASC_ISSUER_ID` |
| `supabase-deploy.yml` | `target` (`dev` or `prod`), `dir` (`supabase`), `environment` | `SUPABASE_ACCESS_TOKEN` |

What `release.yml` publishes is farrier's release contract, not restated here: `<bin>-<triple>` executables,
`SHA256SUMS`, and `SHA256SUMS.minisig` whose trusted comment is `<owner>/<repo> <tag> <sha40>`.

## The consumer's config

```rust
/// `.gatekeeper/config.toml`, parsed once at the edge; an unknown key is an error.
struct Config {
    packs: BTreeSet<PackId>,
    integration: BranchName,
    pin: Pin,
    budgets: BTreeMap<GateId, Secs>,
    tools: BTreeMap<ToolId, ToolSource>,
    scope: Option<ScopeConfig>,
    legs: BTreeMap<LegId, Leg>,
    rust: Option<RustPack>,
    swift: Option<SwiftPack>,
    ts: Option<TsPack>,
    supabase: Option<SupabasePack>,
}

enum Pin {
    Release { version: Version, sha256: BTreeMap<Target, Sha256> },
    Checkout,
}

enum PackId { Core, Rust, Swift, Ts, Supabase, Local }
struct GateId { pack: PackId, name: GateName }

struct RustPack { manifest: PathBuf }
struct SwiftPack { packages: Vec<PathBuf>, projects: Vec<XcodeProject>, sources: Vec<PathBuf>, tokens: Vec<PathBuf> }
struct TsPack { projects: Vec<PathBuf>, install: String }
struct SupabasePack { dir: PathBuf }

enum ToolSource { Pinned, Run(String) }

struct ScopeConfig { graphs: BTreeSet<GraphKind>, ignore: Vec<Glob> }
enum GraphKind { Pnpm, Cargo, SwiftPm }
struct Leg { platform: Platform, setup: BTreeSet<Toolchain>, needs: BTreeSet<Capability>, after: Option<LegId> }
```

`Pin::Checkout` is valid only in a repo whose workspace builds `gatekeeper`. `[pin]` holds no commit: the binary
knows the commit it was built from, and `core/pin` compares that with each `uses:` line. `ToolSource::Run` is for
the one repo that builds a pinned tool itself: drover checks its own tree with its own build. A pack listed in
`packs` needs its table (`rust` needs none: the manifest defaults to `Cargo.toml`); a table without its pack is an
error. `scope` and `legs` arrive with G10; an `after` leg must itself have no `after`.

## A gate, a pack, a pinned tool

```toml
# packs/rust/gates/clippy.toml (a local gate in .gatekeeper/gates/ has the same shape)
about = "clippy pedantic; the banned APIs are denied outside tests"
fix = "return the error; only tests may unwrap, panic, index or print"
budget_secs = 90
paths = ["**/*.rs", "**/Cargo.toml", "Cargo.lock"]
lock = "cargo"

[[step]]
run = "cargo clippy --workspace --all-features --lib --bins --locked -- -D warnings"

[[step]]
run = "cargo clippy --workspace --all-features --tests --locked -- -D warnings {test_allow}"
```

```rust
struct Gate {
    id: GateId,
    about: String,
    fix: String,
    budget: Secs,
    scope: Scope,
    platform: Platform,
    setup: BTreeSet<Toolchain>,
    needs: BTreeSet<Capability>,
    steps: Vec<Step>,
    reads_pr: bool,
    uploads: Vec<Upload>,
    leg: LegId,
    shards: Shards,
}

enum Step {
    Run(String),
    Tool { tool: ToolId, args: Vec<String> },
    Check(CheckId),
}

enum Scope { Always, Paths(Vec<Glob>), Projects(Vec<Glob>) }
struct Upload { name: UploadName, paths: Vec<Glob> }
struct Shards { hosted: NonZeroU32, self_hosted: NonZeroU32 }

enum Platform { Any, Linux, Macos }
enum Toolchain { Rust, Node, Swift }
enum Capability { Containers }
```

`Run` is a command of the repo's toolchain. `Tool` is a tool gatekeeper pins and fetches itself (drover, farrier,
actionlint, cargo-deny, oxlint, knip, anti-slop-swift). `Check` is logic built into the engine (a floor check, the
fixture run, the token lint, the pin check), one file per check.

A gate file has `paths` or `projects`, never both; with neither it always runs. `leg` defaults to the gate's pack
and `shards` to one. `Scope::Projects`, `leg` and `shards` arrive with G10; `uploads` with G4. In a `run` line,
`{projects}` is the affected project roots the gate's globs match and `{shard}` is `i/n`.

```toml
# packs/rust/pack.toml
about = "fmt, clippy with the shared lints, cargo-deny, the banned-API fixtures"
platform = "linux"
setup = ["rust"]
```

```toml
# packs/core/tools/actionlint.toml
version = "1.7.12"
[asset.x86_64-unknown-linux-gnu]
url = "https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz"
sha256 = "…"
```

A fixture case is a folder: files laid over the pack's `_base` repo, plus `expect`, a line the failure must print.

```
packs/rust/fixtures/_base/               a small repo that passes every rust gate
packs/rust/fixtures/bans/floor-missing/  Cargo.toml without `unwrap_used = "deny"`; expect: `unwrap_used`
packs/rust/fixtures/bans/clean/          no files: the base must pass
```

## Affected projects (G10)

```rust
struct Graph { projects: BTreeMap<ProjectId, Project> }
struct Project { root: RepoPath, needs: BTreeSet<ProjectId> }

enum Trigger { Change(BTreeSet<RepoPath>), Everything(Widened) }
enum Affected { All(Widened), Projects(BTreeSet<ProjectId>) }
enum Widened { NotAPullRequest, Forced, NoBase, OutsideEveryProject(RepoPath) }

fn affected(graph: &Graph, trigger: Trigger, ignore: &[Glob]) -> Affected
```

- A pull request and a lane's run are a `Change` against the base; a push, a schedule and `scope: full` are
  `Everything`.
- A project is affected when a changed file is under its root, or when it needs an affected project.
- A changed file under no project and matched by no `ignore` glob widens the run to all, and the report names the
  file.
- `affected` is pure: the graph is read from the workspace manifests once, at the edge (one reader per
  `GraphKind`), and passed in.

## The plan and the report

```rust
struct Plan { base: Option<Rev>, affected: Affected, jobs: Vec<Job> }
struct Job { leg: LegId, shard: Option<Shard>, after: Option<LegId>, gates: Vec<Planned> }
struct Shard { index: NonZeroU32, of: NonZeroU32 }
struct Planned { gate: Gate, decision: Decision }
enum Decision { Run, Skip(Skip), CannotRunHere(Platform) }
enum Skip { OutOfScope, NoPullRequest, NotPublic, ForkPullRequest, CiOnly(Capability) }

struct Report { version: Version, jobs: Vec<JobReport> }
struct JobReport { leg: LegId, shard: Option<Shard>, host: Host, setup: Secs, gates: Vec<GateReport> }
struct GateReport { id: GateId, outcome: Outcome, took: Secs, budget: Secs, log: PathBuf, kept: Vec<Kept> }
struct Kept { name: UploadName, files: Vec<PathBuf> }

enum Host { GithubHosted { warm_cache: bool }, SelfHosted, Lane }
enum BudgetMode { Enforce, Report }

enum Outcome {
    Passed { slow: bool },
    Failed(Failure),
    OverBudget,
    Skipped(Skip),
}

enum Failure { Exit(i32), Timeout, MissingTool(String), CannotRunHere(Platform) }
```

`Plan` is a pure function of the config, the packs, the changed files, the project graph and the host. CI and a
lane call the same function, so they plan the same gates; only the shard count differs (a lane's host plans every
sharded gate as `1/1`). `BudgetMode` is `Enforce` for `GithubHosted { warm_cache: true }` and `Report` for every
other host, so `OverBudget` exists only there; elsewhere a gate over budget is `Passed { slow: true }`.

`setup` is the time from the job's start to its first gate: checkout, toolchains, cache restore, the fetch. The
job's first step stamps the start; on a lane's host it is the fetch alone. `report.json` is the `Report`; the job
summary is one table of it:

```
| job        | host          | setup | gate         | took | budget |        |
| rust       | github-hosted |  38 s | rust/clippy  | 12 s |   90 s | ok     |
| local 2/4  | self-hosted   |  95 s | local/unit   | 310 s|  300 s | slow   |
```

## Verbs

Every verb is started through the wrapper: `./.gatekeeper/run <verb>`.

| Verb | What |
|---|---|
| `check [--pack P] [--leg L] [--shard I/N] [--gate ID] [--base REV] [--pr-body FILE] [--after-reports DIR] [--all]` | plan and run; `--all` ignores scope; `--after-reports` holds the downloaded artifacts of the leg an `after` job waits for |
| `list [--json]` | the planned gates with their budgets |
| `pin <version>` | check that release's signed list, then write `[pin]`, every `uses: …@<sha> # v<version>` line and the wrapper |
| `baseline <gate>` | rewrite a gate's baseline; after the first write it only shrinks |
| `skill --to <dir>` | write the usage skill of this version |
| `selftest [--pack P]` | every gate against its fixtures |
| `ci plan`, `ci done` | the workflow's two calls; `plan` prints `matrix=` (jobs that wait for none) and `after=` (jobs that wait for one leg) |
| `release sign --identifier <bin> <file>` | codesign, notarize, verify (the release workflow's) |
| `supabase status`, `supabase apply --target dev\|prod` | the migration ledger |

Exit codes: 0 every planned gate passed; 1 a gate failed or ran over an enforced budget; 2 usage or a config error.

## What a consumer repo holds

```
.gatekeeper/config.toml          packs, pin, budgets, the packs' tables
.gatekeeper/run                  the wrapper; written by `pin`, never by hand
.gatekeeper/gates/<name>.toml    its own gates, one a file
.gatekeeper/baseline/<gate>.txt  known findings, shrink-only
.gatekeeper/tokens-allowed.txt   the swift token lint's allow-list (swift pack with `tokens`)
```

Nothing else of gatekeeper's is copied into a consumer.
