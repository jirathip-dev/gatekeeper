# Code ratchets: data shapes (#100)

## The two records

Both are #99's and neither is new. A check with named findings keeps a list; a check that measures keeps a
ceiling.

| Record | File | One entry | Falls by | Used by |
|---|---|---|---|---|
| list | `.gatekeeper/baseline/<pack>-<gate>.txt` | one finding line, sorted | `baseline <gate>` drops lines that are gone; a line still listed but gone fails the gate (`stale`) | `ts/knip`, `rust/dead-code`, `rust/complexity`, `ts/complexity` |
| ceiling | `ci/ratchet/<count>.d/<date>-<n>.toml` | `ceiling = <n>`; the lowest file wins | `baseline <gate>` adds a lower file; a count a whole step under the ceiling fails until it does | `core/duplication`, `core/surface` |

```toml
# ci/ratchet/duplication.d/2026-12-09-1240.toml
ceiling = 1240
```

The list's format is unchanged. The ceiling file is #99's `ci/tests/ratio.d` file with a whole number in
place of a ratio.

## Config: three optional tables in `.gatekeeper/config.toml`

```rust
// crates/gatekeeper-model/src/ratchet.rs

/// `[rust]` gains one key.
struct RustPack {
    manifest: String,
    unsafe_crates: Vec<String>,
    exported: Excuses,          // `[rust.exported]`: files whose `pub` items are used from outside the workspace
}

/// `[duplication]`: every key optional, the gate on without the table.
#[serde(default, deny_unknown_fields)]
struct DuplicationConfig {
    ignore: Excuses,            // `[duplication.ignore]`: more paths jscpd does not read, each with its reason
}

/// `[surface]`: what this repo counts. No table: nothing is counted and the gate says so.
#[serde(default, deny_unknown_fields)]
struct SurfaceConfig {
    names: Vec<SetName>,                    // each a set of the `[docs.names]` list: its names are one count
    lines: BTreeMap<SetName, Vec<Glob>>,    // `[surface.lines]`: each key one count of non-blank lines
}
```

`Excuses` (`glob = "reason"`), `SetName` and `Glob` are the docs gate's types. Refused when the config is
parsed: `names` without `[docs.names]`; one name in both `names` and `lines`; an empty glob list; an excuse that
matches no file (a finding, as in the docs gate).

No key sets a threshold, a minimum clone size or a step. Those are fixed:

```rust
const DUPLICATION_STEP: u32 = 10;       // equals jscpd's minLines: the headroom never holds a new clone
const SURFACE_STEP: u32 = 1;
const DUPLICATION_IGNORED: [&str; 9] = [
    "**/fixtures/**", "**/__fixtures__/**", "**/testdata/**", "**/__snapshots__/**", "**/golden/**",
    "**/vendor/**", "**/third_party/**", "**/*.d.ts", "**/*.min.*",
];
```

