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
    secrets:
      private-names:    # the hygiene list, one name a line; a public repo's same-repo PR fails without it
        required: false
```

| Job | Runs on | What |
|---|---|---|
| `plan` | linux | gets the pinned binary, runs `gatekeeper ci plan`, emits the matrix |
| `pack` (matrix: one per planned pack and platform; a repo's own gates are the pack `local`) | the pack's platform | provisions the pack's toolchains and cache, runs `gatekeeper check --pack <pack>`, uploads `report.json` |
| `done` | linux | `gatekeeper ci done`: merges the reports, writes the summary table, fails on any `FAIL` or over-budget gate |

The other three workflows:

| Workflow | Inputs | Secrets (from the caller's environment) |
|---|---|---|
| `release.yml` | `bin`, `sha`, `branch`, `targets` (default linux x86_64 and macOS arm64), `build-env`, `version-check`, `allow-partial`, `prune-days` (14), `environment` (`release`) | `MACOS_CERT_P12`, `MACOS_CERT_PASSWORD`, `ASC_API_KEY_P8`, `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID` |
| `testflight.yml` | `project`, `scheme`, `platforms`, `upload`, `sign` (`export` or `archive`), `xcode`, `runner`, `prepare` (a script of the caller's, run first), `environment` (`testflight`) | `ASC_KEY_P8`, `ASC_KEY_ID`, `ASC_ISSUER_ID` |
| `supabase-deploy.yml` | `target` (`dev` or `prod`), `dir` (`supabase`), `environment` | `SUPABASE_ACCESS_TOKEN` |

## The consumer's config

```rust
/// `.gatekeeper/config.toml`, parsed once at the edge; an unknown key is an error.
struct Config {
    packs: BTreeSet<PackId>,
    integration: BranchName,
    pin: Pin,
    budgets: BTreeMap<GateId, Secs>,
    tools: BTreeMap<ToolId, ToolSource>,
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
```

`Pin::Checkout` is valid only in a repo whose workspace builds `gatekeeper`. `ToolSource::Run` is for the one repo
that builds a pinned tool itself: drover checks its own tree with its own build. A pack listed in `packs` needs
its table (`rust` needs none: the manifest defaults to `Cargo.toml`); a table without its pack is an error.

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
    paths: Vec<Glob>,
    platform: Platform,
    setup: BTreeSet<Toolchain>,
    needs: BTreeSet<Capability>,
    steps: Vec<Step>,
    reads_pr: bool,
}

enum Step {
    Run(String),
    Tool { tool: ToolId, args: Vec<String> },
    Check(CheckId),
}

enum Platform { Any, Linux, Macos }
enum Toolchain { Rust, Node, Swift }
enum Capability { Containers }
```

`Run` is a command of the repo's toolchain. `Tool` is a tool gatekeeper pins and fetches itself (drover,
actionlint, cargo-deny, oxlint, knip, anti-slop-swift). `Check` is logic built into the engine (a floor check, the
fixture run, the token lint, the pin check), one file per check.

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

## The plan and the report

```rust
struct Plan { base: Option<Rev>, gates: Vec<Planned> }
struct Planned { gate: Gate, decision: Decision }
enum Decision { Run, Skip(Skip), CannotRunHere(Platform) }
enum Skip { OutOfScope, NoPullRequest, NotPublic, ForkPullRequest, CiOnly(Capability) }

struct Report { version: Version, gates: Vec<GateReport> }
struct GateReport { id: GateId, outcome: Outcome, took: Secs, budget: Secs, log: PathBuf }

enum Outcome {
    Passed,
    Failed(Failure),
    OverBudget,
    Skipped(Skip),
}

enum Failure { Exit(i32), Timeout, MissingTool(String), CannotRunHere(Platform) }
enum BudgetMode { Enforce, Report, Waive }
```

`Plan` is a pure function of the config, the packs, the changed files and the host. CI and a lane call the same
function, so they plan the same gates. `report.json` is the `Report`; the job summary is a table of it:
`| gate | took | budget |`.

## Verbs

| Verb | What |
|---|---|
| `gatekeeper check [--pack P] [--gate ID] [--base REV] [--pr-body FILE] [--all]` | plan and run; `--all` ignores scope |
| `gatekeeper list [--json]` | the planned gates with their budgets |
| `gatekeeper pin <version>` | write `[pin]` and every `uses: …@v<version>` line |
| `gatekeeper baseline <gate>` | rewrite a gate's baseline; after the first write it only shrinks |
| `gatekeeper skill --to <dir>` | write the usage skill of this version |
| `gatekeeper selftest [--pack P]` | every gate against its fixtures |
| `gatekeeper ci plan`, `gatekeeper ci done` | the workflow's two calls |
| `gatekeeper release sign --identifier <bin> <file>` | codesign, notarize, verify (the release workflow's) |
| `gatekeeper supabase status`, `gatekeeper supabase apply --target dev\|prod` | the migration ledger |

Exit codes: 0 every planned gate passed; 1 a gate failed or ran over budget; 2 usage or a config error.

## What a consumer repo holds

```
.gatekeeper/config.toml          packs, pin, budgets, the packs' tables
.gatekeeper/gates/<name>.toml    its own gates, one a file
.gatekeeper/baseline/<gate>.txt  known findings, shrink-only
.gatekeeper/tokens-allowed.txt   the swift token lint's allow-list (swift pack with `tokens`)
```

Nothing else of gatekeeper's is copied into a consumer.
