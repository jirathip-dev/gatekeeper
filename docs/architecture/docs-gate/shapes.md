# The docs gate: data shapes (#83)

## The config: `[docs]` in `.gatekeeper/config.toml`

Parsed once with the rest of the config, an unknown key refused. Every key is optional and the table itself is
optional: the gate runs on the defaults.

| Key | Type | Default | Meaning |
|---|---|---|---|
| `also` | globs | none | more docs, beside the default paths |
| `skip` | globs | none | a backticked span that matches is not read (path and name checks) |
| `watch` | globs | none | source files whose added or removed keys need a changed doc line |
| `generated` | globs | none | more agent text that must be generated, beside `skill/**/*.md`; also docs paths |
| `[docs.records]` | glob = reason | none | more records, beside `docs/architecture/**` and `docs/design/**` |
| `[docs.written]` | glob = reason | none | agent text written by hand |
| `[docs.names]` | table | none | the name sets: `run` or `file`, and `[docs.names.sets]` |

The fixed lists, in the model and not in any repo's file:

```rust
const DOCS: [&str; 6] = ["README.md", "docs/**/*.md", "skill/**/*.md", "CLAUDE.md", "AGENTS.md", ".drover/*.md"];
const RECORDS: [&str; 2] = ["docs/architecture/**", "docs/design/**"];
const AGENT_TEXT: [&str; 1] = ["skill/**/*.md"];
```

The types. Each value that can be wrong is refused when the config loads, so the check holds no invalid one.

```rust
// crates/gatekeeper-model/src/docs.rs

/// `[docs]` in `.gatekeeper/config.toml`: every key optional, the gate on without the table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocsConfig {
    pub also: Vec<Glob>,
    pub skip: Vec<Glob>,
    pub watch: Vec<Glob>,
    pub generated: Vec<Glob>,
    pub records: Excuses,
    pub written: Excuses,
    pub names: Option<Names>,
}

/// `glob = "reason"` lines: what is excused and why.
pub struct Excuses(Vec<(Glob, Reason)>);

/// Text that is not empty once trimmed.
pub struct Reason(String);

/// `[docs.names]`: where the real names come from and how each set's names look in a doc.
pub struct Names {
    pub source: Source,
    pub sets: BTreeMap<SetName, Vec<Prefix>>,
}

/// `run = "…"` or `file = "…"`: exactly one.
pub enum Source {
    Run(String),
    File(String),
}

/// A set's name: lowercase letters, digits and dashes, as a gate's.
pub struct SetName(String);

/// `<text>{name}`: the text is not empty and `{name}` ends it.
pub struct Prefix(String);

impl Prefix {
    /// The word a span claims: `Some` when the span starts with the prefix and a word follows it.
    pub fn claim<'a>(&self, span: &'a str) -> Option<&'a str>;
}
```

A word starts with a letter or a digit and runs over letters, digits, `_` and `-`. So `<verb>`, `…`, `*` and
`--flag` after a prefix claim nothing.

`Config` gains `pub docs: DocsConfig`. `RunCtx` and `CheckCx` gain the same value, as they hold `rust`.

## A doc's class

| Class | Which files | Paths | Links | Names | Must be listed as generated |
|---|---|---|---|---|---|
| record | `RECORDS`, `[docs.records]` | no | yes | no | no |
| agent text | `AGENT_TEXT`, `generated`, less `[docs.written]` | yes | yes | yes | yes |
| current | every other doc: `DOCS`, `also`, and a page in `[docs.written]` | yes | yes | yes | no |

A file that matches two lists takes the first row that fits.

```rust
// crates/gatekeeper-engine/src/docs/scope.rs
pub(crate) enum Class { Record, AgentText, Current }
pub(crate) struct Doc { pub path: String, pub class: Class }
```

## The names list

What `run` prints on stdout, or what `file` holds. One name per line, fields split by one tab:

```
<set> TAB <name> [TAB <description>]
```

- `set`: a `SetName`. A set the config does not name is read for its descriptions only.
- `name`: not empty, no tab, no backtick.
- `description`: the rest of the line. A set is "described" when any of its lines has one.
- An empty line is skipped. Any other line that does not fit is a finding.

```rust
// crates/gatekeeper-engine/src/docs/names.rs
/// Each set's names, each with its description when the list gives one.
pub(crate) struct Sets(BTreeMap<SetName, BTreeMap<String, Option<String>>>);
```

## What the check prints

One line per finding, no line number, at most once per file and span. These lines are the baseline's lines.

| Part | A finding |
|---|---|
| paths | ``docs/b.md: `src/gone.rs`: no such path`` |
| paths | ``docs/b.md: `src/lib.rs:3`: src/lib.rs has fewer than 3 lines`` |
| links | ``docs/b.md: link `docs/none.md` does not exist`` |
| watch | ``lib/cli.rs: `new_flag` added, but no changed doc line names it (or `no doc: <why>` in the PR body)`` |
| names | ``README.md: `tool spwan`: no verb `spwan` `` |
| the names list | ``names: `sh scripts/names.sh` exited 1: <the end of its stderr>`` |
| the names list | ``names: line 4 is not <set> TAB <name> [TAB <description>]`` |
| the names list | ``names: set `verb` has no name`` |
| the names list | ``names: `docs/names.tsv` is not listed as generated`` |
| descriptions | ``names: kind `sheet` has the description of `modal` `` |
| descriptions | ``names: kind `sheet` has no description`` |
| agent text | ``skill/tool/pages/ask.md: agent text is not listed as generated`` |
| excuses | ``[docs.written] `skill/old/**` matches no file`` |

A path finding follows today's rules for what a span must look like to be read as a path, with one more: a
missing path that git ignores (`git check-ignore`) is no finding.

## The baseline: `.gatekeeper/baseline/core-docs.txt`

The generic file of any gate: the finding lines above, sorted, one per line. `baseline core/docs` writes it once
and after that only drops lines. No new code and no new format.

```
README.md: `docs/old.md`: no such path
docs/setup.md: link `img/flow.png` does not exist
skill/tool/SKILL.md: agent text is not listed as generated
```

`watch` findings exist only against a base, so they are never in a list: `baseline` runs without one.

## What `core/docs` reads of `.drover/arch.toml`

One key, `generated`, through a one-key read in `arch/config.rs`, as `Frozen` reads `frozen` at the base. A file
that does not parse is `core/arch`'s finding, not this gate's.

```rust
// crates/gatekeeper-engine/src/arch/config.rs
/// The `generated` globs alone: what the docs gate holds agent text and a names file to.
pub(crate) fn generated(root: &Path) -> Result<Vec<Glob>>;
```

`Arch` keeps a `docs` key only to refuse it: a table there is a parse error that names `.gatekeeper/config.toml`.