| File in the pack | Holds |
|---|---|
| `packs/core/files/jscpd.json` | `minTokens` 100, `minLines` 10, jscpd's default mode (`mild`), `gitignore` on, the json reporter |
| `packs/rust/files/clippy.toml` | `too-many-lines-threshold = 60` (there today), `cognitive-complexity-threshold = 25` (clippy's default, written down) |
| `packs/ts/files/complexity/oxlintrc.json` | `eslint/complexity` max 20, `eslint/max-lines-per-function` max 60, blank lines and comments skipped; no other rule |

## The counted unit of each check

| Gate | One unit | Its list line |
|---|---|---|
| `ts/knip` | one knip issue, as today | `src/a.ts: exports: old` |
| `rust/dead-code` (machete) | one dependency of one manifest that machete calls unused | `crates/core/Cargo.toml: unused dependency: anyhow` |
| | one name in a manifest's machete `ignored` list (`package.metadata` or `workspace.metadata`) | `crates/core/Cargo.toml: ignored for cargo-machete: prost` |
| `rust/dead-code` (the scan) | one `pub fn`, `const` or `static` that no other crate names | `crates/core/src/hosts.rs: pub fn held_slots: no other crate names it` |
| | one `pub` item that only test code names | `crates/sys/src/lib.rs: pub fn rusage: only tests name it` |
| | one `pub` type that nothing names | `crates/model/src/old.rs: pub struct Old: nothing names it` |
| `rust/complexity` | one function over one threshold | `crates/core/src/drive.rs: fn pass: over 60 lines` · `...: cognitive complexity over 25` |
| `ts/complexity` | the same | `web/src/board.ts: function render: complexity over 20` · `...: over 60 lines` |
| `core/duplication` | one duplicated line: a line of the second copy of a clone, as jscpd's `statistics.total.duplicatedLines` sums them | none: a number |
| `core/surface` | one name of a counted set; one non-blank line of a counted file | none: a number per count |

A line holds no line number and no measured value, so it stays the same while the finding stays. Two findings
with the same text in one gate get ` #2`, ` #3` in file order (an unnamed TS function is `function #2`), as
`swift/tokens` numbers its hits today.

## Reading each tool

```rust
// crates/gatekeeper-engine/src/ratchet/reports.rs: pure, a tool's output in, units out
fn machete(stdout: &str) -> Vec<String>;                 // `<package> -- <manifest>:` then one tab-indented name a line
fn clippy(json_lines: &str) -> Vec<String>;              // `reason = "compiler-message"`, `message.code.code` one of the two lints;
                                                         //   the path from `spans[0].file_name`, the name after `fn ` in `spans[0].text[0].text`
fn oxlint(json: &str) -> Vec<String>;                    // `diagnostics[]`: `filename`, `code`, the name between backticks in `message`
struct Duplication { duplicated_lines: u32, clones: Vec<Clone>, by_format: BTreeMap<String, u32> }
fn jscpd(report: &str) -> Result<Duplication>;           // `statistics.total.duplicatedLines`, `statistics.formats.*.total`, `duplicates[]`
```

| Tool | How it runs |
|---|---|
| knip | as today |
| cargo-machete | the pinned binary, `--with-metadata`, at the `[rust] manifest` folder; exit 1 with a list is findings, any other failure is an error |
| the `pub` scan | no tool: the files of the workspace members, read by the engine |
| clippy | `cargo clippy --workspace --all-targets --all-features --locked --message-format=json -- -A warnings --force-warn clippy::too_many_lines --force-warn clippy::cognitive_complexity`, with `CLIPPY_CONF_DIR` at the pack's files. `--force-warn` reports a function even under an `allow` |
| oxlint | the pinned binary over the `[ts]` projects with the pack's complexity config, `--format json` |
| jscpd | the pinned binary over the repo root with `--config` at the pack's settings (so a repo's own `.jscpd.json` is not read), `--format` the formats of the configured packs (`rust`; `typescript`, `tsx`, `javascript`, `jsx`; `swift`), `--ignore` the globs of `DUPLICATION_IGNORED`, the `generated` globs of `.drover/arch.toml` and `[duplication.ignore]`, `--output` the check's scratch folder; it reads `jscpd-report.json` there. Exit 0 is a finished scan, clones or not; anything else is an error. A `jscpd:ignore-start` comment in a read file is a finding no list takes |

clippy and oxlint were run for this ADR (`adr.md`, "Where it stands"). cargo-machete and jscpd were not: neither
is on this host and an architect lane installs nothing. Their rows follow machete 0.10.1's and jscpd 5.4.3's own
docs, and R1 and R2 each start by capturing real output as a pure case.

## The `pub` scan

```rust
// crates/gatekeeper-engine/src/ratchet/pubs.rs: pure
enum Kind { Value /* fn, const, static */, Type /* struct, enum, union, trait, type */ }
struct Item { path: String, kind: Kind, word: &'static str, name: String, target: Target }
/// A compilation target, by path: a member's `src/` less its bins is its lib; `src/main.rs`, each `src/bin/` entry,
/// `tests/`, `benches/`, `examples/` and `build.rs` are targets of their own.
struct Target { member: String, part: Part }
/// Where each identifier is written, split by target and by test or src code.
struct Mentions(BTreeMap<String, BTreeMap<Target, Seen>>);
struct Seen { src: u32, test: u32, impl_heads: u32 }
fn read(path: &str, text: &str, target: &Target) -> (Vec<Item>, Mentions);
fn findings(items: &[Item], mentions: &Mentions) -> Vec<String>;
```

