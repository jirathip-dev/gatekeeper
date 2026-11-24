# core/docs: what a failure looks like

Each case is one change on the pack's fixture repo. `gatekeeper selftest` runs every case and fails unless the gate fails on it printing the line shown, so these examples cannot go stale.

## also-adds-a-folder

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
also = ["roles/**/*.md"]
```

Writes `roles/impl.md`:

```
See `lib/gone.rs`.
```

The gate fails and its log prints `roles/impl.md: `lib/gone.rs`: no such path`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## default-agent-files

Writes `.drover/rules.md`:

```
- don't add a line to list.txt, as `lib/gone-rules.rs` says (enforced by: CI arch-check)
```

Writes `AGENTS.md`:

```
See `lib/gone-agents.rs`.
```

Writes `CLAUDE.md`:

```
See `lib/gone-claude.rs`.
```

The gate fails and its log prints `.drover/rules.md: `lib/gone-rules.rs`: no such path
AGENTS.md: `lib/gone-agents.rs`: no such path
CLAUDE.md: `lib/gone-claude.rs`: no such path`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## default-skill-page

Writes `skill/tool/SKILL.md`:

```
# tool

Run `lib/gone.rs` first.
```

The gate fails and its log prints `skill/tool/SKILL.md: `lib/gone.rs`: no such path`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## excuse-matches-nothing

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.records]
"docs/old/**" = "old decisions"
```

The gate fails and its log prints `[docs.records] `docs/old/**` matches no file`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## generated-adds-a-folder

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
generated = ["docs/commands/**/*.md"]
```

Writes `docs/commands/spawn.md`:

```
# spawn

Reads `lib/cli.rs`.
```

The gate fails and its log prints `docs/commands/spawn.md: agent text is not listed as generated`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## ignored-path

Writes `.gitignore`:

```
target*/
out/
```

Writes `docs/b.md`:

```
The run writes `out/b.log`.
```

The gate passes.

## in-a-code-block

Writes `docs/c.md`:

```
# c

```
`lib/gone.rs`
```
```

The gate passes.

## listed-hit-edit-above

Writes `.gatekeeper/baseline/core-docs.txt`:

```
docs/b.md: `lib/gone.rs`: no such path
```

Writes `docs/b.md`:

```
# b

A line added above the old hit.

See `lib/gone.rs`.
```

The gate passes.

## listed-hit-fixed

Writes `.gatekeeper/baseline/core-docs.txt`:

```
docs/b.md: `lib/gone.rs`: no such path
```

The gate fails and its log prints `stale: shrink the baseline (./.gatekeeper/run baseline core/docs): docs/b.md: `lib/gone.rs`: no such path`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## name-in-a-record

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/architecture/x.md`:

```
A later slice adds `tool spwan`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate passes.

## name-in-its-set

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn --dry-run <f>` or `tool queue`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate passes.

## name-outside-its-set

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spwan`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate fails and its log prints `docs/b.md: `tool spwan`: no verb `spwan``.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## name-without-description

Writes `.drover/arch.toml`:

```
generated = ["names.tsv"]
generate = "true"
```

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
file = "names.tsv"

[docs.names.sets]
kind = []
```

Writes `names.tsv`:

```
kind	modal	A box over a screen.
kind	sheet
```

The gate fails and its log prints `names: kind `sheet` has no description`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## names-bad-line

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nspawn\n'
```

The gate fails and its log prints `docs: 3 files, 1 findings
names: line 2 is not <set> TAB <name> [TAB <description>]`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## names-command-fails

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn`.
```

Writes `names.sh`:

```
echo "no such registry" >&2
exit 1
```

The gate fails and its log prints `docs: 3 files, 1 findings
names: `sh names.sh` exited 1: no such registry`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## names-file-not-generated

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
file = "names.tsv"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn`.
```

Writes `names.tsv`:

```
verb	spawn
```

The gate fails and its log prints `docs: 3 files, 1 findings
names: `names.tsv` is not listed as generated`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## names-from-a-file

Writes `.drover/arch.toml`:

```
generated = ["names.tsv"]
generate = "printf 'verb\tspawn\n' > names.tsv"
```

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
file = "names.tsv"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn`.
```

Writes `names.tsv`:

```
verb	spawn
```

The gate passes.

## no-claim

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool <verb>`, `tool --dry-run` or `cargo build --locked`.

```
`tool spwan`
```
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate passes.

## real-paths

Writes `README.md`:

```
See `src/lib.rs:2`, `src/lib.rs`, [doc](docs/a.md) and `drover spawn --dry-run <f>`.
```

Writes `docs/a.md`:

```
`../src/lib.rs` and `drover queue <fleet> add --top 3`
```
`src/gone.rs`
```
```

Writes `src/lib.rs`:

```
fn a() {}
fn b() {}
```

The gate passes.

## record-broken-link

Writes `docs/architecture/x/adr.md`:

```
This slice adds `lib/planned.rs`; see [shapes](shapes.md).
```

The gate fails and its log prints `docs/architecture/x/adr.md: link `shapes.md` does not exist`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## record-planned-path

Writes `docs/architecture/x/adr.md`:

```
This slice adds `lib/planned.rs`; see [shapes](shapes.md).
```

Writes `docs/architecture/x/shapes.md`:

```
# shapes
```

The gate passes.

## records-added

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.records]
"docs/decisions/**" = "dated decisions: they name files as they were"
```

Writes `docs/decisions/one.md`:

```
We deleted `lib/gone.rs` on this date.
```

The gate passes.

## set-not-described

Writes `.drover/arch.toml`:

```
generated = ["names.tsv"]
generate = "true"
```

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
file = "names.tsv"

[docs.names.sets]
kind = []
```

Writes `names.tsv`:

```
kind	modal
kind	sheet
```

The gate passes.

## set-prints-nothing

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
flag = ["--{name}"]
```

Writes `docs/b.md`:

```
Run `--dry-run`.
```

Writes `names.sh`:

```
printf 'flag\told-flag\nflag\tdry-run\n'
```

The gate fails and its log prints `docs: 3 files, 1 findings
names: set `verb` has no name`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## skill-page-listed

Writes `.drover/arch.toml`:

```
generated = ["skill/**"]
generate = "true"
```

Writes `skill/tool/SKILL.md`:

```
# tool

Read `lib/cli.rs` first.
```

The gate passes.

## skill-page-no-arch-file

Removes `.drover/arch.toml`.

Writes `skill/tool/SKILL.md`:

```
# tool

Read `lib/cli.rs` first.
```

The gate fails and its log prints `skill/tool/SKILL.md: agent text is not listed as generated`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## skill-page-unlisted

Writes `skill/tool/SKILL.md`:

```
# tool

Read `lib/cli.rs` first.
```

The gate fails and its log prints `skill/tool/SKILL.md: agent text is not listed as generated`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## skill-page-written

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.written]
"skill/tool/SKILL.md" = "the front page is prose; the pages under it are generated"
```

Writes `skill/tool/SKILL.md`:

```
# tool

Read `lib/cli.rs` first.
```

The gate passes.

## skip-a-claim

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
skip = ["tool spwan"]

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}"]
```

Writes `docs/b.md`:

```
Old docs say `tool spwan`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate passes.

## skip-a-span

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
skip = ["lib/gone.rs"]
```

Writes `docs/b.md`:

```
See `lib/gone.rs`.
```

The gate passes.

## stale-path-line-and-link

Writes `README.md`:

```
See `src/lib.rs:2`, `src/lib.rs`, [doc](docs/a.md) and `drover spawn --dry-run <f>`.
```

Writes `docs/a.md`:

```
`../src/lib.rs` and `drover queue <fleet> add --top 3`
```
`src/gone.rs`
```
```

Writes `docs/b.md`:

```
`src/gone.rs`, `src/lib.rs:3`, [x](docs/none.md), `drover spawn --gone`, `drover nope`
```

Writes `src/lib.rs`:

```
fn a() {}
fn b() {}
```

The gate fails and its log prints `docs/b.md: `src/gone.rs`: no such path
docs/b.md: `src/lib.rs:3`: src/lib.rs has fewer than 3 lines
docs/b.md: link `docs/none.md` does not exist`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## two-names-one-description

Writes `.drover/arch.toml`:

```
generated = ["names.tsv"]
generate = "true"
```

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
file = "names.tsv"

[docs.names.sets]
kind = []
```

Writes `names.tsv`:

```
kind	modal	A box over a screen: a dialog, a fixed confirm or alert, or a tool window.
kind	sheet	A box over a screen: a dialog, a fixed confirm or alert, or a tool window.
```

The gate fails and its log prints `names: kind `sheet` has the description of `modal``.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## two-prefixes

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs.names]
run = "sh names.sh"

[docs.names.sets]
verb = ["tool {name}", "./tool {name}"]
```

Writes `docs/b.md`:

```
Run `tool spawn`, then `./tool spwan`.
```

Writes `names.sh`:

```
printf 'verb\tspawn\nverb\tqueue\n'
```

The gate fails and its log prints `docs/b.md: `./tool spwan`: no verb `spwan``.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## watched-key-documented

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1
```

Writes `docs/a.md`:

```
`--new-flag`
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate passes.

## watched-key-in-a-record

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1
```

Writes `docs/architecture/x.md`:

```
The plan adds `--new-flag`.
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate fails and its log prints `lib/cli.rs: `new_flag` added, but no changed doc line names it (or `no doc: <why>` in the PR body)`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`

## watched-key-no-doc

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
no doc: internal only
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate passes.

## watched-key-undocumented

Writes `.gatekeeper/config.toml`:

```
packs = ["core"]
integration = "main"
pin = "checkout"

[docs]
watch = ["lib/cli.rs"]
```

With the PR body (`{head}` is the case's commit, `{base}` its parent):

```
Closes #1
```

Writes `lib/cli.rs`:

```
struct Args {
    new_flag: bool,
}
```

The gate fails and its log prints `lib/cli.rs: `old_flag` removed, but no changed doc line names it (or `no doc: <why>` in the PR body)
lib/cli.rs: `new_flag` added, but no changed doc line names it (or `no doc: <why>` in the PR body)`.

Fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs. Rerun: `./.gatekeeper/run check --gate core/docs`
