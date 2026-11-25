# ADR: code ratchets: dead code, duplication, complexity, surface (#100)

## Summary

Decision: four ratchets from six new gates and the knip gate, which stays as it is. Dead code and complexity keep a list of named findings that only shrinks, as knip does today. Duplication and surface keep one number each, in the ceiling folders of the test health design (#99). A list or a number goes up only in a commit that carries the owner's decision. No new tool: the pinned oxlint has both complexity rules.
Slices: gatekeeper: a cap raise, dead code (with the gate that guards every list), duplication, complexity, surface, its own adoption and release 1.7.0. Then one adoption per repo, drover and shed first.
Open questions: (1) Dead code and complexity as named lists, not bare counts: swapping one dead item for a new one then fails. Agree? (2) May a surface count ever rise? Recommended: only with your decision in the commit, as a cap raise. (3) Approve one cap raise for gatekeeper, 11,700 to 12,700?

Status: proposed 2026-11-25 (architect lane). Read on gatekeeper staging 06e8d38 (1.5.0), drover staging
4cf52e4bf, shed faee4f3b. Read `README.md` first.

## Question

How does gatekeeper count dead code, duplication, over-long and over-complex functions and public surface in
every repo, so that each count can only fall, with the same result on a lane and in CI, with no file every PR
edits, and with no baseline mechanism beyond the two #99 settles?

## Where it stands

- `baseline.rs` keeps a gate's known findings in `.gatekeeper/baseline/<gate>.txt`. A new finding fails, a
  listed one that is gone fails until `baseline <gate>` drops it. `ts/knip` runs on it. No rule stops a PR
  from adding a line to the file by hand.
- #99 (read at `origin/issue-99` 161323a, with its amendment of 2026-11-25; not on staging yet) adds the ceiling
  folder: one file per lowering in `ci/tests/ratio.d/`, the lowest wins, `baseline core/test-health` writes it,
  a value a whole step under the ceiling fails until it is lowered, an edited or deleted file fails.
- `rust/clippy` runs with `-D warnings` and the pack's `too-many-lines-threshold = 60`, so a repo on the rust
  pack has no function over 60 lines except under an `allow`. drover allows `too_many_lines` for its whole
  workspace. `cognitive_complexity` is a clippy nursery lint and is off everywhere.
- Run on clippy 0.1.100: `--force-warn clippy::too_many_lines` reports a function that carries
  `#[allow(clippy::too_many_lines)]`, in the JSON messages, with the `fn` line in the span.
- Run on the pinned oxlint 1.94.0: `eslint/complexity` and `eslint/max-lines-per-function` both exist and
  report in `--format json`. A named function's name is in the message; an arrow function has none.
- Run on rustc 1.100: the compiler reports no dead code for a `pub` item it can reach from the crate root, in
  a binary as in a library. It is exact for everything else, and that lint is already an error.
- jscpd's current release, 5.4.3, is one self-contained binary per platform with the flags, config keys and
  JSON report of the Node versions before it. It skips what git ignores unless told not to. cargo-machete
  0.10.1 ships a binary for both of gatekeeper's targets. Neither was run here: the lane installs nothing.
- The docs gate reads one names list per repo (`<set> TAB <name>`, from a command or a generated file) and
  holds the docs to it.
- A prototype of the `pub` scan in `shapes.md` finds 315 items in drover (313 of them `pub` values that only
  their own crate uses), 33 in shed, 12 in gatekeeper.
- gatekeeper counts 9,753 lines against a cap of 10,500; #99 raises it to 11,700 and adds about 1,650.

## What this ADR assumes of #99

The ceiling folder's file and rules, `baseline <gate>` as its writer, the date as a typed value, the rule for
which Rust lines are test code, and the list left as it is. If #99's folder changes before it lands,
duplication and surface follow it. Dead code and complexity need only the test-code rule.

## Options

### 1. How a count is recorded

| Option | Verdict |
|---|---|
| A. A ceiling number for all four | No for dead code and complexity. A PR that deletes one dead item and adds another passes. The failure cannot name the new item. Two open PRs that both clean up break the second one at merge. knip's lists in the repos that have them would turn into a weaker check. |
| B. A list for all four | No for duplication and surface. A clone has no stable name (its line ranges move with every edit above it). A surface list could never take a new flag, even when an old one leaves. |
| C. A list where a finding has a name, a ceiling where the check measures | **Yes.** Both are #99's. Dead code and complexity get the stricter and more exact record; duplication and surface get the only one that fits them. |
| D. No record: compare with the base | No. It needs the tool run on the base tree too, and a push has no base. |
| E. One file per PR holding the amount it lowered | No. It composes across PRs, but it is the second mechanism the owner ruled out. |

### 2. Unused Rust `pub` items

| Option | Verdict |
|---|---|
| A. A parser crate or an outside tool (rust-analyzer based) | No. A new dependency, and the owner said gatekeeper implements it. |
| B. Call a `pub` item dead when its name appears nowhere else | No. Tried on drover: 2 findings in 1,839 items. Common names hide almost everything. |
| C. Find each `pub` value no other crate names and have it made `pub(crate)`; the compiler then does the reachability | **Yes.** The scan answers the one question a name scan can answer, and the exact answer comes from the lint that is already an error. Plus two small rules: only tests name it, and a type nothing names. |

### 3. Counting functions over a threshold in Rust

| Option | Verdict |
|---|---|
| A. Read the `rust/clippy` gate's output | No. It fails on the first warning, and an `allow` hides a function from it. |
| B. Count the `allow` attributes by text | No. A workspace-wide allow, as drover's, has no attribute per function. |
| C. One more clippy pass with the two lints forced on | **Yes.** `--force-warn` reports through every `allow`. Costs one more check of the workspace's own crates. |

### 4. Complexity in TypeScript

| Option | Verdict |
|---|---|
| A. ESLint | No. A new dependency, not needed. |
| B. The pinned oxlint with a second config that holds only the two rules | **Yes.** Proved by a run. A second config keeps `ts/oxlint`'s findings and list as they are. |

### 5. What duplication counts

| Option | Verdict |
|---|---|
| A. Clones | No. A clone that grows, or two that merge into one longer one, does not move the count. |
| B. A percentage | No. It falls when unrelated code is added. |
| C. Duplicated lines, one number per repo over all its languages | **Yes.** It moves with the amount. The gate's note prints the clones and the lines per language. |
| D. Both, or one ceiling per language | No. More folders for no case a single number misses. |
| E. jscpd 5's own baseline: fail on a clone that is new against a git ref | No as the record: it stores no count, has nothing to check on a push, and is a second mechanism. R2 may use it only to name the new clones in a failing run's log, if a real run shows it holds; the verdict never depends on it. |

### 6. Where surface names come from

| Option | Verdict |
|---|---|
| A. gatekeeper parses clap, serde and error enums | No. One parser per language and framework. |
| B. A second names file for surface | No. Two lists of one repo's names. |
| C. The docs gate's names list: `[surface]` says which of its sets are counted, and which files' lines | **Yes.** The repo already prints its real names for the docs gate, and that gate fails a doc that uses a name the list lacks, which keeps the list honest. |

### 7. Where the rules that need a base live

| Option | Verdict |
|---|---|
| A. In each check and in `run/gate.rs` | No. A cached gate result would skip them, and six checks would each read git. |
| B. One gate, `core/ratchet`, never cached | **Yes.** The measuring checks read the tree alone. One place says what a change may do to a list or a ceiling. |

### 8. How a ratchet goes up

| Option | Verdict |
|---|---|
| A. Never | No. A tool bump can find more, and a feature can need a flag. With no way up the gate gets switched off instead. |
| B. A commit trailer with the owner's decision, as `Cap:` | **Yes**, pending open question 2. `Ratchet: <record> <old> → <new> (#issue, q<N>)`. A release bump may also add list lines, since it is the bump that brings the new checks. |

## Decision

The owner's four decisions, each with what this ADR settles.

1. **Four ratchets** (owner 1, option 1C).

   | Ratchet | Gates | Record |
   |---|---|---|
   | dead code | `ts/knip` (as today), `rust/dead-code` | list |
   | duplication | `core/duplication` | ceiling `ci/ratchet/duplication.d/` |
   | complexity | `rust/complexity`, `ts/complexity` | list |
   | surface | `core/surface` | one ceiling per count, `ci/ratchet/surface-<count>.d/` |

   Every run prints the count: a list's length, or the number beside its ceiling.
2. **One mechanism** (owner 2). Nothing new stores a baseline. The list is today's. The ceiling folder is
   #99's, moved from `health/` to a shared `ratchet/folder.rs` with a step as its one parameter; the ratio
   becomes a caller.
3. **`core/ratchet`** (options 7B, 8B) is what makes a list a ratchet: a change that adds a line to a pack
   gate's list fails, and so does one that edits or deletes a ceiling file or adds one that is not lower. The
   two ways past it are the `Ratchet:` trailer and, for lists only, a moved `[pin]`.
4. **Dead code, TypeScript.** `ts/knip`, its config and its list do not change. Its list is now held by
   `core/ratchet` like every other.
5. **Dead code, Rust** (owner 3, option 2C). `rust/dead-code` has two steps and one list.
   cargo-machete runs pinned with `--with-metadata`; each unused dependency is a finding, and so is each name
   in a manifest's machete `ignored` list, so that list is on the ratchet too. The scan reads every workspace
   member: a `pub fn`, `const` or `static` that no other crate names is a finding whose fix is `pub(crate)`;
   an item only tests name is a finding; a type nothing names is a finding. Binaries and libraries follow one
   rule. A crate that is published, and a file under `[rust.exported]`, are exempt.
6. **Duplication** (owner 3, option 5C). jscpd 5 runs pinned over the repo with the pack's settings, never
   the repo's own jscpd config: a clone is at least 100 tokens and 10 lines. It reads the languages of the
   configured packs and skips what git ignores, fixture, snapshot, golden and vendored folders, the arch
   file's generated files, and `[duplication.ignore]` with a reason each. Tests are read. The count is jscpd's
   duplicated lines. The step is 10 lines, the smallest clone, so the headroom under a ceiling never fits a
   new clone.
7. **Complexity** (owner 3, options 3C, 4B). Rust: over 60 lines, or cognitive complexity over 25, in
   `packs/rust/files/clippy.toml`. TypeScript: over 60 lines, or complexity over 20, in the pack's oxlint
   complexity config. No repo key changes a threshold. A finding is one function over one threshold, named by
   file and function with no number in it.
8. **Surface** (owner 3, option 6C). `[surface] names` lists sets of the docs gate's names list; each set's
   number of names is a count. `[surface.lines]` names globs; their non-blank lines are a count. So a repo
   declares its flags, config keys and refusal codes by printing them in its names list, and its agent-doc
   lines by a glob. The step is 1. Reused from the docs gate: the list, its reader, the set name type and the
   excuse type. Not reused: prefixes and doc classes.
9. **Languages.** Rust and TypeScript get all four. Swift gets duplication and surface: no approved tool
   finds dead Swift code or measures a Swift function.
10. **Rollout** (owner 4). The gates are on for a repo from its bump to 1.7.0. The bump PR writes each list
    and each first ceiling with `baseline <gate>`, so every ratchet starts at the repo's count of that day.

Who writes what:

| File | Writer | Moves |
|---|---|---|
| `.gatekeeper/baseline/<gate>.txt` | `baseline <gate>` | shrinks; grows only by hand under a `Ratchet:` trailer or in a release bump |
| `ci/ratchet/<count>.d/*.toml` | `baseline <gate>`, in any PR | down; up only when a commit with the trailer replaces the folder's files |
| `packs/*/files/*` (thresholds, jscpd settings) | a gatekeeper release | with the release |

## Consequences

- A list is stricter than a count: deleting one dead item does not pay for a new one. A file that is renamed
  takes its listed findings along as new ones, so the rename fixes them. knip behaves this way today.
- drover starts with about 315 dead-code entries. Nearly all are live code that is `pub` without need; each
  fix is one word, and only then does the compiler show which were dead. The scan misses a dead item that
  shares its name with a live one, and holds types only to "nothing names it".
- An `allow` no longer hides a long function. drover's complexity list will be long at its adoption (not
  measured). `cognitive_complexity` is a nursery lint: a toolchain bump that changes its result must fix the
  function or carry the trailer.
