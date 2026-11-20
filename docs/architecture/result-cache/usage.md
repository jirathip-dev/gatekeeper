# The result cache: how the callers read (#17)

Read this first. These are the call sites once the slices land; `adr.md` says why, `shapes.md` has the types.
Every name here is new. Shas and run numbers are made up.

## A consumer's config

```toml
# .gatekeeper/config.toml
packs = ["core", "rust"]
integration = "staging"

[cache]            # the table turns the cache on; without it nothing is ever skipped
recheck = 1        # every hit is rerun: the measuring mode a consumer starts in (default 10: one hit in ten)
```

The other two keys, with their defaults: `max_age_days = 14` and `pull_requests = "own"` (`"read"`: a pull request
never writes; see the open question). Nothing is configured for where records live: a lane's host uses the folder
the wrapper already uses, CI uses GitHub's cache through `gates.yml`. The consumer's `ci.yml` does not change.

## A gate says what it reads

```toml
# .gatekeeper/gates/unit.toml
about = "unit tests of the web app"
fix = "fix the failing test the log names"
budget_secs = 120
run = "pnpm vitest run"
paths = ["web/**"]                 # what it reads: the path scope's list and the cache's, one declaration
versions = ["node --version", "pnpm --version"]   # each command's output is part of the key
```

- `paths` is the whole declaration of the files a gate reads. Without it the gate's input is every tracked file,
  so it is cached only while nothing at all changed.
- A pack adds files every one of its gates reads, once, in `pack.toml`: `always = ["**/Cargo.toml", "Cargo.lock",
  "rust-toolchain*", "clippy.toml", "rustfmt.toml"]`. Path scope uses the same list, so a lockfile change both
  runs the gate and misses the cache. `.gatekeeper/**` is always an input of every gate.
- A pack gate gets its toolchain's version from `setup` (`rust`: `rustc -Vv`). A gate with a `run` step and
  neither `setup` nor `versions` is never cached, and says so.
- `cache = false` is for a gate that reads the network, the clock or a secret: `rust/deny` (the advisory list),
  `core/hygiene`. A gate that takes the base or the PR body (`core/arch`, `core/pr-body`) and a gate that keeps
  files (`[[upload]]`) are never cached, with no key to set.

```
$ ./.gatekeeper/run list
core/cap          5s   key 4c1e9a02…   412 files
core/arch        45s   not cached: reads the base
rust/clippy      90s   key 9b07d3aa…    38 files · rustc 1.91.0
rust/deny        20s   not cached: cache = false
local/unit      120s   key e2f4410c…   211 files · node v22.11.0 · pnpm 9.14.2
local/ui        120s   not cached: keeps files
```

## A lane's run

```
$ ./.gatekeeper/run check
gatekeeper 1.3.0 · packs core rust · base 3f9c2ab8e1d0 · 3 files changed · host lane · cache host folder
cached core/cap           (0.4s) /    5s   lane run 11-20 14:02 at 7a1b3c9
ok     core/arch            2.1s /   45s
ok     core/actionlint      0.6s /    5s
ok     rust/fmt             0.8s /   10s
FAIL   rust/clippy          6.2s /   90s
       src/pack.rs:41:17: used `unwrap()` on an `Option` value (clippy::unwrap_used)
       fix: return the error; only tests may unwrap
       log: target/gatekeeper/rust-clippy.log   rerun: ./.gatekeeper/run check --gate rust/clippy
ok     rust/deny            1.7s /   20s
cached local/unit        (41.0s) /  120s   lane run 11-20 14:02 at 7a1b3c9
ok     local/build         38.2s /  120s   rechecked: cached by lane run 11-20 13:40 at 51d0e77
1 failed: rust/clippy · 2 cached, saved 41.4s · 1 rechecked · setup 0.3s
```

- `cached` names the run that proved it, and shows that run's time in brackets. It counts as passed.
- A failure is never remembered: `rust/clippy` runs again every time until it passes.
- `drover turn done` runs the same command at the same head a minute later: every gate that passed is `cached`.
- `--no-cache` runs everything and writes nothing; `--all` runs everything and writes what passed.

When a rerun of a cached gate fails, the gate fails, loudly, and the record is deleted:

```
FAIL   local/unit          39.8s /  120s   cached pass does not hold
       it passed in lane run 11-20 14:02 at 7a1b3c9 with the same key e2f4410c… and fails now:
       the gate reads something it does not declare. Add it to `paths`, or set `cache = false`.
       log: target/gatekeeper/local-unit.log
```

## CI

Nothing changes in the consumer's workflow. `gates.yml` restores the records before the gates and saves them
after, as it does the pinned tools today. Which runs read and write:

| Run | Reads | Writes |
|---|---|---|
| a pull request from the repo | the integration branch's records, then its own | its own, which no other run can read |
| a pull request from a fork | nothing | nothing |
| a push to the integration branch | nothing: every gate runs | the integration branch's records |
| `scope: full`, `--all` | nothing: every gate runs | what the same run would write without it |
| a schedule, any other push, `--no-cache` | nothing: every gate runs | nothing |
| a lane's host | the host folder | the host folder |

The summary `gates / done` writes:

```
| job   | host          | setup | gate         | took    | budget |        | proved by                     |
| core  | github-hosted |  21 s | core/cap     | (0.4 s) |    5 s | cached | run 9113 · staging · 3f9c2ab  |
| rust  | github-hosted |  38 s | rust/clippy  | 12.0 s  |   90 s | ok     |                               |
| local | github-hosted |  40 s | local/unit   | (41 s)  |  120 s | cached | run 9120 · this PR · 88e01d4  |
| local | github-hosted |  40 s | local/e2e    | 95.0 s  |  300 s | ok     | rechecked: run 9120           |

cache: 2 of 4 gates cached · ran 107.0 s · saved 41.4 s (28 %) · 1 rechecked, 0 differed · 0 records refused
```

`proved by` links to the run. A rerun that differs fails `gates / done` and names both runs.

## Measuring it against a framework's cache

A consumer that caches with a build framework today keeps it, adds `[cache]` with `recheck = 1`, and lets both
run on the same pull requests. With `recheck = 1` gatekeeper skips nothing: each hit is rerun and printed as
`rechecked`, so the summary's `saved` is what it would have saved and `differed` is how often it would have been
wrong. After 50 pull-request runs the adoption issue compares three numbers:

| Number | From | It matches when |
|---|---|---|
| reruns that differed | the summary's `differed`, summed over the runs | 0 |
| share of gate time saved | the summary's `saved / (ran + saved)`, summed; the framework's own cached-task time on the same runs | gatekeeper's share is at least the framework's |
| the gates job's wall time | a trial PR with `recheck = 10` and the framework's cache off, five pushes; the median of the 50 runs | the trial's median is at most the 50 runs' |

Only then does the repo set `recheck = 10` and drop the framework, in its own PR.
