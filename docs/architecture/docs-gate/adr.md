# ADR: the docs gate is on by default, with name sets and generated agent text (#83)

## Summary

Decision: the docs check becomes its own core gate, on in every repo with no setup, over the readme, the docs folder, skill pages, the agent instruction files and the rules folder. Its settings move to gatekeeper's config file; the rest of the arch file stays where it is. A repo leaves only by turning the gate off with a reason, and old hits sit in the gate's list, which only shrinks. A repo may name a command or a generated file that lists its real names; a doc that uses a name outside the list fails. Skill pages must be listed as generated unless the repo says why one is written by hand, and two names may not share one description, which is the fault of blocks 393.
Slices: gatekeeper: a cap raise, the gate with its defaults, name sets, generated agent text, then gatekeeper's own docs and release 1.5.0. After that one pin bump per repo, which also writes the list of old hits; blocks, drover and farrier move their table in that bump.
Answered: (1) design records get the link check only; (2) the cap raise is 10,500, not 10,000, filed as #84, because #75 landed first.

Status: approved by the owner 2026-11-24 (q3393), with the amendment below. Read `README.md` first.

## Question

The docs check is a part of `core/arch` that runs only where a repo wrote a `[docs]` table. How does it become a
check every repo has from one pin bump, what else may it hold docs to (real names, generated agent text), where
does its config live, and in which order do gatekeeper and its consumers change?

## Where it stands (gatekeeper staging 9b6517b, version 1.3.0)

- The check is 87 lines, `crates/gatekeeper-engine/src/arch/docs.rs`: a backticked path must exist, a markdown
  link must resolve, and (`watch`) a config key added or removed needs a changed doc line that names it.
- `check/arch.rs` calls it only when `.drover/arch.toml` has `[docs]`. Three repos have the table: blocks, drover
  and farrier. blocks' `paths` leaves out its 48 skill pages and its design records.
- A repo narrows the check by writing a shorter `paths` list. Nothing asks for a reason.
- A docs finding carries a line number. The arch baseline lists sizes and layer crossings only, so an old docs
  hit cannot be listed: a repo with hits cannot turn the check on without fixing all of them first.
- Every gate already has a baseline that only shrinks, one file per gate, compared by whole lines. `ts/oxlint`
  prints its findings without line numbers, so its list survives an edit.
