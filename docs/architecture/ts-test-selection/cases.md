# Per-file test selection: each rule and the case that proves it (#46)

A case is a folder, `crates/gatekeeper-engine/tests/select/<case>/case.toml`, over one small workspace,
`tests/select/_base/`. One test globs the folder and fails on a count that is off, as `tests/affected/` does.

```
_base/
  vitest.config.ts  package.json  package-lock.json  graph.toml
  apps/web/src/price.ts            imports packages/ui/src/button.ts
  apps/web/src/price.test.ts       imports ./price
  packages/ui/src/button.ts        packages/ui/src/button.test.ts
  packages/ui/src/label.ts         packages/ui/src/label.test.ts
  packages/tools/src/sum.ts        packages/tools/src/sum.test.ts
```

`_base` has no guard, no walker and no reader, so a case adds exactly the test its rule is about:

```toml
# tests/select/reader-basename/case.toml
about = "a test that reads a changed file beside it is found by the file's basename"

[files]                                   # laid over _base before the base commit
"apps/web/src/banner.html" = "<p>one</p>"
"apps/web/src/banner.test.ts" = "readFileSync(new URL('./banner.html', import.meta.url))"

[change]
"apps/web/src/banner.html" = "<p>two</p>"

closure = []                              # what vitest lists for this change

[expect]
"apps/web/src/banner.test.ts" = ["reads-basename"]
```