- Rust repos run clippy over their own crates once more per run.
- jscpd is pinned at 5, the tool's current major: one binary, no node, a rewritten engine. A count from an
  older jscpd need not match it.
- Two open PRs that both lower one ceiling: the second to merge is a whole step under after the first lands
  and needs one more `baseline` run (drover's update round). Lists have no such case.
- A PR that adds a flag, a config key or a refusal code and removes none needs the owner (open question 2).
- Surface counts what the repo's names list prints. A repo that prints no set counts nothing, and the gate's
  note says so; the adoption issue is where a repo declares its sets.
- From the bump on, a hand-added line fails in every pack gate's list: docs, tokens and knip too.
- gatekeeper gains about 900 lines (open question 3).

## Slices

Gatekeeper. Each slice adds files to collected folders; `baseline.rs`, `run/gate.rs` and the model's `Config`
get one line each.

| # | Issue | Blocked on | AC |
|---|---|---|---|
| R0 | cap: raise gatekeeper to 12,700 (filed by the orchestrator) | open question 3, #99 G0 | the raise file with its reason; nothing else |
| R1 | dead code: `ratchet/` with `reports`, `pubs` and `trailer`; `rust/dead-code` with cargo-machete pinned and `[rust.exported]`; `core/ratchet` with the list rule; the `<n> listed` note; this ADR's `arch.toml` lines | R0, #99 G1 | the dead-code and ratchet fixture cases of `boundaries.md`; the pure cases read real machete output; `core/cap`'s cases pass with no `expect` file edited; `ts/knip`'s cases untouched |
| R2 | duplication: `folder` and `judge`, moved out of #99's `health/ratchet.rs`; `core/duplication` with jscpd 5 pinned and the pack's settings; `[duplication.ignore]`; the folder rule in `core/ratchet`; the `baseline` arm | R1 | the duplication cases; #99's ratio cases pass unedited; a second `baseline core/duplication` writes nothing; the same count on Linux and macOS |
| R3 | complexity: `rust/complexity`, `ts/complexity`, the thresholds in the pack files | R1 | the complexity cases, with a function under an `allow` counted and a second unnamed TS function told apart |
| R4 | surface: `core/surface`, `[surface]`, the names reader that returns every set | R2 | the surface cases; a raise passes only with its trailer; no `[surface]` passes with its note |
| R5 | gatekeeper adopts it: its own lists and ceilings, `[surface]` over its verbs and skill lines, the skill topic, release 1.7.0 | R1 to R4, #99 G7 | its own PR shows the six gates green on Linux and macOS; `git grep` finds the new keys in the skill |

Adoption, one issue per repo after R5. drover and shed first.

| # | Issue | Blocked on | AC |
|---|---|---|---|
| A1 | drover adopts the code ratchets | R5, drover#1587, #99 A1 | pin 1.7.0; the lists of `rust/dead-code`, `rust/complexity` and its TS gates and the first ceilings, each by `baseline`; its names list prints the `flag`, `config-key` and `refusal` sets and `[surface]` counts them and its agent docs; the `arch.toml` line; the PR body gives each starting count |
| A2 | shed adopts the code ratchets | R5, #99 A2 | the same; a crate another repo builds against goes under `[rust.exported]` with its reason |
| A3… | each other fleet repo, one each | R5 | the same for the packs it has; `[surface]` once it has a names list |

## Open questions

1. **Lists for dead code and complexity.** The issue speaks of counts. A list's length is that count, and it
   also fails a swap and names the new item. A bare ceiling instead would be option 1A.
2. **Raising a surface count.** Recommended: yes, only in a commit with `Ratchet: surface-<count> <old> → <new>
   (#issue, q<N>)` after your answer, as a cap raise works. The other way: never, and a new flag must replace
   an old one.
3. **The cap.** 11,700 after #99's raise, about 11,400 used once #99 lands, about 900 arrive: approve one
   raise to 12,700, as R0?

## `.drover/arch.toml` lines

Gatekeeper, in R1:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/src/ratchet"
glob = "*.rs"

[[collect]]
dir = "crates/gatekeeper-engine/tests/ratchet"
glob = "*/case.toml"

[[ban]]
pattern = "runner::"
paths = ["crates/gatekeeper-engine/src/ratchet/**"]
except = ["crates/gatekeeper-engine/src/ratchet/change.rs"]
reason = "the ratchet rules are pure: a tool's output in, findings out; one module reads git"

[[ban]]
pattern = "fs::"
paths = ["crates/gatekeeper-engine/src/ratchet/**"]
except = ["crates/gatekeeper-engine/src/ratchet/folder.rs", "crates/gatekeeper-engine/src/ratchet/change.rs"]
reason = "the ratchet rules are pure: a tool's output in, findings out; two modules read files"
```

The six check files, the gate files and the fixture cases need no line: their folders are collected and
`check/mod.rs` is generated by `sh scripts/gen.sh`. No file joins `frozen`.

Each adopting repo, in its adoption PR:

```toml
[[collect]]
dir = "ci/ratchet"
glob = "*.d/*.toml"
```

A ceiling is one file per lowering, so no PR edits another's. A list is rewritten by `baseline <gate>` and
never merged by hand.
