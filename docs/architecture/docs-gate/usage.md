# The docs gate: how the callers read (#83)

Sketches of the commands, the config and the code after the last slice. Names of functions inside a module are
the impl lane's; the calls across modules are the design.

## A repo that writes no config

The bump PR is two commands and two changed files:

```
$ ./.gatekeeper/run pin 1.5.0
$ ./.gatekeeper/run baseline core/docs
.gatekeeper/baseline/core-docs.txt: 14 findings
$ git add .gatekeeper && git commit -m "gatekeeper 1.5.0: the docs gate, 14 old hits listed"
```

From then on the readme, `docs/`, `skill/`, `CLAUDE.md`, `AGENTS.md` and `.drover/*.md` are checked on every run.

## A lane that breaks a doc

```
$ ./.gatekeeper/run check
ok    core/arch             7.9s /   45s
FAIL  core/docs             0.3s /   10s
      docs: 212 files, 2 findings
      README.md: `crates/core/src/gate.rs`: no such path
      skill/gatekeeper/packs/core.md: `core/hygeine`: no core gate `hygeine`
      fix: make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed
           leaves with ./.gatekeeper/run baseline core/docs
      log: target/gatekeeper/core-docs.log   rerun: ./.gatekeeper/run check --gate core/docs
```

A lane that fixes a listed hit is told to shrink the list, by the code every gate shares:

```
FAIL  core/docs             0.3s /   10s
      stale: shrink the baseline (./.gatekeeper/run baseline core/docs): README.md: `docs/old.md`: no such path
```

## The config, every key

All of it is optional. This is a repo that uses each key once:

```toml
# .gatekeeper/config.toml
[docs]
also = ["roles/**/*.md"]                      # more docs, beside the defaults
skip = [".drover/**", "scripts/shards.txt"]   # spans the gate does not read (a path only a lane's host has)
watch = ["crates/core/src/config.rs"]         # a key added or removed here needs a changed doc line
generated = ["docs/commands/**/*.md"]         # more agent text that must be generated, beside skill/**/*.md

[docs.records]                                # dated docs, links only, each with its reason
"docs/decisions/**" = "dated decisions: they name files as they were"

[docs.written]                                # agent text written by hand, each with its reason
"skill/tool/SKILL.md" = "the front page is prose; the command pages under it are generated"

[docs.names]
run = "sh scripts/names.sh"                   # or: file = "docs/names.tsv", a generated file

[docs.names.sets]                             # the prefixes that mark a span as a name of the set
verb = ["tool {name}"]
flag = ["--{name}"]
key = ["[limits] {name}"]
```

## Name sets

The command, or the file, prints one name per line: the set, a tab, the name, and for a set that has them a tab
and the description.

```
$ sh scripts/names.sh
verb	spawn
verb	queue
flag	dry-run
key	stall_minutes
```

A backticked span that starts with a set's prefix claims the word after it:

| Span in a doc | Prefix | Word | Result |
|---|---|---|---|
| `` `tool spawn --dry-run <f>` `` | `tool ` | `spawn` | in `verb`: passes |
| `` `tool spwan` `` | `tool ` | `spwan` | ``README.md: `tool spwan`: no verb `spwan` `` |
| `` `--dry-run` `` | `--` | `dry-run` | in `flag`: passes |
| `` `tool <verb>` `` | `tool ` | none: `<` starts no word | not a claim |
| `` `cargo build --locked` `` | none matches at the start | | not a claim |

gatekeeper's own table (S4) needs no build: the names are file names.

```toml
[docs.names]
run = "sh scripts/names.sh"

[docs.names.sets]
verb = ["./.gatekeeper/run {name}", "gatekeeper {name}"]
core = ["core/{name}"]
rust = ["rust/{name}"]
swift = ["swift/{name}"]
ts = ["ts/{name}"]
supabase = ["supabase/{name}"]
```

```sh
# scripts/names.sh
for f in crates/gatekeeper/src/cmd/*.rs; do n=$(basename "$f" .rs); [ "$n" = mod ] || printf 'verb\t%s\n' "$n"; done
for f in packs/*/gates/*.toml; do p=${f#packs/}; printf '%s\t%s\n' "${p%%/*}" "$(basename "$f" .toml)"; done
```

A repo whose names need its own binary (blocks' kinds) lets its generator write the list, and names the file:

```toml
[docs.names]
file = "skill/blocks/names.tsv"      # under `skill/blocks/**`, which .drover/arch.toml lists as generated

[docs.names.sets]
kind = []                            # no prefix: only its descriptions are checked
```

## Generated agent text, and blocks#393

A skill page that is not listed as generated fails until the repo lists it or says why it is written by hand:

```
skill/tool/pages/ask.md: agent text is not listed as generated
```

The fault of blocks#393, as the gate reports it from the list blocks' generator writes:

```
kind	modal	A box over a screen: a dialog, a fixed confirm or alert, or a tool window.
kind	sheet	A box over a screen: a dialog, a fixed confirm or alert, or a tool window.
```

```
FAIL  core/docs
      names: kind `sheet` has the description of `modal`
```

## Opting out

```toml
[off]
"core/docs" = "docs are being rewritten in #412; back on when it merges"
```

`check`, `list` and the `gates / done` summary print `off: <reason>`, as for any gate.

## Moving an old table (blocks)

```toml
# .drover/arch.toml, before: deleted in the bump PR
[docs]
paths = ["docs/*.md", "docs/schema/*.md", "README.md"]
skip = [".drover/**"]
```

```toml
# .gatekeeper/config.toml, after
[docs]
skip = [".drover/**"]
```

A bump that forgets the move fails `core/arch` with one line:

```
.drover/arch.toml: [docs] moved to .gatekeeper/config.toml: move `watch` and `skip`, drop `paths`
```

## The gate file

```toml
# packs/core/gates/docs.toml
about = "docs and agent text name real paths, links and names; agent text is generated"
fix = "make the doc true: correct the path, link or name, or rerun the generator; a listed hit you fixed leaves with ./.gatekeeper/run baseline core/docs"
budget_secs = 10
cache = false

[[step]]
check = "docs"
```

## The check's entry (`check/docs.rs`)

It reads the typed table and the diff, asks each part for its findings and joins them. No part reads the config
file, the environment or another part.

```rust
/// Docs and agent text name real paths, links and names; agent text is listed as generated.
pub fn run(cx: &Cx) -> Result<Vec<String>> {
    let listed = arch::config::generated(cx.root)?;
    let docs = scope::docs(cx.root, cx.docs);
    let (sets, mut bad) = names::read(cx.root, cx.docs.names.as_ref(), &listed)?;
    bad.extend(scope::unmatched(cx.docs, &docs));
    bad.extend(generated::unlisted(&docs, &listed));
    for doc in &docs {
        let text = text::read(cx.root, doc)?;
        bad.extend(paths::broken_links(cx.root, doc, &text));
        if doc.class != Class::Record {
            bad.extend(paths::stale(cx.root, &cx.docs.skip, doc, &text));
            bad.extend(names::unknown(&sets, &cx.docs.skip, doc, &text));
        }
    }
    if let (Some(base), true) = (cx.base, wants_doc_lines(cx.pr_body)?) {
        bad.extend(watch::undocumented(cx.docs, &docs, &arch::frozen::diff(cx.root, base)?));
    }
    cx.note.set(format!("docs: {} files, {} findings", docs.len(), bad.len())).ok();
    Ok(bad)
}
```

`check/arch.rs` loses its docs lines and its read of the PR body; nothing else in it changes.
