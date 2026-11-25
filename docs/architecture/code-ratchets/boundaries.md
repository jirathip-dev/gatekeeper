# Code ratchets: module boundaries and statecharts (#100)

## Where the code goes

```
crates/gatekeeper-model/src/
  ratchet.rs            DuplicationConfig, SurfaceConfig, CountName, Counted, the fixed steps and ignores
  lib.rs                Config gains `duplication` and `surface`; RustPack gains `exported` (one line each)

crates/gatekeeper-engine/src/
  ratchet/              new, a collected folder
    mod.rs              the folder's modules, nothing else
    reports.rs    pure  a tool's output -> units: machete, clippy, oxlint, jscpd
    pubs.rs       pure  files' text -> `pub` items and mentions -> findings
    judge.rs      pure  a count, its step, the lowest file -> findings; `fits`
    trailer.rs    pure  `Cap:` and `Ratchet:` lines of commit bodies (cap.rs's parser moves here)
    folder.rs           a ceiling folder in the tree: read, the lowest, write (from #99's health/ratchet.rs)
    change.rs           against the base: a list that grew, a ceiling file touched; the one module that reads git
  check/
    machete.rs          runs cargo-machete, reads each manifest's `ignored`
    unused_pub.rs       reads the members' files, calls pubs
    clippy_complexity.rs
    oxlint_complexity.rs
    duplication.rs      runs jscpd; measure, then judge
    surface.rs          names through docs::names, lines through globs; measure, then judge
    ratchet.rs          core/ratchet: change.rs's findings less the trailers and a moved pin
    knip.rs             unchanged
    cap.rs              its trailer parser is ratchet::trailer; nothing else changes
  baseline.rs           one more arm beside arch's and #99's: a ceiling check's `measure` -> folder::write
  run/gate.rs           the note gains `<n> listed` when the gate's list is not empty; nothing else
  health/ratchet.rs     (#99) keeps the ratio's rounding and calls ratchet::folder
  docs/names.rs         gains one reader that returns every set of the list, for surface
```

## Who may call whom

```
 cli ──▶ engine::baseline ──▶ check::{duplication, surface}::measure
                          └─▶ ratchet::folder (write)
 run/gate.rs ──▶ baseline (unlisted, stale): as today, for every list
 check/{machete, clippy_complexity, oxlint_complexity}.rs ──▶ runner (the tool) · ratchet::reports
 check/unused_pub.rs ──▶ graph::cargo · health::count (the test tail, #99) · ratchet::pubs
 check/duplication.rs ──▶ runner (jscpd) · arch::config::generated · ratchet::{reports, judge, folder}
 check/surface.rs ──▶ docs::names · ratchet::{judge, folder}
 check/ratchet.rs ──▶ ratchet::{change, trailer}
 check/cap.rs ──▶ ratchet::trailer
 health/ratchet.rs ──▶ ratchet::folder
```

- `reports`, `pubs`, `judge` and `trailer` are pure: no `fs::`, no `runner::`. An arch ban holds it.
- Only `change.rs` reads git. Only `folder.rs` writes under `ci/ratchet/`, and only from `baseline`.
- A measuring check reads the tree alone, so it gives a lane, a PR and a push the same answer. Everything that
  needs a base is in `core/ratchet`.
- Nothing in `ratchet/` calls `check/`, `select/`, `cache/` or `docs/`. The checks call it.
- Nothing reads the GitHub API or the environment.
- The model crate holds the shapes and no rule.

## Statechart: a listed finding (dead code, complexity)

```
                 adoption: baseline <gate>
  [in the code] ───────────────────────────▶ [listed]   it passes; the note counts it
        ▲                                        │
        │ a change adds a new one:               │ the code is fixed or deleted
        │ the gate fails, nothing is written     ▼
  [new] ┘                                    [stale]    the gate fails until baseline <gate> drops the line
                                                 │
                                                 ▼
                                             [gone]     final
```

No arrow goes from `[new]` to `[listed]` except a commit with the `Ratchet:` trailer or a release bump:
`core/ratchet` fails any other change that adds a line.

## Statechart: a ceiling (duplication, each surface count)

```
              baseline <gate>
  [none] ──────────────────────▶ [at c]     c = the count rounded up to the step
   every run fails: "no ceiling"   │  ▲
                                   │  │ count > c ──▶ the gate fails; nothing is written
       count + step <= c: the gate │  │
       fails until baseline adds   ▼  │
       a lower file            [at c' < c] ──▶ ...
                                   │
                                   │ a commit with `Ratchet: <count> c → r`: the folder's files replaced by one at r
                                   ▼
                               [at r > c]
```

Without the trailer no arrow raises it: `core/ratchet` fails an edited or deleted file and a new file that is
not lower.

## How it is tested

| What | Where | How |
|---|---|---|
| `rust/dead-code` | `packs/rust/fixtures/dead-code/<case>/` | selftest with the real pinned machete: `unused-dependency`, `ignored-dependency`, `pub-no-other-crate`, `pub-only-tests`, `pub-type-unnamed`, `pub-in-a-binary`; passing: `exported-file`, `published-crate`, `named-by-another-crate`, `type-used-through-a-field` |
| `rust/complexity` | `packs/rust/fixtures/complexity/<case>/` | selftest with real clippy: `too-many-lines`, `under-an-allow-still-counts`, `cognitive` |
| `ts/complexity` | `packs/ts/fixtures/complexity/<case>/` | selftest with the pinned oxlint: `complexity`, `max-lines`, `second-unnamed-function` |
| `core/duplication` | `packs/core/fixtures/duplication/<case>/` | selftest with the pinned jscpd: `over-ceiling`, `no-ceiling`, `a-step-under`, `ignore-comment`; passing: `under-the-minimum-size`, `ignored-with-a-reason`, `generated-file`, `fixture-folder` |
| `core/surface` | `packs/core/fixtures/surface/<case>/` | `name-added`, `line-added`, `removed-and-not-lowered`, `set-not-in-the-list`; passing: `one-added-one-removed`, `no-table` |
| `core/ratchet` | `packs/core/fixtures/ratchet/<case>/` | a case's commit is the change and its `message` the commit body: `list-grew`, `ceiling-edited`, `ceiling-deleted`, `new-file-not-lower`, `trailer-names-another-count`; passing: `list-grew-with-trailer`, `list-grew-with-pin-moved`, `raised-with-trailer`, `local-gate-list-grew` |
| the pure rules | `crates/gatekeeper-engine/tests/ratchet/<case>/case.toml` | one case a folder: a tool's real output captured once (machete, clippy with and without an `allow`, oxlint, jscpd) or a small tree of `.rs` files in; the unit lines, the count or the findings out |
| the list itself | `crates/gatekeeper/tests/check.rs` | today's test of listed, new and stale findings stays; one line more asserts the `<n> listed` note |
| the real tools on real code | gatekeeper's own gates | from its adoption slice every PR to gatekeeper runs all six gates on its own tree, on Linux and macOS |

Every new pack gate has a failing fixture case, or selftest refuses it, as today. `ts/knip`'s cases are not
touched.