- `[off]` in `.gatekeeper/config.toml` turns off one gate with a reason. A part of a gate cannot be turned off.
- The generated-file check (in `core/arch`) proves a file equals what the generator writes. It does not prove the
  generator is right: blocks' skill was listed and fresh while every text gave `sheet` the description of `modal`
  (blocks#393).
- Nothing says which docs must be generated. blocks, sendmeter and gatekeeper list their skill pages by choice.
- drover's `[docs] cli = true` checked docs against drover's own command tree. gatekeeper's port left it out,
  since it cannot link a repo's CLI; drover's adoption (#1587) turns it into a test in drover.
- drover reads three list keys of `.drover/arch.toml` for hotspots, and the architect role ends every ADR with
  lines for that file.
- The core pack's CI job sets up no toolchain. gatekeeper has four dependencies and no regex library.
- A repo's `generate` command and its lesson checks already run through the engine's runner.
- Tags reach 1.3.0. The release after #77 is 1.4.0. The line cap is 9,500 and staging counts 9,192.

## Options

### 1. Where the check runs

| Option | Verdict |
|---|---|
| A. Stay a part of `core/arch`, with a default table when the file has none | No. `[off]` would turn off layers, bans and sizes with it, and a repo with no arch file gets "nothing to check". |
| B. Its own gate, `core/docs` | **Yes.** `[off]`, `[budgets]` and the baseline work per gate, with no new code. |

### 2. Where the config lives

| Option | Verdict |
|---|---|
| A. `[docs]` stays in `.drover/arch.toml` | No. The gate's switch (`[off]`) and its list of old hits would be in gatekeeper's folder and its keys in another. |
| B. All of `.drover/arch.toml` moves into `.gatekeeper/config.toml` | No. drover's hotspots, its role prompts and the architect role read that path. A move in two repos with no change in behaviour is its own issue. |
| C. `[docs]` alone moves to `.gatekeeper/config.toml`; the rest stays | **Yes.** One typed table beside `[rust]`, `[ts]` and `[swift]`, parsed once, an unknown key refused. |
| D. A new file, `.gatekeeper/docs.toml` | No. A third config file for seven keys. |

### 3. How a repo narrows the default paths

| Option | Verdict |
|---|---|
| A. `paths` replaces the defaults, as today | No. That is how blocks came to skip its skill. |
| B. The defaults are fixed; a key adds paths; leaving is `[off]` with a reason | **Yes.** |

### 4. Design records (`docs/architecture/**`, `docs/design/**`)

| Option | Verdict |
|---|---|
| A. Checked in full, like any doc | No. A record names files a later slice adds, files a later PR deletes and files of other repos. Each rename would rewrite history, and a baseline that only shrinks cannot take a new record's planned paths. |
| B. Links only | **Yes** (open question 1, answered: links only). A record stays what was decided on its date; its links between its own files must work. |
| C. Not checked | No. A broken link in a record the impl brief points at is drift of the plain kind. |

### 5. The list of old hits

| Option | Verdict |
|---|---|
| A. A section in the arch baseline | No. Another format and another writer, for a gate that left `core/arch`. |
| B. The gate's own baseline, with findings that carry no line number | **Yes.** `.gatekeeper/baseline/core-docs.txt`, written by the `baseline` verb, as for any gate. |
| C. The same with line numbers kept | No. An edit above a listed hit would turn it into a new finding and a stale line. |

### 6. Where the real names come from

| Option | Verdict |
|---|---|
| A. gatekeeper learns each kind of name itself (a CLI's help, a schema) | No. It cannot link or run every repo's code, and each new kind would be a release. |
| B. The repo names a command, or a generated file, that prints `set`, `name` and an optional description per line | **Yes.** A shell script can print it, and no parser is needed. |
| C. The same as JSON | No. Harder to print from a script, no gain. |
| D. A span is matched to a set by a regular expression | No. A new dependency. A prefix before the name covers commands, flags, gate ids, rule ids and config keys. |

### 7. How a repo marks agent text as generated or written by hand

| Option | Verdict |
|---|---|
| A. A marker line inside each generated file | No. A page written by hand has no marker, so it passes: the default points the wrong way. |
| B. By path: skill pages must be generated; a key adds paths; a page written by hand is named with its reason | **Yes.** |
| C. Every doc must be generated | No. A readme and a guide are written. |

### 8. Generated, and wrong (blocks#393)

| Option | Verdict |
|---|---|
| A. The listing rule alone | No. blocks' skill was listed and fresh. |
| B. Find twin lines in the text: two lines equal but for a name | No. A guess: tables and short lists repeat by design. |
| C. The name list carries each name's description; every name has one and no two share one | **Yes.** It reads the registry the texts are made from, and it cannot guess wrong. |
| D. Leave it to a test in each repo | No. blocks#393 adds that test for blocks; the next repo would write it again. |

## Decision

1. **Its own gate.** `core/docs` runs the built-in check `docs`. It is on in every repo that has the core pack,
   with or without config. `[off]` with a reason is the only way out, and `check`, `list` and the summary show it.
2. **The config is `[docs]` in `.gatekeeper/config.toml`** (option 2C). Every key is optional. A `[docs]` table
   left in `.drover/arch.toml` is refused by `core/arch` with a line that names the new home. Layers, bans, sizes,
   frozen, generated, shared and comments stay in `.drover/arch.toml`.
3. **Default paths are fixed:** `README.md`, `docs/**/*.md`, `skill/**/*.md`, `CLAUDE.md`, `AGENTS.md`,
   `.drover/*.md`. `also` adds paths. No key removes one.
4. **A doc has one of three classes** (`shapes.md`). A current doc gets every check. A record
   (`docs/architecture/**`, `docs/design/**`, and what `[docs.records]` adds with a reason) gets the link check
   only. Agent text (`skill/**/*.md` and what `generated` adds) gets every check and must be listed as generated.
5. **A finding has no line number**: the file, the span and what is wrong. The gate's baseline is the generic one,
   `.gatekeeper/baseline/core-docs.txt`. A listed hit passes, a new one fails, a fixed one must leave the list.
6. **One result on every host.** A missing path under a folder git ignores is no finding, so a doc that names
   build output reads the same on a lane's host and in a fresh CI clone.
7. **Name sets** (option 6B). `[docs.names]` gives one source, `run` (a command) or `file` (a path), and for each
   set the prefixes that mark a backticked span as one of its names. A span that starts with a prefix claims the
   word after it; a word the set does not hold is a finding. A `file` must be listed as generated, so a list kept
   by hand cannot drift. A `run` command must work with no build, since the core job sets up no toolchain.
8. **Generated agent text** (option 7B). Each file of the agent-text class must match a `generated` glob of
   `.drover/arch.toml`, where the existing check keeps it equal to its generator's output. A page written by hand
   is named in `[docs.written]` with its reason.
9. **One description per name** (option 8C). When a set's lines carry descriptions, a name with none and two
   names with the same one are findings. This is blocks#393: `sheet` with the description of `modal`.
10. **`watch` and `skip` move as they are.** `watch` still needs a base and a PR body with no `no doc:` line; a
    changed line in a record does not count as documentation.
11. **An excuse that matches no file is a finding** (`[docs.records]`, `[docs.written]`), so it leaves with the
    files it excused.
12. **Release 1.5.0, then one bump per repo.** The bump PR runs `pin 1.5.0` and `baseline core/docs` and commits
    both. A repo that skipped 1.4.0 goes straight to 1.5.0.

How an old table moves, in the bump PR of the three repos that have one:

| Key in `.drover/arch.toml` `[docs]` | In `.gatekeeper/config.toml` `[docs]` |
|---|---|
| `paths` | deleted where the defaults cover it; an entry outside the defaults goes to `also` |
| `watch`, `skip` | moved unchanged |
| `cli = true` (drover) | dropped at drover's adoption (#1587), where it becomes a test; a name set may replace that test later |

## Consequences

- A repo with no config is checked from its bump on. Its old hits are counted once, in a file the owner can read,
  and can only go down.
- A rename or a delete must fix the current docs that name the file, in the same PR. That is the point.
- A finding no longer shows its line; it quotes the span, which a search finds. In exchange an edit above a
  listed hit changes nothing.
- The five docs cases leave `core/arch`'s fixtures for `core/docs`'s, and their `expect` lines lose the line
  number. No other `expect` file of `core/arch` changes. `core/arch` no longer reads the PR body.
- Records get less than the issue's default paths promise: their paths and names are not checked. drover checks
  its records' paths today and would stop. Open question 1.
- A description is checked in the name list, not in each text. A generator that reads the right description and
  prints it under the wrong name is not caught here; that stays a test of the generator.
- A names command runs on every `check`. It is the repo's own code, started through the runner like `generate`.
- A repo that cannot print its names without a build keeps them in a generated file and pays nothing in the core
  job.
- `core/docs` is never cached: it reads the base and the PR body.
- gatekeeper gains about 400 lines: 9,192 today, 9,500 the cap. Open question 2.

## Slices

Gatekeeper (five issues to file). They run in order: each adds a field to the same config type.

| # | Issue | Blocked on | AC |
|---|---|---|---|
| S0 | cap: raise gatekeeper to 10,500 (#84, filed by the orchestrator) | the owner's answer | the raise file with its reason; nothing else |
| S1 | `core/docs`, the gate: defaults, classes, `also`, `skip`, `watch`, `[docs.records]`, the baseline | S0 | a repo with no `[docs]` fails on a stale path in a skill page; `arch/docs.rs` is gone and `core/arch` refuses a `[docs]` table, naming the new home; findings have no line number; a listed hit survives an edit above it; a record's planned path passes and its broken link fails; a missing path under an ignored folder passes; the S1 cases of `cases.md` are in and selftest passes |
| S2 | name sets: `[docs.names]`, `run` or `file`, the prefixes | S1 | a span with a name outside its set fails and names the set; a placeholder and a fenced block pass; a failing command, a bad line, an empty set and a `file` not listed as generated are each one finding; the S2 cases are in |
| S3 | generated agent text: the listing rule, `generated`, `[docs.written]`, descriptions | S2 | a skill page outside `generated` fails; one named in `[docs.written]` passes; two names with one description fail, and the case is blocks#393's two lines; the S3 cases are in |
| S4 | gatekeeper's own docs under the gate; release 1.5.0 | S3, 1.4.0 released (#77) | gatekeeper declares its verbs and each pack's gates as name sets; its baseline is committed or absent; the readme has the gate, the keys and the 1.5.0 upgrade note; the skill is regenerated; the PR body lists, for each consumer's integration head, the hits the default finds and the gate's time |

Every repo on gatekeeper (one issue each, after S4), all with the same AC: `./.gatekeeper/run pin 1.5.0`, then
`./.gatekeeper/run baseline core/docs`; the list is committed (no file when there is no hit); `.drover/arch.toml`
has no `[docs]`; `check` is green at the head; the PR body gives the number of listed hits; no `[off]` line for
`core/docs` without the owner's answer.

| Repo | Blocked on | Also in its bump |
|---|---|---|
| blocks | S4 | its table moves (`skip` stays, `paths` goes); a `kind` set with descriptions, in a file its generator writes: the check that fails on #393's fault |
| drover | S4, its adoption (#1587) | its table moves (`watch`, `skip`); its role prompts join through `also` |
| farrier | S4, its adoption | its table moves at adoption |
| sendmeter | S4 | its skill folder under `.claude/` joins through `generated` (it is listed in its arch file already) |
| shed | S4 | each skill page and command page is listed as generated or named in `[docs.written]` |
| cooee, swag, loom, patchwork, one private repo | S4 | nothing |
| beacon, scene-esp32, compact | S4, their adoption | nothing: they adopt at 1.5.0 or later with the gate on |

## How it is tested

- **Fixtures.** `core/docs` gets a fixture folder like every gate: one folder per case with the files it lays
  over the base repo and the line it must print, or `passes`. `cases.md` lists each decision with its case.
  A case that needs a `[docs]` table carries its own `.gatekeeper/config.toml`; a names case carries its script or
  its list.
- **Selftest.** `selftest` runs every case: the base repo must pass with the defaults alone, each failing case must
  print its line, and a gate with no failing case fails. The baseline cases run through the same gate run, so
  "listed passes" and "fixed must leave" are proven by the generic code, not by a copy.
- **The config edge.** One engine test over small `[docs]` tables: an unknown key, both `run` and `file`, a prefix
  without `{name}`, an empty reason. Each is a load error with the key's name.
- **Before the release** (S4): a run of the gate on every consumer's integration head, with the hit count and the
  time in the PR body, so the owner sees each repo's debt and the 10 s budget is checked on real docs.

## Open questions, answered

1. **Records.** The owner chose links only: a record under `docs/architecture/**` or `docs/design/**` gets the
   link check and nothing else, as decision 4 says.
2. **The cap.** One raise to 10,500, not 10,000, filed as #84, because #75 lands first.

## `.drover/arch.toml` lines

Gatekeeper, in S1:

```toml
[[collect]]
dir = "crates/gatekeeper-engine/src/docs"
glob = "*.rs"

[[ban]]
pattern = "docs::"
paths = ["crates/gatekeeper-engine/src/arch/**"]
reason = "the docs gate reads arch's config and diff; arch never calls the docs gate"
```

No file joins `frozen` and none joins `generated`: the check file, the cases and the gate file land in folders
that are collected already, and `sh scripts/gen.sh` writes the check list. In S4, gatekeeper's own
`.gatekeeper/config.toml` gains the `[docs.names]` table shown in `usage.md`.

A consumer's bump PR deletes lines from its `.drover/arch.toml` (the `[docs]` table) and adds none.
