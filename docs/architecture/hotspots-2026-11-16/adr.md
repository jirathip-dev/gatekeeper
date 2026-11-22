# ADR: hotspots week of 2026-11-16: the skill's text as a folder, the run file by job (#67)

## Summary

Decision: both files are hot because every feature writes its own piece into them. The skill's guide text becomes
a folder with one file per feature, and the old file is frozen. The run file is split in four by job; a run's
inputs, a gate's report and a run line's placeholders each get one owner, and the small rest is marked shared
under a size limit.
Slices: one pull request, two commits: the skill's topics folder, then the run split. Nothing a user sees changes.
Open questions: none.

## Status

Proposed 2026-11-22 (architect lane). Nothing is built. Read `usage.md` first.

## Question

The report names `crates/gatekeeper-engine/src/skill.rs` (6 merged PRs) and `crates/gatekeeper-engine/src/run.rs`
(5 merged PRs, 9 predicted conflicts). A hot file is a one-slot queue: pull requests that touch it land one at a
time. What in each file draws every feature to it, and what layout lets two features land without meeting there?

## What the code says first

The last six pull requests on each file (#32, #36, #38, #60, #62, #63, #64 between them):

| File | What a pull request added | PRs | Why it lands there |
|---|---|---|---|
| `skill.rs` | sentences to `SKILL.md`'s opening | #32, #38, #62, #63, #64 | the text is one string literal, lines 48 to 80 |
| `skill.rs` | a line on a pack page; two kinds of fixture file on an example page | #32, #36 | the page renderers share the file |
| `run.rs` | an input to the run: `results`, `env`, `timings` | #32, #36, #64 | `RunCtx` is written out field by field in six places |
| `run.rs` | a field of the gate's report: `cache`; `selection`, `files` | #32, #64 | the report is written out twice in `run.rs` |
| `run.rs` | a place in the run line: `{pr-body}`, `{files}`, `{report}` | #36, #62, #64 | one function knows every place |
| `run.rs` | a layer round a gate's run: the cache, the selection | #32, #38, #62, #64 | the loop holds the layers inline |
| `run.rs` | a helper with no better home: `changed` | #60, #62 | since moved to `scope` |

- Each new input edited seven files: `run.rs`, `selftest.rs`, `cmd/check.rs` and four test files. In most of
  them the edit is one line saying the new field is unset.
- `run.rs` is 390 lines; the size rule allows 400. The next feature cannot land there without a split.
- `skill/gatekeeper/**` is already `generated`: a conflict in the built skill is solved by `just gen`.
- `core/arch` reads `frozen` from the base's `arch.toml`, and judges each hunk: more lines added than removed
  fails. So the pull request that freezes a file may still rewrite it, and later ones may reword it.
- The pinned drover (`build-staging-04ae328`) knows `shared`. A shared file must fall under a `[[size]]` rule.
- No open pull request touches either file. `src/` is at 7,565 of 7,700 lines.

## Options

**1. The skill's text**

| Option | Verdict |
|---|---|
| A. Leave `skill.rs`, list it `shared` | No. It is a list: one sentence a feature, appended at the same spot. |
| B. One Rust file a topic, with a generated `mod.rs`, as `check/` and `cmd/` do | No. Prose in string literals with escaped braces, and one more generated file. |
| C. One markdown file a topic in a folder the build script lists, as it lists `packs/` | **Yes.** No list is checked in, and the text is written as it is read. |
| D. One page a topic, linked from `SKILL.md` | Not now. It changes what a lane reads; this conversion changes no word. |

**2. What stays in `skill.rs`**

| Option | Verdict |
|---|---|
| A. Everything but the text, frozen | No. The page renderers grew twice in two weeks; a frozen file would refuse the third. |
| B. Everything but the text, not listed | No. The file stays on next week's report for churn that has already ended. |
| C. The opening and the assembly, frozen; one file a page kind beside it | **Yes.** The frozen file is the old list; a renderer change has its own file. |

**3. The run file**

| Option | Verdict |
|---|---|
| A. List `run.rs` `shared` as it is | No. At 390 of 400 lines it is full, and a new input would still edit seven files. |
| B. Split it by job, nothing else | Not enough. The same three lists move to smaller files and stay shared. |
| C. A folder with one file a placeholder and one a layer, with a generated `mod.rs` | No. There are eight places and two layers. The layers' order is a decision (the selection is in the cache key) and belongs in one function, not in file names. |
| D. Split by job, give each of the three lists one owner, and list the small rest `shared` under its own size limit | **Yes.** |

## Decision

1. **Topics.** `crates/gatekeeper-engine/src/skill/topics/<topic>.md`, one file a feature. Plain markdown, copied
   as it is into `SKILL.md` between the opening and `## Packs`, by file name. The first two are `cache.md` and
   `select.md`: today's two paragraphs, word for word, one sentence a line.
2. **Collected by the build.** `build.rs` lists the folder into `OUT_DIR/topics.rs`, as it does `packs/`.
3. **`skill.rs` is frozen.** It keeps `SkillFile`, `files`, `write`, the two path helpers and `SKILL.md`'s frame.
   The renderers move beside it: `skill/pack.rs`, `skill/example.rs`, `skill/tools.rs`.
4. **`run.rs` by job.** `run/cx.rs` (the contexts and where a run happens), `run/gate.rs` (one gate's run),
   `run/line.rs` (a run line's places). `run.rs` keeps the loop over the jobs and `one()`, the layers round a gate.
5. **A run's inputs have one owner.** `RunCtx` is `#[non_exhaustive]` with `RunCtx::lane(root, cache)`. The six
   places that write it out call that and assign what they set.
6. **A gate's report has one owner.** `GateReport::unrun(gate, outcome)` in the model names every field once;
   the struct is `#[non_exhaustive]`. `run::unrun` is deleted and its five callers moved.
7. **A place belongs to its feature.** `select` hands `{files}` and `{report}` over as `Picked::vars()`.
   `run/line.rs` fills what it is given, then the places every line has.
8. **`Millis` from a `Duration` lives in the model.** `run::millis` is deleted and its callers moved.
9. **`run.rs` is `shared`, and held to 150 lines.** It is where the layers meet, by nature. The limit keeps it
   the loop and the layers: anything else goes to a file under `run/` or to the feature's own module.

Shapes: `shapes.md`. Who may call whom, and the chart of `one()`: `boundaries.md`.

## Consequences

- Documenting a feature is one new file. Two such pull requests meet only in the built `SKILL.md`, which is
  regenerated.
- A new input opens two files, not seven. A new report field or run-line place opens none under `run/`.
- A new layer still opens `run.rs`. Two such pull requests no longer queue; if they collide, one update round.
- `skill.rs` cannot grow. A new kind of page (today: the skill, a pack, an example) needs an ADR.
- Topics are ordered by file name. A topic cannot lean on another being read first.
- `SKILL.md` gains line breaks inside its two long paragraphs. Its words, and every other built file, are the same.
- `skill.rs` and `run.rs` stay as files beside their folders, though `cache/` and `select/` use `mod.rs`: the
  `frozen` and `shared` lines then name the files the report names, and the two leave the queue on the next pass.
- Tests change only where they build a context: four files, no assertion. Behaviour is unchanged, but this is
  not a `--refactor` issue in drover's sense, which allows tests to move only.
- `src/` grows by roughly 30 lines (four files' imports, `RunCtx::lane`), inside the 135 of headroom. No raise.
- Older ADRs name `run.rs` by line count. They are history and are not edited.
- If `run/gate.rs` or `run/cx.rs` turns hot, the report will say so with numbers. Neither is listed in advance.

## How the pull request proves it

```
just gen
git diff --exit-code <base> -- skill/gatekeeper/packs skill/gatekeeper/examples     # 0: the same bytes
git show <base>:skill/gatekeeper/SKILL.md | tr -s '[:space:]' '\n' > "$DROVER_SCRATCH_TARGET/old.words"
tr -s '[:space:]' '\n' < skill/gatekeeper/SKILL.md | diff "$DROVER_SCRATCH_TARGET/old.words" -   # 0: the same words
./.gatekeeper/run check                                                             # 0: every gate, core/arch included
```

## Slices

One pull request for #67, on this branch. Each commit leaves the tree green.

| # | Commit | Carries |
|---|---|---|
| 1 | The skill's topics folder: decisions 1 to 3 | `frozen`, and the `[[collect]]` table |
| 2 | The run split: decisions 4 to 9 | `shared`, and the `[[size]]` table |

## The `.drover/arch.toml` lines

Two top-level keys, beside `generated` and before the first table:

```toml
frozen = ["crates/gatekeeper-engine/src/skill.rs"]
shared = ["crates/gatekeeper-engine/src/run.rs"]
```

One table after the last `[[collect]]`:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/src/skill/topics"
glob = "*.md"
```

One table after the existing `[[size]]`, which stays and still covers every file:

```toml
[[size]]
paths = ["crates/gatekeeper-engine/src/run.rs"]
max_file_lines = 150
```

`generated` and `generate` do not change: `skill/gatekeeper/**` is listed, and `sh scripts/gen.sh` writes it.