A case may also set `delete`, `select` (the gate's table), `trigger` and `full` (in place of `expect`).

Two tests read every case:

- **pure**, in `cargo test`: `closure` and the workspace's test files are the evidence; `select` must return
  exactly `expect`, files and reasons.
- **real vitest**, in gatekeeper's own gate `select-vitest`: the case is laid out as a git repo, and vitest
  must list exactly `closure` for the change and exactly the workspace's tests without it. vitest is pinned in
  `_base/package.json` and its lockfile, and only this gate installs it (the owner's yes in the owner's answers, 2026-11-22).

So a stubbed closure cannot drift from what vitest does.

**How a case proves that leaving its rule out would miss a test.** `expect` is the whole selection with every
reason. The test named for a rule carries that rule as its only reason, so no other rule selects it. The pure
test also fails when a reason or a `Full` kind appears in no case.

## The union

| Member | Case | The change | Selected, and by what alone |
|---|---|---|---|
| closure | `closure-import` | `packages/ui/src/button.ts` modified | `button.test.ts` and `apps/web/src/price.test.ts`, which reaches it through `price.ts`: `closure`. `label.test.ts` and `sum.test.ts` are not selected |
| closure, vitest's own trigger | `closure-config-changed` | `vitest.config.ts` modified | all four tests: `closure` |
| always-run, a configured path | `always-path` | `docs/guide.md` modified; `packages/tools/guards/licence.test.ts` under `always` | the guard, which sits in a project: `always-path` |
| always-run, in no project | `always-no-project` | `docs/guide.md` modified; `scripts/release.test.ts` | it: `always-no-project` |
| fs-readers, the repo path | `reader-path` | `docker/Dockerfile` modified; a test's text has `'docker/Dockerfile'` | it: `reads-path` (no extension, so the basename rule is silent) |
| fs-readers, the basename | `reader-basename` | above | `banner.test.ts`: `reads-basename` |
| fs-readers, a whole token only | `reader-basename-not-a-token` | `supabase/config.toml` modified; one test names `my-config.toml`, one `config.toml.bak`, a guard exists | the guard only |
| fs-readers, a walker in the changed file's project | `walker-own-project` | `apps/web/src/banner.css` modified, imported by nothing; `apps/web/src/no-inline.test.ts` and `packages/tools/src/tree.test.ts` each call `readdirSync(here)` | the first: `reads-walks-project`. The walker in `packages/tools` is not selected |
| fs-readers, a walker that names a directory above the changed file | `walker-ancestor-dir` | `.github/workflows/ci.yml` modified; one walker in `packages/tools` has `join(root, '.github', 'workflows')`, one has `'supabase/migrations'` | the first: `reads-walks-dir`. The second is not selected |
| fs-readers, a top-level directory only between quotes or slashes | `walker-top-level-dir` | `docs/guide.md` modified; four walkers, with `join(root, 'docs')`, `` `${root}/docs/` ``, `'docs-site'` and `'reads the docs'` | the first two: `reads-walks-dir` |
| fs-readers, each walker token | `walker-each-token` | `docs/guide.md` modified; eleven tests that name `'docs'`, one token each: `readdirSync`, `opendir`, `scandir`, `globSync`, `fdir`, `import.meta.glob`, `ls-files`, `git grep`, `execFileSync`, `execSync`, `spawn`; two more that name it, one with `globalThis`, one with no token | the eleven: `reads-walks-dir`. The other two are not selected |
| the text rules skip comment-only lines | `comment-only-line` | `supabase/config.toml` modified; one test names `config.toml` on a `//` line and on a `*` line of a block comment; one has `load(dir) // config.toml` | the second: `reads-basename`. The first is not selected |
| project readers, per test | `project-reader` | `packages/ui/src/button.ts` modified; `packages/tools/src/layout.test.ts` has `'packages/ui/src'` | it: `names-project`. A test naming `packages/ui-kit` is not selected, nor is `sum.test.ts` beside it in `packages/tools` |

## The widenings

| Rule | Case | The change | Selected, and by what alone |
|---|---|---|---|
| a deleted file's former project | `deleted-module` | `packages/ui/src/old.ts` deleted; nothing imports it | `button.test.ts`, `label.test.ts`: `widened-former-project`; `closure` is empty. No test of `apps/web` or `packages/tools` |
| a renamed-away file | `renamed-module` | `packages/ui/src/label.ts` moved to `tag.ts`; `label.test.ts` edited to import it | `button.test.ts`: `widened-former-project` alone. `label.test.ts`: that and `closure` |
| the exception, a deleted test file | `deleted-test-file` | `packages/ui/src/label.test.ts` deleted; a guard exists | the guard only: `button.test.ts` is not selected |
| a deleted module's import specifier | `deleted-module-specifier` | `packages/ui/src/theme.ts` deleted; `apps/web/src/theme.test.ts` loads `'@ui/theme'` | it, in another project: `widened-specifier` alone. The tests of `packages/ui`: `widened-former-project` |

In `deleted-module-specifier` the test loads the module through a variable, so vitest lists the files without
an error. What vitest does with a static import of a missing file is pinned by one more case in S2.

## The full suite

| Rule | Case | The change | Result |
|---|---|---|---|
| an empty selection is the full suite | `empty-means-full` | `docs/guide.md` modified; no guard, walker or reader | `full = "empty"` |
| a file of `full` | `full-file-changed` | `package-lock.json` modified, listed in `full` | `full = { file = "package-lock.json" }` |
| the gatekeeper folder | `gatekeeper-folder-changed` | `.gatekeeper/config.toml` modified | `full = { file = ".gatekeeper/config.toml" }` |
| not a change | `not-a-change` | `trigger = { everything = "forced" }` | `full = { not-a-change = "forced" }` |

## What the issue is about

| Rule | Case | The change | Result |
|---|---|---|---|
| a file outside every project no longer runs everything | `file-outside-every-project` | `.github/workflows/ci.yml` modified; a guard exists | the guard only. The same change under the per-project rule is `tests/affected/file-outside-every-project`: every project |

## Shards and timings

One folder a case under `crates/gatekeeper-engine/tests/split/`: files, timings, the shard count and each
shard's expected slice.

| Decision | Case |
|---|---|
| longest first, each to the lightest shard | `by-duration` |
| a file with no time takes the median | `unknown-takes-median` |
| no timings at all: an even split that every shard agrees on | `no-timings` |
| fewer files than shards: the rest are `EmptyShard`, skipped, passing | `fewer-files-than-shards` |
| a full run is split the same way | `full-is-split` |
| for any timings the shards are a partition | `tests/split.rs`, one test over generated timings |
| shards that report different digests fail `done` | `tests/plan/shards-disagree` |
| slices that do not sum to the selection fail `done` | `tests/plan/slices-do-not-sum` |

## The result cache

| Decision | Case in `tests/cache/` |
|---|---|
| the same tree and the same list hit | `selected-same-list-hits` |
| the same tree and another base, so another list, miss | `selected-list-differs-misses` |

## Through the binary

In `crates/gatekeeper/tests/select.rs` and `replay.rs`, with a stub script in place of vitest:

| Decision | Test |
|---|---|
| `check` prints the selection line and writes `selection.json` with every reason | `check_prints_the_selection_and_writes_the_list` |
| 5,000 selected files reach the run line as arguments | `files_are_passed_as_arguments` |
| a vitest list that fails fails the gate and runs nothing | `a_failed_list_fails_the_gate` |
| a sharded `[select]` gate without `{report}` is refused | `a_sharded_select_gate_needs_a_report` |
| `ci plan` puts one timings entry in every shard's matrix entry | `ci_plan_names_one_timings_entry` |
| `ci done` merges the shards' times into one file per gate | `ci_done_merges_the_times` |
| `replay` prints per-run counts, the totals and each missed failure | `replay_reports_counts_totals_and_missed_failures` |
| `replay` exits 1 when gatekeeper missed a failure | `replay_exits_1_on_a_missed_failure` |

## Fixtures and selftest

`[select]` is a key of a gate file and no pack gate uses it, so there is no pack fixture and `selftest` does not
change. The skill's generated text gains the selection lines of `usage.md`.
