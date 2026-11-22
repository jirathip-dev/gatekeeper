# Hotspots, week of 2026-11-16: data shapes (#67)

## A topic

```
crates/gatekeeper-engine/src/skill/topics/<topic>.md
```

- `<topic>`: lowercase letters, digits and dashes. The name is the sort key and appears nowhere in the output.
- The text: markdown, UTF-8. Copied as it is, trailing blank lines trimmed. No heading is required, nothing
  is substituted.

The build script lists the folder, as it lists `packs/` today. Nothing is checked in:

```rust
// OUT_DIR/topics.rs, written by crates/gatekeeper-engine/build.rs, sorted by file name
pub static TOPICS: &[(&str, &str)] = &[
    ("cache", include_str!("…/src/skill/topics/cache.md")),
    ("select", include_str!("…/src/skill/topics/select.md")),
];
```

The folder is inside the engine's `src/`, so the build script already reruns on a change there and already
counts the files in `GATEKEEPER_SOURCE`.

## `SKILL.md`, assembled

```
the opening            skill.rs: the front matter, the title with the version, the three steps, `list` and local gates
                       (blank line)
each topic             skill/topics/*.md, by file name, a blank line after each
## Packs               skill.rs: one line a built-in pack, from its pack.toml
## Pinned tools        skill/tools.rs: the head and one line a tool file
```

`SkillFile { path, text }`, `skill::files()` and `skill::write(to)` keep their shape and their callers.

## A run's context

```rust
// crates/gatekeeper-engine/src/run/cx.rs
/// Where and how a plan runs; `out` takes the logs and `report.json`.
#[non_exhaustive]
pub struct RunCtx { /* the sixteen fields of today, all still `pub` */ }

impl RunCtx {
    /// A lane's plain run of the repo at `root`: no base, no result cache, this host's platform.
    #[must_use]
    pub fn lane(root: &Path, cache: &Path) -> RunCtx;
}
```

| Field | What `lane` gives it |
|---|---|
| `root`, `cache` | the arguments |
| `out` | `out_dir(root)` |
| `host` | `Host::Lane` |
| `platform` | `platform()` |
| `manifest` | `Cargo.toml` |
| `base`, `pr_body`, `job_start`, `after_reports`, `ts`, `swift`, `results`, `timings` | `None` |
| `tools`, `env` | empty |

`#[non_exhaustive]` is the point: another crate (the cli, every integration test) can read and assign a field
but cannot write the struct out, so a new field never breaks a caller that does not use it. `CheckCx` keeps its
shape and moves to the same file.

## A gate's report

```rust
// crates/gatekeeper-model/src/report.rs
#[non_exhaustive]
pub struct GateReport { /* the eleven fields of today */ }

impl GateReport {
    /// A gate that did not run, or has not yet: its outcome and budget, no time, no log, nothing kept.
    #[must_use]
    pub fn unrun(gate: &Gate, outcome: Outcome) -> GateReport;
}

impl From<Duration> for Millis { /* saturating, as `run::millis` is today */ }
```

`GateReport::unrun` is the one place every field is named. `run::unrun` and `run::millis` are deleted; their
callers in `run`, `cache` and `select` move to these. The JSON is unchanged: same fields, same defaults.

## A run line's places

```rust
// crates/gatekeeper-engine/src/run/line.rs
/// A run line's `{name}` places and what fills each, applied in order.
pub(crate) type Vars = Vec<(&'static str, String)>;
```

| Place | Owner | Filled with |
|---|---|---|
| `{files}` on a `[select]` gate | `select` (`Picked::vars`) | `"$@"`: the files are the shell's arguments |
| `{report}` | `select` (`Picked::vars`) | where the line writes vitest's JSON report |
| `{shard}`, `{projects}` | `run/gate.rs` | this job's shard, the projects this run covers |
| `{--base}`, `{pr-body}`; a tool step's `{--base}`, `{--pr-body}` | `run/line.rs` | from the context |
| `{files}`, `{scripts}` | `run/line.rs` | the pack's two folders, written under `out` |
| `{tool:<name>}` | `run/line.rs` | the fetched tool's path |

A `[select]` gate's `{files}` is filled before the pack's, as today. The run always hands such a gate its
`Picked`; `selftest` runs pack gates only, and no pack gate selects.

## `.drover/arch.toml`

The exact lines are at the end of `adr.md`: `skill.rs` frozen, the topics folder collected, `run.rs` shared
and held to 150 lines.