| Question | Rule |
|---|---|
| Which items | a line outside test code that starts `pub`, then any of `async`, `unsafe`, `const`, `extern "..."`, then a kind word and a name. Not `pub(crate)` or `pub(super)`, not `pub mod`, not `pub use`, not fields or variants |
| What is exempt | every item of a crate whose manifest does not say `publish = false` (its `pub` items are its product; the gate's note names such a crate); a file matching `[rust.exported]`; an item under `#[no_mangle]` or `#[export_name]` |
| What a mention is | the item's name as a whole identifier in a `.rs` file of a workspace member. Comments are not read, strings are. The declaration itself is no mention, nor is a `pub use` line |
| Test code | #99's rule: a file's tail from its first `#[cfg(test)]` line, and files under `tests/` or `benches/` |
| Finding: no other crate names it | a value with no mention in any target but its own. Fix: `pub(crate)`; from then on the compiler's own dead-code lint, which is exact and already an error, covers it |
| Finding: only tests name it | any item whose mentions outside its declaration are all in test code |
| Finding: nothing names it | a type with no mention anywhere but its declaration and `impl` headers. A type is not held to "no other crate names it": another crate can use it through a field or a return value without writing its name |
| Binaries and libraries | the same rule for both. The compiler reports no dead code for a `pub` item it can reach from the crate root, in a binary as in a library (run on rustc 1.100, 2026-11-25). In a binary no other target can name the item, so every such `pub` value is a finding until it is `pub(crate)` |

The scan matches names, not paths. So it never calls a used item unused, except one used only through a macro
that builds the name or from outside the workspace (`[rust.exported]`), and it misses a dead item that shares
its name with a live one.

## The ceiling rules

```rust
// crates/gatekeeper-engine/src/ratchet/judge.rs: pure
struct Counted { name: CountName, n: u32, step: u32 }
/// `duplication`, or `surface-<set>`: the folder's name under `ci/ratchet/`.
struct CountName(String);
fn fits(n: u32, step: u32) -> u32;                               // n rounded up to a multiple of the step
fn ceiling(c: &Counted, lowest: Option<u32>) -> Vec<String>;     // the tree alone
```

| The tree | Finding |
|---|---|
| no file in the folder | `duplication: no ceiling: ./.gatekeeper/run baseline core/duplication` |
| `n > ceiling` | `duplication: 1262 duplicated lines, over the ceiling 1240` |
| `n + step <= ceiling` | `duplication: 1228 duplicated lines, a step under the ceiling 1240: ./.gatekeeper/run baseline core/duplication` |

Writer: `baseline <gate>` writes `fits(n, step)` when that is the first file or lower than the ceiling, and
nothing otherwise. A second run writes nothing.

## What a change may do: `core/ratchet`

```rust
// crates/gatekeeper-engine/src/ratchet/change.rs: reads git; needs a base, passes without one
struct Grown { gate: GateId, added: u32 }                         // lines of a pack gate's list that the base does not have
enum Touched { Edited(String), Deleted(String), NotLower { file: String, n: u32, base: u32 } }
// crates/gatekeeper-engine/src/ratchet/trailer.rs: pure
/// `Cap: <old> → <new> (#issue, q<N>)` and `Ratchet: <record> <old> → <new> (#issue, q<N>)`.
struct Trailer { word: &'static str, record: Option<String>, old: u32, new: u32, issue: u32, ask: u32 }
fn find(commit_bodies: &str, word: &str) -> Vec<Trailer>;
```

| The change | Finding, unless |
|---|---|
| adds a line to a pack gate's list | a commit of the change carries `Ratchet: <pack/gate> <old> → <new> (#issue, q<N>)`, or the change moves `[pin]` (a release may find more, and its bump writes the lists) |
| edits or deletes a file under `ci/ratchet/<count>.d/`, or adds one that is not under the base's ceiling | a commit carries `Ratchet: <count> <old> → <new> (#issue, q<N>)` |

A local gate's list is the repo's own and is not held. #99's `ci/tests/ratio.d` keeps its own rule and has no
trailer. `core/cap` reads its `Cap:` trailer through the same `trailer.rs`.

## The gate files

| Gate | Steps | Budget | Notes |
|---|---|---|---|
| `rust/dead-code` | `check = "machete"`, `check = "unused-pub"` | 60 s | one list for both |
| `rust/complexity` | `check = "clippy-complexity"` | 180 s | one more clippy pass over the workspace's own crates |
| `ts/complexity` | `check = "oxlint-complexity"` | 30 s | |
| `core/duplication` | `check = "duplication"` | 60 s | no setup: jscpd 5 is one binary |
| `core/surface` | `check = "surface"` | 10 s | `cache = false`: it may run the names command |
| `core/ratchet` | `check = "ratchet"` | 5 s | `cache = false`: it reads the base and the commits |

## The pinned tools

| File | Tool | Status |
|---|---|---|
| `packs/ts/tools/knip.toml` | knip | there today |
| `packs/rust/tools/cargo-machete.toml` | cargo-machete 0.10.1, its release archive per target with its sha256 | approved |
| `packs/core/tools/jscpd.toml` | jscpd 5.4.3, its binary package per target (`jscpd-linux-x64-gnu`, `jscpd-darwin-arm64`) with its sha256 | approved |
| none | clippy is the repo's toolchain; oxlint is pinned today | no new dependency |
