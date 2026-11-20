# The result cache: data shapes (#17)

## What a consumer and a gate write

```rust
/// `[cache]` in `.gatekeeper/config.toml`; the table's presence turns the cache on.
struct CacheConfig {
    recheck: NonZeroU32,          // one hit in this many is rerun; 1 reruns every hit. Default 10.
    max_age_days: NonZeroU32,     // an older record is a miss. Default 14.
    pull_requests: PullRequests,  // default Own
}
enum PullRequests { Own, Read }

/// New keys of a gate file. `paths` (G4) and `projects` (G10) stay the one declaration of what a gate reads.
struct GateFile {
    // …
    versions: Vec<String>,   // commands whose output is part of the key
    cache: bool,             // default true; false for a gate that reads the network, the clock or a secret
}

/// New key of `pack.toml`: files every gate of the pack reads, for path scope and for the key alike.
struct PackFile {
    // …
    always: Vec<Glob>,
}
```

## The key

```rust
/// sha256 of the canonical JSON of `KeyParts`, in hex. It is the record's file name.
struct Key(Sha256);

struct KeyParts {
    schema: u32,                           // 1; a change of this shape misses everything
    engine: Engine,                        // which gatekeeper, and with it the packs' files and built-in checks
    gate: GateId,
    gate_file: Sha256,                     // the gate file's bytes
    unit: Unit,
    platform: PlatformId,
    versions: BTreeMap<String, String>,    // each probe command and its output: toolchains, then the gate's own
    tools: BTreeMap<ToolId, Sha256>,       // the pinned asset of each tool a step names, for this target
    inputs: Sha256,                        // of the sorted `mode blob path` lines of every input file
}

enum Engine { Release { version: Version, sha256: Sha256 }, Checkout { binary: Sha256 } }
struct PlatformId { target: String, os: String }   // the triple; the runner image and its version, else `uname -sr`

enum Unit {
    Gate,                                  // a gate with `paths`, or with none: one record
    Project(ProjectId),                    // a `projects` gate run unsharded: one record per project (G10)
    Shard { shard: Shard, projects: BTreeSet<ProjectId> },   // a sharded job: one record for exactly this set
}
```

What `inputs` covers, by a gate's scope:

| Scope | The input files |
|---|---|
| none | every tracked file |
| `paths` | the files its globs match, the pack's `always`, `.gatekeeper/**` |
| `projects`, per project | the project's root, the root of every project it needs, every file outside all projects that `[scope] ignore` does not match |

A file is hashed as it is in the worktree: git's blob id for a clean tracked file, the worktree's bytes for a
modified or new one. An ignored file is never an input.

A gate has no key, and always runs, when it has `cache = false`, takes `{--base}` or `{--pr-body}`, keeps files,
or has a `run` step with no `setup` and no `versions`:

```rust
enum Keyed { Key(Key, KeyParts), No(Unkeyed) }
enum Unkeyed { OptedOut, ReadsBase, ReadsPullRequest, KeepsFiles, NoVersion }
```

## The record

```rust
/// `<store>/<tier>/<key>.json`. There is no outcome field: only a pass that really ran is ever written.
struct Record {
    key: Key,
    parts: KeyParts,
    proof: Proof,
    took: Millis,        // what the proving run spent
}

struct Proof { scope: Scope, at: UnixSecs, commit: Option<Rev>, run: Option<RunRef> }
enum Scope { Integration, PullRequest(u64), Host }
struct RunRef { repo: String, run: u64, attempt: u32, leg: LegId }
```

`commit` is `None` for a lane's dirty tree; `run` is `None` on a lane's host. A record is a few hundred bytes.

## Which runs read and write

