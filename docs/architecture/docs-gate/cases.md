# The docs gate: each decision and the case that proves it (#83)

A case is a folder, `packs/core/fixtures/docs/<case>/`: the files it lays over the pack's base repo, and either
`expect` (the gate must fail and print that text) or `passes`. `selftest` runs every case and the base repo alone,
which must pass on the defaults. A case that needs a `[docs]` table carries its own `.gatekeeper/config.toml`; a
`body` file is the PR body; `.gatekeeper/baseline/core-docs.txt` in a case is its list of old hits.

## S1: the gate, its defaults and its list

| Decision | Case | Result |
|---|---|---|
| on with no `[docs]` table: a stale path in a skill page | `default-skill-page` | fails: `skill/tool/SKILL.md: ` and the span |
| the other default paths: `CLAUDE.md`, `AGENTS.md`, `.drover/rules.md` | `default-agent-files` | fails: one line per file |
| real paths, a line in range, a relative path, a link | `real-paths` (was arch's `docs-name-real-paths`) | passes |
| a missing path, a line past the end, a broken link | `stale-path-line-and-link` (moved) | fails: the three lines, no line number |
| a span and a path inside a code block | `in-a-code-block` | passes |
| `also` adds a folder | `also-adds-a-folder` | fails: a file under `roles/` |
| `skip` leaves a span unread | `skip-a-span` | passes |
| a record's planned path | `record-planned-path` | passes |
| a record's broken link | `record-broken-link` | fails: the link line |
| `[docs.records]` adds a folder, with its reason | `records-added` | passes |
| an excuse that matches no file | `excuse-matches-nothing` | fails: ``[docs.records] `docs/old/**` matches no file`` |
| a missing path under an ignored folder | `ignored-path` | passes |
| a listed hit passes, and an edit above it changes nothing | `listed-hit-edit-above` | passes |
| a fixed hit still in the list | `listed-hit-fixed` | fails: `stale: shrink the baseline` |
| a watched key with a changed doc line | `watched-key-documented` (moved) | passes |
| a watched key, `no doc:` in the PR body | `watched-key-no-doc` (moved) | passes |
| a watched key with neither | `watched-key-undocumented` (moved) | fails: the two lines, no line number |
| a watched key named only in a record | `watched-key-in-a-record` | fails |

In `core/arch`'s fixtures, one new case: `docs-table-moved` has `[docs]` in `.drover/arch.toml` and must fail with
`[docs] moved to .gatekeeper/config.toml`. Its five `docs-*` cases are the ones moved above.

## S2: name sets

| Decision | Case | Result |
|---|---|---|
| a name of its set, from `run` | `name-in-its-set` | passes |
| a name outside its set | `name-outside-its-set` | fails: ``no verb `spwan` `` |
| a placeholder, a flag after the prefix, a span in a code block | `no-claim` | passes |
| two prefixes for one set | `two-prefixes` | fails on the second prefix's span |
| the list from `file`, listed as generated | `names-from-a-file` | passes |
| a `file` not listed as generated | `names-file-not-generated` | fails: `is not listed as generated` |
| the command exits 1 | `names-command-fails` | fails: `exited 1` |
| a line that is no name line | `names-bad-line` | fails: `line 2 is not` |
| a set the config names prints no name | `set-prints-nothing` | fails: ``set `verb` has no name`` |
| a name in a record | `name-in-a-record` | passes |
| `skip` covers a span that would claim | `skip-a-claim` | passes |

## S3: generated agent text

| Decision | Case | Result |
|---|---|---|
| a skill page under a `generated` glob | `skill-page-listed` | passes |
| a skill page outside every `generated` glob | `skill-page-unlisted` | fails: `agent text is not listed as generated` |
| the same with no `.drover/arch.toml` at all | `skill-page-no-arch-file` | fails: the same line |
| a page named in `[docs.written]` | `skill-page-written` | passes |
| `generated` adds a folder | `generated-adds-a-folder` | fails: a file under `docs/commands/` |
| two names with one description (blocks#393's two lines) | `two-names-one-description` | fails: ``kind `sheet` has the description of `modal` `` |
| a described set with a name that has none | `name-without-description` | fails: `has no description` |
| a set with no descriptions at all | `set-not-described` | passes |

## Outside the fixtures

| What | Where | Proves |
|---|---|---|
| the config edge | one engine test, a table of small `[docs]` texts | an unknown key, `run` with `file`, neither, a prefix without `{name}` or with nothing before it, an empty reason, a set name with a capital: each a load error that names the key |
| `Prefix::claim` | unit cases beside the type | the word rule: where a word starts and ends |
| `[off]` for `core/docs` | the plan's existing cases for `[off]` | nothing new: the gate is a gate |
| real docs | S4's PR body | hits and time per consumer, at each integration head |
