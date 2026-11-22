# Hotspots, week of 2026-11-16 (#67)

## Summary

Decision: both files are hot because every feature writes its own piece into them. The skill's guide text becomes
a folder with one file per feature, and the old file is frozen. The run file is split in four by job; a run's
inputs, a gate's report and a run line's placeholders each get one owner, and the small rest is marked shared
under a size limit.
Slices: one pull request, two commits: the skill's topics folder, then the run split. Nothing a user sees changes.
Open questions: none.

## Read in this order

| File | What |
|---|---|
| `usage.md` | how a pull request documents a feature, how a test and `check` build a run, the loop, a report, a run line's places |
| `adr.md` | the question, what the last six pull requests on each file did, the options, the nine decisions, consequences, the proof, the `.drover/arch.toml` lines |
| `shapes.md` | a topic file, `SKILL.md` assembled, `RunCtx::lane`, `GateReport::unrun`, the places and their owners |
| `boundaries.md` | where the code goes, what opens which file, which module may call which, the chart of one gate |

## The two files

| File | Class | Why it is hot | What it becomes |
|---|---|---|---|
| `crates/gatekeeper-engine/src/skill.rs` | a list | five of its last six pull requests added sentences to one string | a folder of topic files; the file is frozen |
| `crates/gatekeeper-engine/src/run.rs` | code that holds three lists | every feature adds an input, a report field, a placeholder or a layer | four files by job; each list gets one owner; the rest is shared, at most 150 lines |

## The layout on one screen

```
 today                                         after
 ─────                                         ─────
 skill.rs   202 lines                          skill.rs            frozen: the frame of SKILL.md, files, write
   SKILL.md's text, one string            ──▶  skill/topics/*.md   one file a feature, listed by the build
   the pack page, the example page,       ──▶  skill/pack.rs · skill/example.rs · skill/tools.rs
   the tools section

 run.rs     390 of 400 lines                   run.rs              shared, 150 lines at most: the jobs, the layers
   RunCtx, CheckCx, where a run happens   ──▶  run/cx.rs           RunCtx::lane: a caller sets only what it needs
   one gate's run, its steps              ──▶  run/gate.rs
   a run line's places                    ──▶  run/line.rs         a feature hands its own places over
   the report, written out twice          ──▶  the model           GateReport::unrun names each field once
```

| A pull request that | Today opens | After |
|---|---|---|
| documents a feature | `skill.rs`, at the same spot as every other | its own topic file |
| gives a run a new input | seven files | two |
| reports something new for a gate | `run.rs`, twice | the model and its own module |
| gives a run line a new place | `run.rs` | its own module |
| wraps a gate's run in a new layer | `run.rs`, one at a time | `run.rs`, not queued |

## What it never does

- Change a verb, a flag, a row, a log or `report.json`.
- Change a word of the skill. The pack and example pages are the same bytes.
- Add a dependency, a generated file or a checked-in list.
- Move a public path: every `run::` and `skill::` name callers use today stays.