```rust
/// Read from the CI environment once, in the cli.
enum Trigger {
    Lane,
    PullRequest { number: u64, fork: bool },
    Push { branch: BranchName },
    Other,                                 // a schedule, a manual run, a tag
}

enum CacheMode { Off(Off), Use { read: Vec<Scope>, write: Option<Scope> } }
enum Off { NotConfigured, Flag, Fork, FullRun, NotAPullRequest }

fn mode(config: Option<&CacheConfig>, trigger: &Trigger, full: bool, no_cache: bool, integration: &BranchName) -> CacheMode
```

| Trigger | `full` | Mode |
|---|---|---|
| `Lane` | no | `Use { read: [Host], write: Host }` |
| `Lane` | yes (`--all`) | `Use { read: [], write: Host }` |
| `PullRequest { fork: false }` | no | `Use { read: [Integration, PullRequest(n)], write: PullRequest(n) }`; with `Read`: `write: None` |
| `PullRequest { fork: false }` | yes | `Use { read: [], write: PullRequest(n) }`; with `Read`: `Off(FullRun)` |
| `PullRequest { fork: true }` | any | `Off(Fork)` |
| `Push` to the integration branch | any | `Use { read: [], write: Integration }` |
| any other `Push`, `Other` | any | `Off(NotAPullRequest)` |

`mode` cannot return a pull request that writes `Integration`, nor an integration push that reads.

## One gate's lookup, and what the report says

```rust
enum Lookup {
    Off(Off),
    Unkeyed(Unkeyed),
    Miss(Key),
    Refused(Key, Refusal),      // the record is there and is not trusted: the gate runs
    Hit(Record),
    Recheck(Record),            // a hit this run was sampled to rerun
}
enum Refusal { KeyMismatch, PartsDiffer, WrongScope(Scope), Expired, Unreadable }

fn lookup(keyed: Keyed, mode: &CacheMode, found: Vec<(Scope, Record)>, now: UnixSecs, salt: &str) -> Lookup
```

- `KeyMismatch`: the file's name or `key` is not the sha256 of its own `parts`. `PartsDiffer`: its `parts` are
  not this run's. `WrongScope`: its `proof.scope` is not the tier it lies in.
- A hit is sampled for a recheck when `sha256(key, salt) mod recheck == 0`. `salt` is the run id and attempt in
  CI and the minute on a lane's host, so a test can fix it.

```rust
enum Outcome { Passed { slow: bool }, Cached(Hit), Failed(Failure), OverBudget, Skipped(Skip) }
struct Hit { proof: Proof, took: Millis }
enum Failure { Exit(i32), Timeout, MissingTool(String), CacheMismatch(Proof) }

struct GateReport { /* … */ cache: CacheNote }
enum CacheNote { None(Off), Unkeyed(Unkeyed), Miss, Refused(Refusal), Hit, Rechecked(Proof) }

struct CacheTotals { cached: u32, ran: Millis, saved: Millis, rechecked: u32, differed: u32, refused: u32 }
```

`Outcome::Cached` prints `cached` and does not fail. `ci done` sums `CacheTotals` over the legs into the summary
line and into the merged `report.json`.

## What `ci plan` tells the workflow

```json
{ "include": [ { "leg": "rust", "runs-on": "ubuntu-latest", "rust": true,
                 "results-restore": "gatekeeper-results-1-integration-rust-\ngatekeeper-results-1-pr-41-rust-",
                 "results-save": "gatekeeper-results-1-pr-41-rust-" } ] }
```

Empty strings mean no step. `gates.yml` restores each prefix into `$RUNNER_TEMP/gatekeeper-results/<tier>/`,
passes the folder as `--results`, and saves the tier named by `results-save` under that prefix plus the run id.
The workflow decides nothing: both strings come from `mode`.

## The store on disk

```
<store>/integration/<key>.json     what the last push to the integration branch proved
<store>/pr-41/<key>.json           what pull request 41's own runs proved
<store>/host/<key>.json            a lane's host
```

`<store>` is `~/.cache/gatekeeper/results` on a lane's host (beside `bin/` and `tools/`) and the restored folder
in CI. A write is a new file moved into place. A host folder drops records older than `max_age_days` as it reads.
